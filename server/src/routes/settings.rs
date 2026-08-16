use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;

use crate::auth::middleware::AuthUser;
use crate::db::settings_repo;
use crate::error::AppError;
use crate::models::{AppSettings, BasicSettings, LlmProfileConfig, McpServerConfig};
use crate::state;
use eventsource_stream::Eventsource;
use futures::StreamExt;

pub fn router() -> Router {
    Router::new()
        .route("/api/settings", get(get_settings).put(save_settings))
        .route("/api/settings/mcp/test", post(test_mcp_service))
}

pub fn default_settings() -> AppSettings {
    let cfg = crate::config::config();
    let models = configured_models(cfg);
    let default_profile = LlmProfileConfig {
        id: "default".into(),
        name: "默认模型服务".into(),
        base_url: cfg.llm_base_url.clone(),
        api_keys: if cfg.llm_api_key.trim().is_empty() {
            vec![]
        } else {
            vec![cfg.llm_api_key.clone()]
        },
        models,
        default_model: cfg.llm_model.clone(),
        api_key: None,
        has_api_key: !cfg.llm_api_key.is_empty(),
    };

    AppSettings {
        llm_profiles: vec![default_profile.clone()],
        active_profile_id: default_profile.id.clone(),
        default_model: default_profile.default_model.clone(),
        active_model: default_profile.default_model.clone(),
        basic: BasicSettings {
            app_name: cfg.app_name.clone(),
            workspace_title: "智能办公助手".into(),
            brand_tagline: "打开即用，专注办公创作".into(),
            default_theme: "default".into(),
        },
        mcp_servers: builtin_mcp_servers(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    }
}

fn configured_models(cfg: &crate::config::Config) -> Vec<String> {
    let mut models = Vec::new();
    for model in cfg
        .llm_text_models
        .iter()
        .chain(cfg.llm_image_models.iter())
        .chain(cfg.llm_video_models.iter())
    {
        let model = model.trim();
        if !model.is_empty() && !models.iter().any(|item| item == model) {
            models.push(model.to_string());
        }
    }
    models
}

fn builtin_mcp_servers() -> Vec<McpServerConfig> {
    let cfg = crate::config::config();
    if cfg.baidu_mcp_api_key.trim().is_empty() {
        return vec![];
    }

    let base = cfg.baidu_mcp_sse_endpoint.trim();
    let endpoint = if base.contains("api_key=") {
        base.to_string()
    } else {
        format!(
            "{}?api_key={}",
            base.trim_end_matches('/'),
            urlencoding::encode(cfg.baidu_mcp_api_key.trim()),
        )
    };

    vec![McpServerConfig {
        id: "builtin-baidu-ai-search".into(),
        name: "百度 AI Search MCP".into(),
        transport: "sse".into(),
        endpoint,
        enabled: true,
        description: Some("内置中文搜索增强服务，适合人名、项目名、技术资料和公开网页检索。部署到你自己的环境时，需要在 .env 中配置百度 MCP Key 才会启用。".into()),
    }]
}

pub fn normalize_settings(mut settings: AppSettings) -> Result<AppSettings, AppError> {
    if settings.llm_profiles.is_empty() {
        return Err(AppError::BadRequest("至少保留一个模型服务配置".into()));
    }

    for profile in &mut settings.llm_profiles {
        if profile.id.trim().is_empty() {
            profile.id = uuid::Uuid::new_v4().to_string();
        }
        if profile.name.trim().is_empty() {
            profile.name = "未命名模型服务".into();
        }
        let mut api_keys: Vec<String> = Vec::new();
        for api_key in &profile.api_keys {
            let api_key = api_key.trim().to_string();
            if !api_key.is_empty() && !api_keys.iter().any(|item| item == &api_key) {
                api_keys.push(api_key);
            }
        }
        if let Some(api_key) = profile.api_key.take() {
            let api_key = api_key.trim().to_string();
            if !api_key.is_empty() && !api_keys.iter().any(|item| item == &api_key) {
                api_keys.push(api_key);
            }
        }
        profile.api_keys = api_keys;
        profile.models = profile
            .models
            .iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect();
        if profile.models.is_empty() {
            return Err(AppError::BadRequest(format!(
                "模型服务「{}」至少需要一个模型",
                profile.name
            )));
        }
        if profile.default_model.trim().is_empty()
            || !profile
                .models
                .iter()
                .any(|item| item == &profile.default_model)
        {
            profile.default_model = profile.models[0].clone();
        }
        profile.has_api_key = !profile.api_keys.is_empty();
    }

    if !settings
        .llm_profiles
        .iter()
        .any(|profile| profile.id == settings.active_profile_id)
    {
        settings.active_profile_id = settings.llm_profiles[0].id.clone();
    }

    let active_profile = settings
        .llm_profiles
        .iter()
        .find(|profile| profile.id == settings.active_profile_id)
        .cloned()
        .unwrap_or_else(|| settings.llm_profiles[0].clone());

    if settings.default_model.trim().is_empty()
        || !active_profile
            .models
            .iter()
            .any(|item| item == &settings.default_model)
    {
        settings.default_model = active_profile.default_model.clone();
    }
    if settings.active_model.trim().is_empty()
        || !active_profile
            .models
            .iter()
            .any(|item| item == &settings.active_model)
    {
        settings.active_model = settings.default_model.clone();
    }

    if settings.basic.app_name.trim().is_empty() {
        settings.basic.app_name = crate::config::config().app_name.clone();
    }
    if settings.basic.workspace_title.trim().is_empty() {
        settings.basic.workspace_title = "智能办公助手".into();
    }
    if settings.basic.brand_tagline.trim().is_empty() {
        settings.basic.brand_tagline = "打开即用，专注办公创作".into();
    }
    if settings.basic.default_theme.trim().is_empty() {
        settings.basic.default_theme = "default".into();
    }

    for builtin in builtin_mcp_servers() {
        if !settings
            .mcp_servers
            .iter()
            .any(|server| server.id == builtin.id || server.endpoint == builtin.endpoint)
        {
            settings.mcp_servers.push(builtin);
        }
    }

    settings.updated_at = chrono::Utc::now().to_rfc3339();
    Ok(settings)
}

pub fn import_startup_llm_profile(settings: &mut AppSettings) {
    let cfg = crate::config::config();
    let base_url = cfg.llm_base_url.trim();
    let model = cfg.llm_model.trim();
    if base_url.is_empty() || model.is_empty() {
        return;
    }

    let startup_keys = if cfg.llm_api_key.trim().is_empty() {
        vec![]
    } else {
        vec![cfg.llm_api_key.trim().to_string()]
    };

    if let Some(profile) = settings.llm_profiles.iter_mut().find(|profile| {
        profile.id == "startup-env"
            || profile.base_url.trim_end_matches('/') == base_url.trim_end_matches('/')
    }) {
        if profile.id == "startup-env" {
            profile.name = "启动配置模型服务".into();
            profile.base_url = base_url.to_string();
        }
        if !profile.models.iter().any(|item| item == model) {
            profile.models.insert(0, model.to_string());
        }
        for configured in configured_models(cfg) {
            if !profile.models.iter().any(|item| item == &configured) {
                profile.models.push(configured);
            }
        }
        for api_key in startup_keys {
            if !profile.api_keys.iter().any(|item| item == &api_key) {
                profile.api_keys.push(api_key);
            }
        }
        if profile.default_model.trim().is_empty() {
            profile.default_model = model.to_string();
        }
        return;
    }

    let models = configured_models(cfg);

    settings.llm_profiles.push(LlmProfileConfig {
        id: "startup-env".into(),
        name: "启动配置模型服务".into(),
        base_url: base_url.to_string(),
        api_keys: startup_keys,
        models,
        default_model: model.to_string(),
        api_key: None,
        has_api_key: !cfg.llm_api_key.trim().is_empty(),
    });
}

async fn get_settings(user: AuthUser) -> Result<Json<AppSettings>, AppError> {
    let pool = state::db_pool();
    let settings = settings_repo::find_by_user(&pool, &user.0.id).await?.unwrap_or_else(default_settings);
    Ok(Json(normalize_settings(settings)?))
}

async fn save_settings(
    user: AuthUser,
    Json(payload): Json<AppSettings>,
) -> Result<Json<AppSettings>, AppError> {
    let pool = state::db_pool();
    let normalized = normalize_settings(payload)?;
    let saved = settings_repo::save_for_user(&pool, &user.0.id, &normalized).await?;
    Ok(Json(saved))
}

async fn test_mcp_service(
    _user: AuthUser,
    Json(payload): Json<McpServerConfig>,
) -> Result<Json<serde_json::Value>, AppError> {
    if payload.transport == "sse" {
        return test_sse_mcp_service(&payload).await;
    }

    if payload.transport != "http" {
        return Ok(Json(json!({
            "ok": false,
            "message": format!("当前仅支持测试 HTTP 类型 MCP 服务，暂不支持 {}", payload.transport),
            "tools": []
        })));
    }

    let endpoint = payload.endpoint.trim().trim_end_matches('/').to_string();
    if endpoint.is_empty() {
        return Err(AppError::BadRequest("MCP 服务地址不能为空".into()));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("构建 MCP 测试客户端失败: {e}")))?;

    let initialize_resp = client
        .post(&endpoint)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "WaLiOffice",
                    "version": "0.2.0"
                }
            }
        }))
        .send()
        .await
        .map_err(|e| AppError::BadRequest(format!("连接 MCP 服务失败: {e}")))?;

    let status = initialize_resp.status();
    let init_text = initialize_resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Ok(Json(json!({
            "ok": false,
            "message": format!("初始化失败: HTTP {} {}", status.as_u16(), init_text),
            "tools": []
        })));
    }

    let _ = client
        .post(&endpoint)
        .header("Content-Type", "application/json")
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }))
        .send()
        .await;

    let tools_resp = client
        .post(&endpoint)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }))
        .send()
        .await
        .map_err(|e| AppError::BadRequest(format!("读取 MCP 工具列表失败: {e}")))?;

    let tools_status = tools_resp.status();
    let tools_text = tools_resp.text().await.unwrap_or_default();
    if !tools_status.is_success() {
        return Ok(Json(json!({
            "ok": false,
            "message": format!("获取工具列表失败: HTTP {} {}", tools_status.as_u16(), tools_text),
            "tools": []
        })));
    }

    let payload_value: serde_json::Value = serde_json::from_str(&tools_text)
        .map_err(|e| AppError::BadRequest(format!("解析 MCP 工具列表失败: {e}")))?;
    let tools = payload_value
        .get("result")
        .and_then(|result| result.get("tools"))
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();

    Ok(Json(json!({
        "ok": true,
        "message": format!("MCP 服务连接成功，共发现 {} 个工具", tools.len()),
        "tools": tools
    })))
}

