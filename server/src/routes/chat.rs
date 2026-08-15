use axum::extract::Query;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use futures::stream::Stream;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

use crate::agent::{run_agent_loop, AgentConfig, AgentEvent};
use crate::auth::middleware::AuthUser;
use crate::db::session_repo;
use crate::error::AppError;
use crate::models::{Artifact, ChatAttachment, ChatMessage, ChatRequest};
use crate::state;

pub fn router() -> Router {
    Router::new().route("/api/chat/stream", post(chat_stream))
}

fn looks_like_markdown_document(content: &str) -> bool {
    let trimmed = content.trim();
    trimmed.starts_with('#')
        || trimmed.contains("\n## ")
        || trimmed.contains("\n- ")
        || trimmed.contains("\n1. ")
        || trimmed.contains("```")
        || trimmed.contains("\n> ")
        || trimmed.contains("\n|")
}

fn build_summary_markdown_artifact(summary: &str, tool_kind: Option<&str>) -> Artifact {
    let now = chrono::Utc::now().to_rfc3339();
    Artifact {
        id: uuid::Uuid::new_v4().to_string(),
        kind: "markdown".into(),
        tool_kind: tool_kind.unwrap_or("doc").to_string(),
        title: "对话整理结果".into(),
        status: "ready".into(),
        content: serde_json::json!({
            "type": "markdown",
            "markdown": summary,
            "source": "assistant_summary",
        }),
        version: 1,
        created_at: now.clone(),
        updated_at: now,
    }
}

