use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::post;
use axum::{Json, Router};
use base64::Engine;
use futures::stream::Stream;
use std::collections::HashSet;
use std::convert::Infallible;
use std::path::{Path as FsPath, PathBuf};
use std::time::Duration;
use tokio_stream::wrappers::ReceiverStream;

use crate::agent::{run_agent_loop, AgentConfig, AgentEvent, IntentAnalyzer};
use crate::auth::middleware::AuthUser;
use crate::db::{file_repo, project_repo, session_repo, DbPool};
use crate::error::AppError;
use crate::models::{Artifact, ChatAttachment, ChatMessage, ChatRequest, PptProject};
use crate::render;
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

                    ocr_section.or_else(|| {
                        Some(format!(
                            "附件 {}（图片）\n- 文件名：{}\n- MIME：{}\n- 大小：{} 字节\n- 说明：图片内容已作为视觉输入随本轮消息一并发送，请直接观察图片回答用户问题。",
                            index + 1,
                            attachment.name,
                            attachment.mime_type,
                            attachment.size,
                        ))
                    })
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
            "用户本次还上传了 {} 张图片；服务端会把带 data URL 的图片作为视觉输入发送给模型，请优先直接结合图像内容回答。",
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

fn allowed_tools_for_kind(tool_kind: Option<&str>) -> Option<Vec<String>> {
    match tool_kind {
        Some("image") => Some(vec!["image_prompt".to_string()]),
        Some("video") => Some(vec!["video_generate".to_string()]),
        _ => None,
    }
}

/// 全局意图分析器（按会话隔离上下文）
static INTENT_ANALYZER: std::sync::OnceLock<std::sync::Mutex<IntentAnalyzer>> = std::sync::OnceLock::new();

fn intent_analyzer() -> &'static std::sync::Mutex<IntentAnalyzer> {
    INTENT_ANALYZER.get_or_init(|| std::sync::Mutex::new(IntentAnalyzer::new()))
}

fn contains_any(text: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|keyword| text.contains(keyword))
}

fn has_image_attachment(req: &ChatRequest) -> bool {
    req.attachments
        .as_deref()
        .map(|items| items.iter().any(|item| item.kind == "image"))
        .unwrap_or(false)
}

fn infer_media_tool_kind(req: &ChatRequest) -> Option<String> {
    match req.tool_kind.as_deref() {
        Some("image") | Some("video") => return req.tool_kind.clone(),
        _ => {}
    }

    if !has_image_attachment(req) {
        return None;
    }

    let text = req.message.trim().to_lowercase();

    // 文本优先意图排除：当用户明确要求"先写/构思/规划/出方案"时，
    // 当前意图是文本/文档生成，不应直接跳到媒体工具
    let text_first_patterns = [
        "先帮我写",
        "帮我写",
        "先写",
        "帮我构思",
        "帮我规划",
        "先出",
        "帮我出",
        "写提示词",
        "出提示词",
        "写prompt",
        "写脚本",
        "先规划",
        "先想",
        "帮我想想",
        "帮我设计",
        "先做个方案",
        "出个方案",
        "写个方案",
        "帮我写个",
        "先构思",
    ];
    let has_text_first_intent = text_first_patterns
        .iter()
        .any(|p| text.contains(p));
    if has_text_first_intent {
        return None; // 交给 LLM 自由选择工具
    }

    if contains_any(
        &text,
        &[
            "生成视频",
            "做视频",
            "制作视频",
            "图生视频",
            "以图生视频",
            "短视频",
            "短片",
            "宣传片",
            "动画",
            "动起来",
            "动态化",
            "动态海报",
            "motion",
            "video",
        ],
    ) {
        return Some("video".to_string());
    }

    let image_intent = contains_any(
        &text,
        &[
            "生成图片",
            "做图片",
            "画图",
            "出图",
            "图生图",
            "以图生图",
            "改图",
            "修图",
            "重绘",
            "换风格",
            "换背景",
            "换衣服",
            "换装",
            "变装",
            "换发型",
            "去除背景",
            "抠图",
            "扩图",
            "其他穿着",
            "穿着",
            "衣服",
            "服装",
            "造型",
            "换成",
            "改成",
            "海报",
            "封面",
            "配图",
            "主视觉",
            "插画",
            "banner",
            "视觉稿",
        ],
    );
    let reference_image_cue = contains_any(
        &text,
        &[
            "基于图片",
            "基于这张图",
            "基于这个图",
            "基于照片",
            "参考图片",
            "参考这张图",
            "用这张图",
            "按照这张图",
            "上传图片",
            "这张图片",
            "这张照片",
        ],
    );

    if image_intent || (reference_image_cue && !text.is_empty()) {
        Some("image".to_string())
    } else {
        None
    }
}