async fn test_sse_mcp_service(
    payload: &McpServerConfig,
) -> Result<Json<serde_json::Value>, AppError> {
    let endpoint = payload.endpoint.trim().to_string();
    if endpoint.is_empty() {
        return Err(AppError::BadRequest("MCP SSE 地址不能为空".into()));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("构建 MCP SSE 客户端失败: {e}")))?;

    let sse_resp = client
        .get(&endpoint)
        .header("Accept", "text/event-stream")
        .send()
        .await
        .map_err(|e| AppError::BadRequest(format!("连接 MCP SSE 服务失败: {e}")))?;

    if !sse_resp.status().is_success() {
        let status = sse_resp.status();
        let body = sse_resp.text().await.unwrap_or_default();
        return Ok(Json(json!({
            "ok": false,
            "message": format!("连接 MCP SSE 服务失败: HTTP {} {}", status.as_u16(), body),
            "tools": []
        })));
    }

    let mut stream = sse_resp.bytes_stream().eventsource();
    let message_endpoint = loop {
        let Some(event) = stream.next().await else {
            return Ok(Json(json!({
                "ok": false,
                "message": "MCP SSE 服务未返回 endpoint 事件",
                "tools": []
            })));
        };

        match event {
            Ok(ev) if ev.event == "endpoint" && !ev.data.trim().is_empty() => {
                break if ev.data.starts_with("http://") || ev.data.starts_with("https://") {
                    ev.data
                } else {
                    format!("http://appbuilder.baidu.com{}", ev.data)
                };
            }
            Ok(_) => {}
            Err(err) => {
                return Ok(Json(json!({
                    "ok": false,
                    "message": format!("解析 MCP SSE 事件失败: {err}"),
                    "tools": []
                })));
            }
        }
    };

    let request_headers = [
        ("Content-Type", "application/json"),
        ("Accept", "application/json, text/event-stream"),
    ];

    let initialize_resp = client
        .post(&message_endpoint)
        .headers(request_headers.iter().fold(
            reqwest::header::HeaderMap::new(),
            |mut headers, (key, value)| {
                headers.insert(*key, reqwest::header::HeaderValue::from_static(value));
                headers
            },
        ))
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "WaLiOffice",
                    "version": "0.2.0"
                }
            }
        }))
        .send()
        .await
        .map_err(|e| AppError::BadRequest(format!("发送 MCP initialize 失败: {e}")))?;

    if !initialize_resp.status().is_success() {
        return Ok(Json(json!({
            "ok": false,
            "message": format!("MCP initialize 失败: HTTP {}", initialize_resp.status().as_u16()),
            "tools": []
        })));
    }

    let _ = client
        .post(&message_endpoint)
        .header("Content-Type", "application/json")
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }))
        .send()
        .await;

    let tools_resp = client
        .post(&message_endpoint)
        .headers(request_headers.iter().fold(
            reqwest::header::HeaderMap::new(),
            |mut headers, (key, value)| {
                headers.insert(*key, reqwest::header::HeaderValue::from_static(value));
                headers
            },
        ))
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }))
        .send()
        .await
        .map_err(|e| AppError::BadRequest(format!("请求 MCP 工具列表失败: {e}")))?;

    if !tools_resp.status().is_success() {
        return Ok(Json(json!({
            "ok": false,
            "message": format!("MCP tools/list 失败: HTTP {}", tools_resp.status().as_u16()),
            "tools": []
        })));
    }

    let tools = loop {
        let Some(event) = stream.next().await else {
            return Ok(Json(json!({
                "ok": false,
                "message": "MCP SSE 未返回工具列表结果",
                "tools": []
            })));
        };

        match event {
            Ok(ev) if ev.event == "message" => {
                let payload_value: serde_json::Value = match serde_json::from_str(&ev.data) {
                    Ok(value) => value,
                    Err(_) => continue,
                };
                if payload_value.get("id").and_then(|v| v.as_i64()) == Some(2) {
                    break payload_value
                        .get("result")
                        .and_then(|result| result.get("tools"))
                        .and_then(|value| value.as_array())
                        .cloned()
                        .unwrap_or_default();
                }
            }
            Ok(_) => {}
            Err(err) => {
                return Ok(Json(json!({
                    "ok": false,
                    "message": format!("读取 MCP SSE 工具列表结果失败: {err}"),
                    "tools": []
                })));
            }
        }
    };

    Ok(Json(json!({
        "ok": true,
        "message": format!("MCP SSE 服务连接成功，共发现 {} 个工具", tools.len()),
        "tools": tools
    })))
}
