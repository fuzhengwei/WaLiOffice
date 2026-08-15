use axum::extract::Query;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use futures::stream::Stream;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;

use crate::agent::{run_agent_loop, AgentConfig, AgentEvent};
use crate::error::AppError;
use crate::models::{ChatMessage, ChatRequest};
use crate::auth::middleware::AuthUser;
use crate::state;
use crate::db::session_repo;

pub fn router() -> Router {
    Router::new().route("/api/chat/stream", post(chat_stream))
}

async fn chat_stream(
    user: AuthUser,
    Json(req): Json<ChatRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let pool = state::db_pool();
    let client = std::sync::Arc::new(crate::llm::LlmClient::for_user(&user.0.id, req.model.as_deref()));

    // 创建或获取会话
    let session = if let Some(ref sid) = req.session_id {
        session_repo::find_by_id(&pool, sid)?
            .ok_or(AppError::NotFound("会话不存在".into()))?
    } else {
        let title: String = req.message.chars().take(30).collect();
        session_repo::create(&pool, &user.0.id, req.project_id.as_deref(), req.tool_kind.as_deref(), &title)?
    };

    let session_id = session.id.clone();

    // 加载历史消息
    let history = session_repo::get_messages(&pool, &session_id, 50).unwrap_or_default();

    // 保存用户消息
    let user_msg = ChatMessage {
        role: "user".into(),
        content: req.message.clone(),
        tool_calls: None,
        tool_call_id: None,
    };
    let _ = session_repo::add_message(&pool, &session_id, &user_msg);

    // 创建 SSE channel
    let (sse_tx, sse_rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(256);

    // clone 必要数据给 agent 任务
    let user_id = user.0.id.clone();
    let project_id = req.project_id.clone();
    let user_message = req.message.clone();
    let agent_config = AgentConfig {
        max_turns: 6,
        system_prompt: String::new(), // 使用默认 OFFICE_AGENT_PROMPT
        allowed_tools: None,
    };

    // 创建 tool context 的 emit 回调
    let sse_tx_clone = sse_tx.clone();
    let emit = move |event: &str, data: serde_json::Value| {
        let _ = sse_tx_clone.try_send(Ok(Event::default().event(event).data(data.to_string())));
    };

    let ctx = crate::agent::tool::ToolContext::new(
        session_id.clone(),
        user_id.clone(),
        project_id.clone(),
        req.model.clone(),
        emit,
    );

    let session_id_for_save = session_id.clone();
    let pool_for_save = pool.clone();

    // 启动 agent 循环
    let mut event_rx = run_agent_loop(history, user_message, ctx, agent_config, client.clone()).await;

    tokio::spawn(async move {
        let mut final_summary = String::new();
        let mut collected_artifacts: Vec<crate::models::Artifact> = Vec::new();

        while let Some(event) = event_rx.recv().await {
            let sse_event = match &event {
                AgentEvent::Thinking { content } => {
                    Event::default().event("state_update").data(serde_json::json!({
                        "phase": "running",
                        "step": "Agent 思考中",
                        "detail": content,
                        "at": chrono::Utc::now().to_rfc3339(),
                    }).to_string())
                }
                AgentEvent::ToolCall { tool, input } => {
                    Event::default().event("state_update").data(serde_json::json!({
                        "phase": "running",
                        "step": format!("调用工具: {tool}"),
                        "detail": serde_json::to_string(input).unwrap_or_default().chars().take(200).collect::<String>(),
                        "at": chrono::Utc::now().to_rfc3339(),
                    }).to_string())
                }
                AgentEvent::ToolResult { tool, success, result, error } => {
                    Event::default().event("tool_result").data(serde_json::json!({
                        "tool": tool,
                        "success": success,
                        "result": result,
                        "error": error,
                    }).to_string())
                }
                AgentEvent::Artifact { artifact } => {
                    collected_artifacts.push(artifact.clone());
                    Event::default().event("artifact_update").data(serde_json::json!({
                        "artifact": artifact,
                        "artifacts": collected_artifacts,
                        "session_id": session_id_for_save,
                        "tool_kind": artifact.tool_kind,
                    }).to_string())
                }
                AgentEvent::Message { content } => {
                    final_summary = content.clone();
                    Event::default().event("message").data(serde_json::json!({
                        "text": content,
                        "session_id": session_id_for_save,
                    }).to_string())
                }
                AgentEvent::TurnEnd { turn } => {
                    Event::default().event("state_update").data(serde_json::json!({
                        "phase": "running",
                        "step": format!("第 {turn} 轮完成"),
                        "detail": format!("已完成 {turn} 轮工具调用"),
                        "at": chrono::Utc::now().to_rfc3339(),
                    }).to_string())
                }
                AgentEvent::Done { summary, artifacts } => {
                    final_summary = summary.clone();
                    if collected_artifacts.is_empty() {
                        collected_artifacts = artifacts.clone();
                    }
                    Event::default().event("done").data(serde_json::json!({
                        "session_id": session_id_for_save,
                        "artifacts": collected_artifacts,
                    }).to_string())
                }
                AgentEvent::Error { message } => {
                    Event::default().event("error").data(serde_json::json!({
                        "message": message,
                    }).to_string())
                }
            };
            let _ = sse_tx.send(Ok(sse_event)).await;

            // 保存 assistant 消息
            if let AgentEvent::Message { content } = &event {
                let assistant_msg = ChatMessage {
                    role: "assistant".into(),
                    content: content.clone(),
                    tool_calls: None,
                    tool_call_id: None,
                };
                let _ = session_repo::add_message(&pool_for_save, &session_id_for_save, &assistant_msg);
            }
        }
    });

    let stream = ReceiverStream::new(sse_rx);
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}