fn merge_session_artifacts(existing: Vec<Artifact>, current_turn: Vec<Artifact>) -> Vec<Artifact> {
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

fn user_file_dir(owner_id: &str) -> PathBuf {
    PathBuf::from(&crate::config::config().data_dir)
        .join("files")
        .join(owner_id)
}

fn sanitize_filename(name: &str, fallback: &str) -> String {
    let cleaned = name
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .collect::<String>()
        .trim()
        .to_string();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

fn ensure_extension(name: &str, extension: &str) -> String {
    if name.to_lowercase().ends_with(&extension.to_lowercase()) {
        name.to_string()
    } else {
        format!("{name}{extension}")
    }
}

fn infer_file_type(name: &str, mime_type: &str) -> String {
    let extension = FsPath::new(name)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase();
    match extension.as_str() {
        "ppt" | "pptx" => "ppt".into(),
        "doc" | "docx" | "md" | "markdown" | "txt" | "pdf" => "doc".into(),
        "xls" | "xlsx" | "csv" | "tsv" => "excel".into(),
        "drawio" | "xml" => "drawio".into(),
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" => "image".into(),
        "mp4" | "webm" | "mov" => "video".into(),
        _ if mime_type.starts_with("image/") => "image".into(),
        _ if mime_type.starts_with("video/") => "video".into(),
        _ if mime_type.contains("spreadsheet") || mime_type.contains("excel") => "excel".into(),
        _ if mime_type.contains("presentation") => "ppt".into(),
        _ if mime_type.contains("pdf")
            || mime_type.contains("word")
            || mime_type.starts_with("text/") =>
        {
            "doc".into()
        }
        _ => "other".into(),
    }
}

async fn save_file_bytes(
    pool: &DbPool,
    owner_id: &str,
    name: &str,
    data: &[u8],
    mime_type: &str,
    description: &str,
    metadata: serde_json::Value,
) -> anyhow::Result<()> {
    if data.is_empty() {
        return Ok(());
    }
    let safe_name = sanitize_filename(name, "walioffice-file.bin");
    let extension = FsPath::new(&safe_name)
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!(".{value}"))
        .unwrap_or_default();
    let storage_name = format!("{}{}", uuid::Uuid::new_v4(), extension);
    let storage_dir = user_file_dir(owner_id);
    tokio::fs::create_dir_all(&storage_dir).await?;
    let storage_path = storage_dir.join(storage_name);
    tokio::fs::write(&storage_path, data).await?;
    let file_type = infer_file_type(&safe_name, mime_type);
    file_repo::create_file(
        pool,
        owner_id,
        &safe_name,
        &storage_path.to_string_lossy(),
        &file_type,
        data.len() as i64,
        None,
        Some(description),
        Some(metadata),
    ).await?;
    Ok(())
}

fn decode_data_url(value: &str) -> Option<(String, Vec<u8>)> {
    let (header, payload) = value.split_once(',')?;
    if !header.starts_with("data:") || !header.contains(";base64") {
        return None;
    }
    let mime_type = header
        .trim_start_matches("data:")
        .split(';')
        .next()
        .unwrap_or("application/octet-stream")
        .to_string();
    let data = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .ok()?;
    Some((mime_type, data))
}

fn mime_extension(mime_type: &str, fallback: &str) -> String {
    match mime_type {
        "image/png" => ".png".into(),
        "image/jpeg" | "image/jpg" => ".jpg".into(),
        "image/webp" => ".webp".into(),
        "image/gif" => ".gif".into(),
        "video/mp4" => ".mp4".into(),
        "video/webm" => ".webm".into(),
        _ => fallback.into(),
    }
}

async fn download_remote_file(
    url: &str,
    expected_prefix: &str,
) -> anyhow::Result<(String, Vec<u8>)> {
    let response = reqwest::Client::new()
        .get(url)
        .timeout(Duration::from_secs(60))
        .send()
        .await?
        .error_for_status()?;
    let mime_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .filter(|value| value.starts_with(expected_prefix))
        .ok_or_else(|| anyhow::anyhow!("远程文件类型不匹配"))?
        .to_string();
    let data = response.bytes().await?.to_vec();
    Ok((mime_type, data))
}

async fn save_media_url_to_files(
    pool: &DbPool,
    owner_id: &str,
    artifact: &Artifact,
    url: &str,
    expected_prefix: &str,
    fallback_name: &str,
    fallback_extension: &str,
    description: &str,
    metadata: serde_json::Value,
) {
    if let Some((mime_type, data)) = decode_data_url(url) {
        let filename = artifact_filename(
            artifact,
            fallback_name,
            &mime_extension(&mime_type, fallback_extension),
        );
        let _ = save_file_bytes(
            pool,
            owner_id,
            &filename,
            &data,
            &mime_type,
            description,
            metadata,
        )
        .await;
        return;
    }

    if url.starts_with("http://") || url.starts_with("https://") {
        if let Ok((mime_type, data)) = download_remote_file(url, expected_prefix).await {
            let filename = artifact_filename(
                artifact,
                fallback_name,
                &mime_extension(&mime_type, fallback_extension),
            );
            let _ = save_file_bytes(
                pool,
                owner_id,
                &filename,
                &data,
                &mime_type,
                description,
                metadata,
            )
            .await;
            return;
        }
    }

    let filename = artifact_filename(artifact, &format!("{fallback_name}-link"), ".url.txt");
    let _ = save_file_bytes(
        pool,
        owner_id,
        &filename,
        url.as_bytes(),
        "text/plain",
        description,
        metadata,
    )
    .await;
}

async fn save_chat_attachments_to_files(
    pool: &DbPool,
    owner_id: &str,
    attachments: &[ChatAttachment],
) {
    for attachment in attachments {
        let description = format!("聊天上传附件：{}", attachment.name);
        if let Some(data_url) = attachment.data_url.as_deref() {
            if let Some((mime_type, data)) = decode_data_url(data_url) {
                let filename = ensure_extension(
                    &sanitize_filename(&attachment.name, "chat-image"),
                    &mime_extension(&mime_type, ".bin"),
                );
                let _ = save_file_bytes(
                    pool,
                    owner_id,
                    &filename,
                    &data,
                    &mime_type,
                    &description,
                    serde_json::json!({
                        "source": "chat_attachment",
                        "original_name": attachment.name,
                        "mime_type": mime_type,
                    }),
                )
                .await;
                continue;
            }
        }

        if let Some(text) = attachment.text_content.as_deref() {
            if !text.trim().is_empty() {
                let lower_name = attachment.name.to_lowercase();
                let filename = if lower_name.ends_with(".md")
                    || lower_name.ends_with(".markdown")
                    || lower_name.ends_with(".txt")
                    || lower_name.ends_with(".csv")
                    || lower_name.ends_with(".json")
                    || lower_name.ends_with(".tsv")
                {
                    sanitize_filename(&attachment.name, "chat-attachment.txt")
                } else {
                    ensure_extension(
                        &format!(
                            "{}-文本副本",
                            sanitize_filename(&attachment.name, "chat-attachment")
                        ),
                        ".txt",
                    )
                };
                let _ = save_file_bytes(
                    pool,
                    owner_id,
                    &filename,
                    text.as_bytes(),
                    attachment.mime_type.as_str(),
                    &description,
                    serde_json::json!({
                        "source": "chat_attachment_text_copy",
                        "original_name": attachment.name,
                        "mime_type": attachment.mime_type,
                    }),
                )
                .await;
            }
        }
    }
}

fn artifact_filename(artifact: &Artifact, fallback: &str, extension: &str) -> String {
    ensure_extension(&sanitize_filename(&artifact.title, fallback), extension)
}

async fn save_generated_artifact_to_files(pool: &DbPool, owner_id: &str, artifact: &Artifact) {
    tracing::info!("保存生成产物到文件: kind={}, title={}, artifact_id={}", artifact.kind, artifact.title, artifact.id);
    let description = format!("智能助手生成：{}", artifact.title);
    let metadata = serde_json::json!({
        "source": "generated_artifact",
        "artifact_id": artifact.id,
        "artifact_kind": artifact.kind,
        "tool_kind": artifact.tool_kind,
    });

    match artifact.kind.as_str() {
        "document" => {
            let sections = artifact
                .content
                .get("sections")
                .cloned()
                .unwrap_or(serde_json::json!([]));
            if let Ok(sections) =
                serde_json::from_value::<Vec<render::docx_render::DocSection>>(sections)
            {
                let data = render::docx_render::DocData {
                    title: artifact.title.clone(),
                    sections,
                };
                let filename = artifact_filename(artifact, "document", ".docx");
                let path = render::output_path(&filename);
                if render::docx_render::render_docx(&data, &path).is_ok() {
                    if let Ok(bytes) = tokio::fs::read(&path).await {
                        let _ = save_file_bytes(pool, owner_id, &filename, &bytes, "application/vnd.openxmlformats-officedocument.wordprocessingml.document", &description, metadata).await;
                    }
                    return;
                }
            }
            if let Some(markdown) = artifact
                .content
                .get("markdown")
                .and_then(|value| value.as_str())
            {
                let filename = artifact_filename(artifact, "document", ".md");
                let _ = save_file_bytes(
                    pool,
                    owner_id,
                    &filename,
                    markdown.as_bytes(),
                    "text/markdown",
                    &description,
                    metadata,
                )
                .await;
            }
        }
        "sheet" => {
            let tables = artifact
                .content
                .get("tables")
                .cloned()
                .unwrap_or(serde_json::json!([]));
            if let Ok(tables) =
                serde_json::from_value::<Vec<render::xlsx_render::SheetTable>>(tables)
            {
                let data = render::xlsx_render::SheetData {
                    title: artifact.title.clone(),
                    tables,
                };
                let filename = artifact_filename(artifact, "spreadsheet", ".xlsx");
                let path = render::output_path(&filename);
                if render::xlsx_render::render_xlsx(&data, &path).is_ok() {
                    if let Ok(bytes) = tokio::fs::read(&path).await {
                        let _ = save_file_bytes(
                            pool,
                            owner_id,
                            &filename,
                            &bytes,
                            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                            &description,
                            metadata,
                        )
                        .await;
                    }
                }
            }
        }
        "markdown" => {
            if let Some(markdown) = artifact
                .content
                .get("markdown")
                .and_then(|value| value.as_str())
            {
                let filename = artifact_filename(artifact, "document", ".md");
                let _ = save_file_bytes(
                    pool,
                    owner_id,
                    &filename,
                    markdown.as_bytes(),
                    "text/markdown",
                    &description,
                    metadata,
                )
                .await;
            }
        }
        "drawio" => {
            if let Some(xml) = artifact.content.get("xml").and_then(|value| value.as_str()) {
                let filename = artifact_filename(artifact, "diagram", ".drawio");
                let _ = save_file_bytes(
                    pool,
                    owner_id,
                    &filename,
                    xml.as_bytes(),
                    "application/xml",
                    &description,
                    metadata,
                )
                .await;
            }
        }
        "ppt" => {
            let maybe_project_id = artifact
                .content
                .get("project_id")
                .and_then(|value| value.as_str());
            let project = maybe_project_id
                .and_then(|project_id| project_repo::load_ppt_project(project_id).ok().flatten());
            let project = project.or_else(|| {
                let slides = artifact.content.get("slides")?.clone();
                let slides = serde_json::from_value(slides).ok()?;
                Some(PptProject {
                    id: artifact
                        .content
                        .get("project_id")
                        .and_then(|value| value.as_str())
                        .unwrap_or(&artifact.id)
                        .to_string(),
                    title: artifact.title.clone(),
                    theme: artifact
                        .content
                        .get("theme")
                        .and_then(|value| value.as_str())
                        .unwrap_or("default")
                        .to_string(),
                    slides,
                    history: None,
                    layout: "16x9".to_string(),
                    created_at: artifact.created_at.clone(),
                    updated_at: artifact.updated_at.clone(),
                    owner_id: owner_id.to_string(),
                })
            });
            if let Some(project) = project {
                let filename = artifact_filename(artifact, "presentation", ".pptx");
                let path = render::output_path(&filename);
                if render::pptx_render::render_pptx(&project, &path).is_ok() {
                    if let Ok(bytes) = tokio::fs::read(&path).await {
                        let _ = save_file_bytes(pool, owner_id, &filename, &bytes, "application/vnd.openxmlformats-officedocument.presentationml.presentation", &description, metadata).await;
                    }
                }
            }
        }
        "image" => {
            if let Some(url) = artifact
                .content
                .get("images")
                .and_then(|value| value.as_array())
                .and_then(|items| items.first())
                .and_then(|value| value.as_str())
            {
                save_media_url_to_files(
                    pool,
                    owner_id,
                    artifact,
                    url,
                    "image/",
                    "image",
                    ".png",
                    &description,
                    metadata,
                )
                .await;
            }
        }
        "video" => {
            if let Some(url) = artifact
                .content
                .get("video_url")
                .and_then(|value| value.as_str())
            {
                save_media_url_to_files(
                    pool,
                    owner_id,
                    artifact,
                    url,
                    "video/",
                    "video",
                    ".mp4",
                    &description,
                    metadata,
                )
                .await;
            }
        }
        _ => {
            let filename = artifact_filename(artifact, "artifact", ".json");
            if let Ok(bytes) = serde_json::to_vec_pretty(artifact) {
                let _ = save_file_bytes(
                    pool,
                    owner_id,
                    &filename,
                    &bytes,
                    "application/json",
                    &description,
                    metadata,
                )
                .await;
            }
        }
    }
}

async fn chat_stream(
    user: AuthUser,
    Json(req): Json<ChatRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let pool = state::db_pool();

    // ── 意图识别 ────────────────────────────────────────────────────────
    let has_image = has_image_attachment(&req);
    let intent_result = {
        let mut analyzer = intent_analyzer().lock().unwrap();
        analyzer.analyze(&req.message, &req.session_id.clone().unwrap_or_default(), has_image)
    };
    tracing::info!(
        "[Intent] session={} intent={:?} confidence={:.2} source={:?} temporal={:?}",
        req.session_id.as_deref().unwrap_or("-"),
        intent_result.intent,
        intent_result.confidence,
        intent_result.source,
        intent_result.temporal_order,
    );

    // 优先用意图识别结果，前端 tool_kind 作为 fallback
    let resolved_tool_kind = if intent_result.confidence >= 0.5 {
        // 意图识别置信度足够时，用意图结果决定 allowed_tools
        match intent_result.intent {
            crate::agent::intent::IntentType::Image => Some("image".to_string()),
            crate::agent::intent::IntentType::Video => Some("video".to_string()),
            _ => req.tool_kind.clone(), // 其他意图交给前端 tool_kind 或 None
        }
    } else {
        // 低置信度时 fallback 到旧逻辑
        infer_media_tool_kind(&req).or_else(|| req.tool_kind.clone())
    };

    let client = std::sync::Arc::new(crate::llm::LlmClient::for_user(
        &user.0.id,
        req.model.as_deref(),
    ).await);

    // 创建或获取会话
    let session = if let Some(ref sid) = req.session_id {
        session_repo::find_by_id(&pool, sid).await?.ok_or(AppError::NotFound("会话不存在".into()))?
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
            resolved_tool_kind.as_deref(),
            &title,
        ).await?
    };

    let session_id = session.id.clone();

    // 加载历史消息
    let history = session_repo::get_messages(&pool, &session_id, 50).await.unwrap_or_default();
    let existing_artifacts = session_repo::get_artifacts(&pool, &session_id).await.unwrap_or_default();

    if let Some(attachments) = req.attachments.as_deref() {
        save_chat_attachments_to_files(&pool, &user.0.id, attachments).await;
    }

    // 保存用户消息
    let user_msg = ChatMessage {
        role: "user".into(),
        content: req.message.clone(),
        tool_calls: None,
        tool_call_id: None,
    };
    let _ = session_repo::add_message(&pool, &session_id, &user_msg).await;

    // 创建 SSE channel
    let (sse_tx, sse_rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(256);

    // clone 必要数据给 agent 任务
    let user_id = user.0.id.clone();
    let project_id = req.project_id.clone();
    let user_message = build_user_message(&req);

    // 构建意图上下文注入
    let intent_addition = {
        let analyzer = intent_analyzer().lock().unwrap();
        analyzer.build_intent_context_addition(&intent_result)
    };
    let system_prompt = if intent_addition.is_empty() {
        String::new() // 使用默认 OFFICE_AGENT_PROMPT
    } else {
        intent_addition // 仅注入意图补充，agent_loop 会拼接到 OFFICE_AGENT_PROMPT 后面
    };

    // 用意图识别结果决定 allowed_tools
    let allowed_tools = if intent_result.confidence >= 0.5 {
        {
            let analyzer = intent_analyzer().lock().unwrap();
            analyzer.allowed_tools_for_intent(&intent_result)
        }
    } else {
        allowed_tools_for_kind(resolved_tool_kind.as_deref())
    };

    let agent_config = AgentConfig {
        max_turns: 6,
        system_prompt,
        allowed_tools,
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
        req.attachments.clone().unwrap_or_default(),
        emit,
    ).with_tool_config(req.tool_config.clone().unwrap_or(serde_json::json!({})));

    let session_id_for_save = session_id.clone();
    let pool_for_save = pool.clone();
    let user_id_for_save = user_id.clone();
    let existing_artifacts_for_save = existing_artifacts.clone();
    if let Some(attachments) = req.attachments.as_ref() {
        ctx.send(
            "state_update",
            serde_json::json!({
                "phase": "running",
                "step": "接收附件",
                "detail": format!("已接收 {} 个附件（支持 md / txt / csv / json / docx / xlsx / pptx / pdf / 图片；图片会优先尝试视觉识别，若视觉输入失败则退化为 OCR/文本辅助）", attachments.len()),
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
    let requested_tool_kind = resolved_tool_kind.clone();

    tokio::spawn(async move {
        let mut final_summary = String::new();
        let mut collected_artifacts: Vec<crate::models::Artifact> = Vec::new();
        let mut saved_artifact_ids = HashSet::new();

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
                    if saved_artifact_ids.insert(artifact.id.clone()) {
                        save_generated_artifact_to_files(
                            &pool_for_save,
                            &user_id_for_save,
                            artifact,
                        )
                        .await;
                    }
                    let session_artifacts = merge_session_artifacts(
                        existing_artifacts_for_save.clone(),
                        collected_artifacts.clone(),
                    );
                    let _ = session_repo::save_artifacts(
                        &pool_for_save,
                        &session_id_for_save,
                        &session_artifacts,
                    ).await;
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
                    for artifact in &collected_artifacts {
                        if saved_artifact_ids.insert(artifact.id.clone()) {
                            save_generated_artifact_to_files(
                                &pool_for_save,
                                &user_id_for_save,
                                artifact,
                            )
                            .await;
                        }
                    }
                    let _ = session_repo::update_summary(
                        &pool_for_save,
                        &session_id_for_save,
                        &summary.chars().take(240).collect::<String>(),
                    ).await;
                    let _ = session_repo::save_artifacts(
                        &pool_for_save,
                        &session_id_for_save,
                        &session_artifacts,
                    ).await;
                    Event::default().event("done").data(serde_json::json!({
                        "session_id": session_id_for_save,
                        "summary": summary,
                        "artifacts": session_artifacts,
                        "new_artifacts": collected_artifacts,
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
                    session_repo::add_message(&pool_for_save, &session_id_for_save, &assistant_msg).await;
            }
        }
    });

    let stream = ReceiverStream::new(sse_rx);
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}
