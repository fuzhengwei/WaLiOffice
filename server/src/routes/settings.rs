use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde_json::json;

use crate::auth::middleware::AuthUser;
use crate::db::settings_repo;
use crate::error::AppError;
use crate::models::{AppSettings, BasicSettings, LlmProfileConfig, McpServerConfig};
use crate::state;

const BUILTIN_MODELS: &[&str] = &["agnes-2.0-flash"];

pub fn router() -> Router {
    Router::new()
        .route("/api/settings", get(get_settings).put(save_settings))
        .route("/api/settings/mcp/test", post(test_mcp_service))
}

fn default_settings() -> AppSettings {
    let cfg = crate::config::config();
    let mut models = vec![cfg.llm_model.clone()];
    for model in BUILTIN_MODELS {
        if !models.iter().any(|item| item == model) {
            models.push((*model).to_string());
        }
    }
    let default_profile = LlmProfileConfig {
        id: "default".into(),
        name: "默认模型服务".into(),
        base_url: cfg.llm_base_url.clone(),
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
            brand_tagline: "直接开始创作，而不是先进入后台".into(),
            default_theme: "default".into(),
        },
        mcp_servers: vec![],
        updated_at: chrono::Utc::now().to_rfc3339(),
    }
}

fn normalize_settings(mut settings: AppSettings) -> Result<AppSettings, AppError> {
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
        profile.models = profile
            .models
            .iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect();
        for model in BUILTIN_MODELS {
            if !profile.models.iter().any(|item| item == model) {
                profile.models.push((*model).to_string());
            }
        }
        if profile.models.is_empty() {
            return Err(AppError::BadRequest(format!("模型服务「{}」至少需要一个模型", profile.name)));
        }
        if profile.default_model.trim().is_empty() || !profile.models.iter().any(|item| item == &profile.default_model) {
            profile.default_model = profile.models[0].clone();
        }
        profile.has_api_key = profile.has_api_key || profile.api_key.as_ref().is_some_and(|value| !value.trim().is_empty());
    }

    if !settings.llm_profiles.iter().any(|profile| profile.id == settings.active_profile_id) {
        settings.active_profile_id = settings.llm_profiles[0].id.clone();
    }

    let active_profile = settings
        .llm_profiles
        .iter()
        .find(|profile| profile.id == settings.active_profile_id)
        .cloned()
        .unwrap_or_else(|| settings.llm_profiles[0].clone());

    if settings.default_model.trim().is_empty() || !active_profile.models.iter().any(|item| item == &settings.default_model) {
        settings.default_model = active_profile.default_model.clone();
    }
    if settings.active_model.trim().is_empty() || !active_profile.models.iter().any(|item| item == &settings.active_model) {
        settings.active_model = settings.default_model.clone();
    }

    if settings.basic.app_name.trim().is_empty() {
        settings.basic.app_name = crate::config::config().app_name.clone();
    }
    if settings.basic.workspace_title.trim().is_empty() {
        settings.basic.workspace_title = "智能办公助手".into();
    }
    if settings.basic.brand_tagline.trim().is_empty() {
        settings.basic.brand_tagline = "直接开始创作，而不是先进入后台".into();
    }
    if settings.basic.default_theme.trim().is_empty() {
        settings.basic.default_theme = "default".into();
    }

    settings.updated_at = chrono::Utc::now().to_rfc3339();
    Ok(settings)
}

async fn get_settings(user: AuthUser) -> Result<Json<AppSettings>, AppError> {
    let pool = state::db_pool();
    let settings = settings_repo::find_by_user(&pool, &user.0.id)?.unwrap_or_else(default_settings);
    Ok(Json(normalize_settings(settings)?))
}

async fn save_settings(user: AuthUser, Json(payload): Json<AppSettings>) -> Result<Json<AppSettings>, AppError> {
    let pool = state::db_pool();
    let normalized = normalize_settings(payload)?;
    let saved = settings_repo::save_for_user(&pool, &user.0.id, &normalized)?;
    Ok(Json(saved))
}

async fn test_mcp_service(_user: AuthUser, Json(payload): Json<McpServerConfig>) -> Result<Json<serde_json::Value>, AppError> {
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
