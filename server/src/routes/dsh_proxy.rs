use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::stream::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::wrappers::ReceiverStream;

use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use crate::state;

/// DSH Agent Engine 的地址
const DSH_ENGINE_URL: &str = "http://127.0.0.1:3780";

/// DSH Agent Engine 的连接超时
const DSH_CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

/// DSH Agent Engine 的请求超时
const DSH_REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

// ─── 请求/响应类型 ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DshChatRequest {
    pub message: String,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub model: Option<String>,
    pub tool_kind: Option<String>,
    pub attachments: Option<Vec<serde_json::Value>>,
    pub tool_config: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct DshStatusResponse {
    pub dsh_available: bool,
    pub engine: String,
    pub version: String,
    pub rust_backend: String,
}

#[derive(Debug, Serialize)]
pub struct DshProxyError {
    pub error: String,
    pub detail: Option<String>,
}

// ─── 工具回调请求类型 ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ToolCallbackRequest {
    pub tool: String,
    pub input: serde_json::Value,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ToolCallbackResponse {
    pub success: bool,
    pub observation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ─── 共享 HTTP 客户端 ──────────────────────────────────────────────

static DSH_CLIENT: once_cell::sync::Lazy<Client> = once_cell::sync::Lazy::new(|| {
    Client::builder()
        .connect_timeout(DSH_CONNECT_TIMEOUT)
        .timeout(DSH_REQUEST_TIMEOUT)
        .build()
        .expect("Failed to build DSH HTTP client")
});

// ─── 路由 ──────────────────────────────────────────────────────────

pub fn router() -> Router {
    Router::new()
        // DSH 代理路由
        .route("/api/agent/dsh/stream", post(dsh_chat_stream))
        .route("/api/agent/dsh/status", get(dsh_status))
        // DSH 工具回调端点（DSH Agent Engine 回调 Rust 后端）
        .route("/api/agent/tool/ppt_plan", post(tool_callback))
        .route("/api/agent/tool/ppt_generate", post(tool_callback))
        .route("/api/agent/tool/doc_generate", post(tool_callback))
        .route("/api/agent/tool/md_generate", post(tool_callback))
        .route("/api/agent/tool/sheet_generate", post(tool_callback))
        .route("/api/agent/tool/chart_generate", post(tool_callback))
        .route("/api/agent/tool/drawio_generate", post(tool_callback))
        .route("/api/agent/tool/image_prompt", post(tool_callback))
        .route("/api/agent/tool/video_generate", post(tool_callback))
        .route("/api/agent/tool/web_search", post(tool_callback))
}

// ─── DSH 可用性检查 ────────────────────────────────────────────────

/// 检查 DSH Agent Engine 是否可用
pub async fn is_dsh_available() -> bool {
    let client = &*DSH_CLIENT;
    match client
        .get(format!("{}/agent/status", DSH_ENGINE_URL))
        .timeout(DSH_CONNECT_TIMEOUT)
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

// ─── DSH 状态端点 ──────────────────────────────────────────────────

async fn dsh_status() -> Json<DshStatusResponse> {
    let available = is_dsh_available().await;
    Json(DshStatusResponse {
        dsh_available: available,
        engine: "dsh".to_string(),
        version: "0.1.0-rc.6".to_string(),
        rust_backend: "running".to_string(),
    })
}

// ─── DSH SSE 流式代理 ──────────────────────────────────────────────

async fn dsh_chat_stream(
    user: AuthUser,
    Json(req): Json<DshChatRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let _pool = state::db_pool();

    // 检查 DSH 是否可用
    if !is_dsh_available().await {
        return Err(AppError::Internal(anyhow::anyhow!(
            "DSH Agent Engine 不可用，请使用 /api/chat/stream 端点"
        )));
    }

    // 构建发送给 DSH 的请求体
    let dsh_body = serde_json::json!({
        "message": req.message,
        "session_id": req.session_id,
        "user_id": user.0.id,
        "model": req.model,
        "tool_kind": req.tool_kind,
        "attachments": req.attachments,
        "tool_config": req.tool_config,
    });

    // 向 DSH Agent Engine 发送 SSE 请求
    let client = &*DSH_CLIENT;
    let dsh_response = client
        .post(format!("{}/agent/stream", DSH_ENGINE_URL))
        .json(&dsh_body)
        .send()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DSH 请求失败: {}", e)))?;

    if !dsh_response.status().is_success() {
        let status = dsh_response.status();
        let body = dsh_response.text().await.unwrap_or_default();
        return Err(AppError::Internal(anyhow::anyhow!(
            "DSH 返回错误 {}: {}",
            status,
            body
        )));
    }

    // 将 DSH 的 SSE 流转换为 axum SSE 流
    let (sse_tx, sse_rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(256);

    tokio::spawn(async move {
        use futures::StreamExt;

        let mut stream = dsh_response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(bytes) => {
                    buffer.push_str(&String::from_utf8_lossy(&bytes));

                    // 解析 SSE 事件
                    while let Some(event_end) = buffer.find("\n\n") {
                        let event_text = buffer[..event_end].to_string();
                        buffer = buffer[event_end + 2..].to_string();

                        if let Some(sse_event) = parse_dsh_sse_event(&event_text) {
                            let _ = sse_tx.send(Ok(sse_event)).await;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("DSH stream error: {}", e);
                    let error_event = Event::default()
                        .event("error")
                        .data(serde_json::json!({"message": format!("DSH 流错误: {}", e)}).to_string());
                    let _ = sse_tx.send(Ok(error_event)).await;
                    break;
                }
            }
        }

        // 确保流结束
        let _ = sse_tx.send(Ok(Event::default().event("done").data("{}"))).await;
    });

    let stream = ReceiverStream::new(sse_rx);
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}

/// 解析 DSH SSE 事件文本为 axum Event
fn parse_dsh_sse_event(text: &str) -> Option<Event> {
    let mut event_type = "message".to_string();
    let mut data = String::new();

    for line in text.lines() {
        if let Some(value) = line.strip_prefix("event:") {
            event_type = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(value.trim());
        }
    }

    if data.is_empty() {
        return None;
    }

    Some(Event::default().event(&event_type).data(&data))
}

// ─── 工具回调端点 ──────────────────────────────────────────────────

/// DSH Agent Engine 回调 Rust 后端执行工具的统一入口
///
/// DSH 的 Office 工具插件通过 HTTP 调用此端点，
/// Rust 后端根据工具名称分派到对应的工具实现。
async fn tool_callback(
    Json(req): Json<ToolCallbackRequest>,
) -> Result<Json<ToolCallbackResponse>, AppError> {
    let _pool = state::db_pool();
    let registry = &crate::agent::registry::REGISTRY;

    // 查找工具（registry.get 是 async 方法）
    let tool = registry
        .get(&req.tool)
        .await
        .ok_or_else(|| AppError::NotFound(format!("工具 {} 不存在", req.tool)))?;

    // 构建 ToolContext
    let session_id = req.session_id.clone().unwrap_or_default();
    let user_id = req.user_id.clone().unwrap_or_default();

    let (emit_tx, _emit_rx) = tokio::sync::mpsc::channel::<(String, serde_json::Value)>(64);
    let emit = move |event: &str, data: serde_json::Value| {
        let _ = emit_tx.try_send((event.to_string(), data));
    };

    let ctx = crate::agent::tool::ToolContext::new(
        session_id,
        user_id,
        None,  // project_id
        None,  // preferred_model
        vec![], // attachments
        emit,
    );

    // 执行工具
    let tool_name = req.tool.clone();
    let result = tool.call(req.input, &ctx).await;

    // 注意：回调场景下 _emit_rx 中的 SSE 事件被忽略，
    // 实际的 SSE 推送由 DSH Agent Engine 负责
    drop(_emit_rx);

    let response = if result.success {
        ToolCallbackResponse {
            success: true,
            observation: result.observation,
            artifacts: result.artifacts.map(|arts| {
                arts.iter()
                    .map(|a| serde_json::json!({
                        "kind": a.kind,
                        "title": a.title,
                        "content": a.content,
                    }))
                    .collect()
            }),
            error: None,
        }
    } else {
        ToolCallbackResponse {
            success: false,
            observation: result.observation.clone(),
            artifacts: None,
            error: Some(result.observation),
        }
    };

    tracing::info!(
        "[DSH Callback] tool={} success={}",
        tool_name,
        response.success
    );

    Ok(Json(response))
}