fn format_attachment_context(attachments: &[ChatAttachment]) -> String {
    if attachments.is_empty() {
        return String::new();
    }

    let mut image_attachment_count = 0usize;
    let sections = attachments
        .iter()
        .enumerate()
        .filter_map(|(index, attachment)| {
            if attachment.kind == "text" {
                let text = attachment
                    .text_content
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .chars()
                    .take(12_000)
                    .collect::<String>();

                format!(
                    "附件 {}（文本）\n- 文件名：{}\n- MIME：{}\n- 大小：{} 字节\n- 正文开始\n{}\n- 正文结束",
                    index + 1,
                    attachment.name,
                    attachment.mime_type,
                    attachment.size,
                    text
                )
                .into()
            } else {
                image_attachment_count += 1;
                let has_inline_image = attachment
                    .data_url
                    .as_deref()
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false);

                if has_inline_image {
                    let ocr_section = crate::image_ocr::extract_text_from_attachment(attachment)
                        .ok()
                        .flatten()
                        .map(|text| text.chars().take(1_200).collect::<String>())
                        .filter(|text| !text.trim().is_empty())
                        .map(|text| format!(
                            "附件 {}（图片）补充 OCR\n- 文件名：{}\n- OCR 提示开始\n{}\n- OCR 提示结束",
                            index + 1,
                            attachment.name,
                            text
                        ));

                    ocr_section
                } else {
                    let ocr_text = crate::image_ocr::extract_text_from_attachment(attachment)
                        .ok()
                        .flatten()
                        .map(|text| text.chars().take(8_000).collect::<String>());
                    let ocr_section = ocr_text
                        .map(|text| format!("\n- OCR 提取文字开始\n{}\n- OCR 提取文字结束", text))
                        .unwrap_or_default();

                    Some(
                        format!(
                            "附件 {}（图片）\n- 文件名：{}\n- MIME：{}\n- 大小：{} 字节\n- 说明：当前仅收到图片附件元信息，尚未附带可供模型识别的图片内容；如需精确识别，请补充图片中的文字说明。",
                            index + 1,
                            attachment.name,
                            attachment.mime_type,
                            attachment.size,
                        ) + &ocr_section
                    )
                }
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let image_note = if image_attachment_count > 0 {
        format!(
            "用户本次还上传了 {} 张图片，图片数据已随本轮消息一并发送给模型。请优先直接结合图像内容回答，不要忽略图片，也不要要求用户重复上传。",
            image_attachment_count
        )
    } else {
        String::new()
    };

    format!(
        "用户本次还上传了 {} 个附件，请将它们视作本轮对话输入的一部分，并优先结合附件内容回答。若用户这轮提问使用“这是什么”“这张图”“这里写了什么”“图里内容”等指代性表达，默认就是在询问这些附件，尤其是图片内容。{}\n\n{}",
        attachments.len(),
        if image_note.is_empty() {
            String::new()
        } else {
            format!("\n\n{}", image_note)
        },
        sections
    )
}

fn build_user_message(req: &ChatRequest) -> String {
    let base = req.message.trim();
    let attachment_context = req
        .attachments
        .as_deref()
        .map(format_attachment_context)
        .unwrap_or_default();

    match (base.is_empty(), attachment_context.is_empty()) {
        (false, true) => base.to_string(),
        (true, false) => attachment_context,
        (false, false) => format!("{base}\n\n{attachment_context}"),
        (true, true) => String::new(),
    }
}

fn merge_session_artifacts(
    existing: Vec<Artifact>,
    current_turn: Vec<Artifact>,
) -> Vec<Artifact> {
    let mut merged = existing;

    for artifact in current_turn {
        if let Some(index) = merged.iter().position(|item| item.id == artifact.id) {
            merged[index] = artifact;
        } else {
            merged.insert(0, artifact);
        }
    }

    merged
}

async fn chat_stream(
    user: AuthUser,
    Json(req): Json<ChatRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let pool = state::db_pool();
    let client = std::sync::Arc::new(crate::llm::LlmClient::for_user(
        &user.0.id,
        req.model.as_deref(),
    ));

    // 创建或获取会话
    let session = if let Some(ref sid) = req.session_id {
        session_repo::find_by_id(&pool, sid)?.ok_or(AppError::NotFound("会话不存在".into()))?
    } else {
        let title_source = if req.message.trim().is_empty() {
            req.attachments
                .as_deref()
                .and_then(|items| items.first())
                .map(|item| format!("围绕附件：{}", item.name))
                .unwrap_or_else(|| "新的办公对话".to_string())
        } else {
            req.message.clone()
        };
        let title: String = title_source.chars().take(30).collect();
        session_repo::create(
            &pool,
            &user.0.id,
            req.project_id.as_deref(),
            req.tool_kind.as_deref(),
            &title,
        )?
    };

    let session_id = session.id.clone();

    // 加载历史消息
    let history = session_repo::get_messages(&pool, &session_id, 50).unwrap_or_default();
    let existing_artifacts = session_repo::get_artifacts(&pool, &session_id).unwrap_or_default();

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
    let user_message = build_user_message(&req);
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
    let existing_artifacts_for_save = existing_artifacts.clone();
    if let Some(attachments) = req.attachments.as_ref() {
        ctx.send(
            "state_update",
            serde_json::json!({
                "phase": "running",
                "step": "接收附件",
                "detail": format!("已接收 {} 个附件（支持 md / txt / 图片，图片将优先尝试视觉识别）", attachments.len()),
                "attachment_count": attachments.len(),
                "at": chrono::Utc::now().to_rfc3339(),
            }),
        );
    }

    // 启动 agent 循环
    let mut event_rx = run_agent_loop(
        history,
        user_message,
        req.attachments.clone().unwrap_or_default(),
        ctx,
        agent_config,
        client.clone(),
    )
    .await;
    let requested_tool_kind = req.tool_kind.clone();

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
                    let session_artifacts = merge_session_artifacts(
                        existing_artifacts_for_save.clone(),
                        collected_artifacts.clone(),
                    );
                    let _ = session_repo::save_artifacts(
                        &pool_for_save,
                        &session_id_for_save,
                        &session_artifacts,
                    );
                    Event::default().event("artifact_update").data(serde_json::json!({
                        "artifact": artifact,
                        "artifacts": session_artifacts,
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
                    if collected_artifacts.is_empty()
                        && requested_tool_kind.as_deref() == Some("doc")
                        && looks_like_markdown_document(&summary)
                    {
                        collected_artifacts.push(build_summary_markdown_artifact(&summary, requested_tool_kind.as_deref()));
                    }
                    let session_artifacts = merge_session_artifacts(
                        existing_artifacts_for_save.clone(),
                        collected_artifacts.clone(),
                    );
                    let _ = session_repo::update_summary(
                        &pool_for_save,
                        &session_id_for_save,
                        &summary.chars().take(240).collect::<String>(),
                    );
                    let _ = session_repo::save_artifacts(
                        &pool_for_save,
                        &session_id_for_save,
                        &session_artifacts,
                    );
                    Event::default().event("done").data(serde_json::json!({
                        "session_id": session_id_for_save,
                        "summary": summary,
                        "artifacts": session_artifacts,
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
                let _ =
                    session_repo::add_message(&pool_for_save, &session_id_for_save, &assistant_msg);
            }
        }
    });

    let stream = ReceiverStream::new(sse_rx);
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}
