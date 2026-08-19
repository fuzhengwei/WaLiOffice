use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tokio::time::{sleep, Instant};

use crate::agent::tool::{OfficeTool, ToolArtifact, ToolContext, ToolResult};
use crate::llm::LlmClient;
use crate::models::{ChatAttachment, ChatMessage};

use super::agnes_media::{
    agnes_video_model, get_json, http_client, post_json, resolve_video_credentials,
};
use super::local_video;

pub struct VideoGenerateTool;

/// V2.5 视频生成方案
#[derive(Debug, Clone, Deserialize, Serialize)]
struct VideoPlan {
    title: String,
    description: String,
    prompt: String,
    negative_prompt: String,
    aspect_ratio: String,
    seconds: u8, // V2.5: 4-12 秒
    mode: String, // V2.5: text | keyframe | reference
}

/// V2.5 创建视频任务响应
#[derive(Debug, Deserialize)]
struct CreateVideoResponse {
    id: String,
    object: Option<String>,
    model: Option<String>,
    status: Option<String>,
    progress: Option<u32>,
    created_at: Option<u64>,
    size: Option<String>,
    seconds: Option<String>,
    quality: Option<String>,
    url: Option<String>,
    error: Option<serde_json::Value>,
}

/// V2.5 查询视频任务响应
#[derive(Debug, Deserialize)]
struct QueryVideoResponse {
    id: Option<String>,
    object: Option<String>,
    model: Option<String>,
    status: String,
    progress: Option<u32>,
    created_at: Option<u64>,
    completed_at: Option<u64>,
    size: Option<String>,
    seconds: Option<String>,
    quality: Option<String>,
    url: Option<String>,
    metadata: Option<VideoMetadata>,
    error: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct VideoMetadata {
    url: Option<String>,
}

fn parse_video_plan(content: &str) -> Result<VideoPlan, String> {
    let value =
        LlmClient::extract_json(content).map_err(|err| format!("视频提示词解析失败: {err}"))?;
    serde_json::from_value::<VideoPlan>(value).map_err(|err| format!("视频提示词结构不正确: {err}"))
}

pub fn normalize_aspect_ratio(input: &str) -> &'static str {
    match input.trim() {
        "9:16" => "9:16",
        "1:1" => "1:1",
        "4:3" => "4:3",
        "3:4" => "3:4",
        "21:9" => "21:9",
        _ => "16:9",
    }
}

/// V2.5 尺寸映射：返回 (size_label, width, height)
fn infer_size_and_dimensions(aspect_ratio: &str) -> (&'static str, u32, u32) {
    match aspect_ratio {
        "9:16" => ("720P", 720, 1280),
        "1:1" => ("720P", 720, 720),
        "4:3" => ("720P", 960, 720),
        "3:4" => ("720P", 720, 960),
        "21:9" => ("720P", 1680, 720),
        _ => ("720P", 1280, 720),
    }
}

/// V2.5 时长校验：4-12 秒
pub fn normalize_seconds(input: u8) -> u8 {
    input.clamp(4, 12)
}

fn parse_seconds(input: &str) -> u8 {
    let trimmed = input.trim();
    // 直接数字
    if let Ok(n) = trimmed.parse::<u8>() {
        return normalize_seconds(n);
    }
    // 档位映射（向后兼容旧配置）
    match trimmed.to_lowercase().as_str() {
        "short" => 4,
        "standard" => 5,
        "long" => 8,
        "max" => 12,
        _ => 5,
    }
}

fn collect_video_images(ctx: &ToolContext, input: &serde_json::Value) -> Vec<String> {
    let mut images = input
        .get("image_urls")
        .or_else(|| input.get("images"))
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if let Some(image) = input
        .get("image_url")
        .or_else(|| input.get("image"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        images.push(image.to_string());
    }

    // 从本轮上传附件中提取图片
    if images.is_empty() {
        images.extend(ctx.attachments.iter().filter_map(attachment_to_data_url));
    }

    // P2: 从会话历史产物中提取图片 URL
    // 当用户说"用刚才那张图""用上面的图片做视频"等，自动从 prior_artifacts 中找图片
    if images.is_empty() {
        images.extend(extract_image_urls_from_artifacts(&ctx.prior_artifacts));
    }

    images
}

/// 从产物历史中提取图片 URL
/// 支持 image 类型产物的 content.image_url / content.url / content.data_url
/// 也支持 PPT/sheet 等产物中嵌入的图片（未来扩展）
pub fn extract_image_urls_from_artifacts(artifacts: &[crate::models::Artifact]) -> Vec<String> {
    let mut urls = Vec::new();
    for artifact in artifacts {
        // 只提取图片类型产物
        if artifact.kind != "image" {
            continue;
        }
        // 从 content JSON 中提取 URL
        if let Some(url) = artifact
            .content
            .get("image_url")
            .or_else(|| artifact.content.get("url"))
            .or_else(|| artifact.content.get("data_url"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            urls.push(url.to_string());
        }
    }
    urls
}

/// 从产物历史中提取视频 URL
/// 用于 videos[] 参考素材："基于这个视频的风格再做一段"
pub fn extract_video_urls_from_artifacts(artifacts: &[crate::models::Artifact]) -> Vec<String> {
    let mut urls = Vec::new();
    for artifact in artifacts {
        if artifact.kind != "video" {
            continue;
        }
        // 跳过分镜方案（无 video_url）
        let art_type = artifact.content.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if art_type == "video_storyboard" || art_type == "video_batch_summary" {
            continue;
        }
        if let Some(url) = artifact
            .content
            .get("video_url")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            urls.push(url.to_string());
        }
    }
    urls
}

/// 从输入参数和产物历史中收集音频 URL
/// 来源优先级：显式 audio_urls → 会话历史视频产物
pub fn collect_video_audios(input: &serde_json::Value, artifacts: &[crate::models::Artifact]) -> Vec<String> {
    let mut audios = Vec::new();

    // 1. 显式传入的 audio_urls
    if let Some(arr) = input.get("audio_urls").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(s) = item.as_str() {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    audios.push(trimmed.to_string());
                }
            }
        }
    }

    // 2. 单个 audio_url
    if let Some(url) = input.get("audio_url").and_then(|v| v.as_str()).map(str::trim).filter(|s| !s.is_empty()) {
        audios.push(url.to_string());
    }

    audios
}

/// 从输入参数和产物历史中收集视频参考 URL
/// 来源优先级：显式 video_urls → 会话历史视频产物
pub fn collect_video_refs(input: &serde_json::Value, artifacts: &[crate::models::Artifact]) -> Vec<VideoRef> {
    let mut refs = Vec::new();

    // 1. 显式传入的 video_urls（字符串数组或对象数组）
    if let Some(arr) = input.get("video_urls").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(s) = item.as_str() {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    refs.push(VideoRef { url: trimmed.to_string(), start_seconds: None, require_audio: None });
                }
            } else if let Some(obj) = item.as_object() {
                if let Some(url) = obj.get("url").and_then(|v| v.as_str()).map(str::trim).filter(|s| !s.is_empty()) {
                    refs.push(VideoRef {
                        url: url.to_string(),
                        start_seconds: obj.get("start_seconds").and_then(|v| v.as_u64()).map(|n| n as f64),
                        require_audio: obj.get("require_audio").and_then(|v| v.as_bool()),
                    });
                }
            }
        }
    }

    // 2. 单个 video_url
    if let Some(url) = input.get("video_url").and_then(|v| v.as_str()).map(str::trim).filter(|s| !s.is_empty()) {
        refs.push(VideoRef { url: url.to_string(), start_seconds: None, require_audio: None });
    }

    // 3. 从会话历史产物中提取视频 URL（用户说"基于这个视频再做一段"）
    if refs.is_empty() {
        for url in extract_video_urls_from_artifacts(artifacts) {
            refs.push(VideoRef { url, start_seconds: None, require_audio: None });
        }
    }

    refs
}

/// 视频参考素材（V2.5 videos[] 参数）
#[derive(Debug, Clone)]
pub struct VideoRef {
    pub url: String,
    pub start_seconds: Option<f64>,
    pub require_audio: Option<bool>,
}

impl VideoRef {
    pub fn to_json(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        obj.insert("url".into(), json!(self.url));
        if let Some(ss) = self.start_seconds {
            obj.insert("start_seconds".into(), json!(ss));
        }
        if let Some(ra) = self.require_audio {
            obj.insert("require_audio".into(), json!(ra));
        }
        serde_json::Value::Object(obj)
    }
}

/// 从历史产物中提取文字内容（文档/PPT/表格/markdown/drawio），用作视频文案素材。
/// 返回 (标题, 内容摘要) 列表，每条最多 800 字。
pub fn extract_text_content_from_artifacts(
    artifacts: &[crate::models::Artifact],
) -> Vec<(String, String)> {
    let mut results = Vec::new();
    for artifact in artifacts {
        match artifact.kind.as_str() {
            "document" | "markdown" => {
                let text = artifact
                    .content
                    .get("content")
                    .or_else(|| artifact.content.get("text"))
                    .or_else(|| artifact.content.get("markdown"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !text.is_empty() {
                    let truncated = truncate_text(text, 800);
                    results.push((artifact.title.clone(), truncated));
                }
            }
            "ppt" => {
                // 从 slides 数组提取每页的 title + content
                if let Some(slides) = artifact.content.get("slides").and_then(|v| v.as_array()) {
                    let mut ppt_text = String::new();
                    for (i, slide) in slides.iter().enumerate() {
                        let title = slide.get("title").and_then(|v| v.as_str()).unwrap_or("");
                        let content = slide.get("content").and_then(|v| v.as_str()).unwrap_or("");
                        if !title.is_empty() {
                            ppt_text.push_str(&format!("第{}页：{}\n", i + 1, title));
                        }
                        if !content.is_empty() {
                            ppt_text.push_str(&format!("{}\n", truncate_text(content, 100)));
                        }
                    }
                    if !ppt_text.is_empty() {
                        results.push((
                            format!("PPT: {}", artifact.title),
                            truncate_text(&ppt_text, 800),
                        ));
                    }
                }
            }
            "sheet" => {
                // 提取表格的 summary 或 headers + rows
                let summary = artifact
                    .content
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let headers = artifact
                    .content
                    .get("headers")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                if !summary.is_empty() || !headers.is_empty() {
                    let text = if !summary.is_empty() {
                        summary.to_string()
                    } else {
                        format!("列: {}", headers)
                    };
                    results.push((
                        format!("表格: {}", artifact.title),
                        truncate_text(&text, 800),
                    ));
                }
            }
            "drawio" => {
                // drawio 产物可能有 description
                let desc = artifact
                    .content
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !desc.is_empty() {
                    results.push((
                        format!("图表: {}", artifact.title),
                        truncate_text(desc, 400),
                    ));
                }
            }
            _ => {}
        }
    }
    results
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}

pub fn attachment_to_data_url(attachment: &ChatAttachment) -> Option<String> {
    if attachment.kind != "image" {
        return None;
    }
    attachment
        .data_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

/// V2.5 智能模式选择：根据图片数量自动路由
/// - 0 图 → text
/// - 1 图 → keyframe (first_frame)
/// - 2 图 → keyframe (first_frame + last_frame)
/// - 3+ 图 → reference (images[] + <Picture N> 占位符)
fn infer_mode(input_mode: &str, image_count: usize) -> &'static str {
    match input_mode.trim() {
        "text" => "text",
        "keyframe" | "first_frame" | "last_frame" | "keyframes" => "keyframe",
        "reference" => "reference",
        // 自动路由
        _ if image_count == 0 => "text",
        _ if image_count <= 2 => "keyframe",
        _ => "reference",
    }
}

fn default_video_plan(topic: &str, aspect_ratio: &str, seconds: u8, mode: &str) -> VideoPlan {
    VideoPlan {
        title: topic.chars().take(24).collect::<String>(),
        description: format!("围绕\u{201c}{topic}\u{201d}生成一支具有明确主体运动和镜头变化的短视频。"),
        prompt: format!(
            "Create a polished short video about: {topic}. Use a clear main subject, visible motion, cinematic camera movement, warm lighting, and an engaging commercial visual style."
        ),
        negative_prompt: "low quality, blurry, distorted, flicker, watermark, text artifacts".into(),
        aspect_ratio: aspect_ratio.into(),
        seconds,
        mode: mode.into(),
    }
}

async fn local_video_artifact(
    ctx: &ToolContext,
    topic: &str,
    plan: &VideoPlan,
    width: u32,
    height: u32,
    frame_rate: u32,
    num_frames: u32,
    generation_mode: &str,
    reference_image_count: usize,
    fallback_reason: impl Into<String>,
) -> ToolResult {
    let fallback_reason = fallback_reason.into();
    ctx.send(
        "state_update",
        json!({
            "phase": "running",
            "step": "本地视频兜底",
            "detail": format!("远程视频服务暂不可用，正在本地合成可播放 MP4：{fallback_reason}"),
            "at": chrono::Utc::now().to_rfc3339(),
        }),
    );

    match local_video::generate_local_video(
        topic,
        &plan.aspect_ratio,
        width,
        height,
        num_frames,
        frame_rate,
    )
    .await
    {
        Ok(output) => ToolResult::ok(
            format!("已为《{topic}》生成本地兜底视频；远程服务失败原因：{fallback_reason}"),
            vec![ToolArtifact {
                kind: "video".into(),
                title: plan.title.clone(),
                content: json!({
                    "type": "generated_video",
                    "title": plan.title,
                    "description": format!("{}（本地兜底合成，可直接预览和下载）", plan.description),
                    "prompt": plan.prompt,
                    "negative_prompt": plan.negative_prompt,
                    "video_url": output.public_url,
                    "file_path": output.file_path,
                    "status": "completed",
                    "progress": 100,
                    "seconds": format!("{:.1}", output.seconds),
                    "size": output.size,
                    "aspect_ratio": plan.aspect_ratio,
                    "duration": format!("{}s", plan.seconds),
                    "generation_mode": generation_mode,
                    "reference_image_count": reference_image_count,
                    "frame_rate": frame_rate,
                    "frame_count": output.frame_count,
                    "provider": "local_ffmpeg_fallback",
                    "model": "local-motion-storyboard",
                    "fallback_reason": fallback_reason,
                }),
            }],
        ),
        Err(local_err) => ToolResult::err(format!(
            "远程视频服务失败：{fallback_reason}；本地兜底视频也生成失败：{local_err}"
        )),
    }
}

#[async_trait]
impl OfficeTool for VideoGenerateTool {
    fn name(&self) -> &str {
        "video_generate"
    }

    fn description(&self) -> &str {
        "生成视频结果：基于 Agnes Video V2.5 创建视频任务并返回可直接预览的 mp4 视频链接。支持三种模式：text（文生视频）、keyframe（首尾帧控制）、reference（多模态参考）。系统根据图片数量自动选择最优模式：0图→text，1-2图→keyframe，3+图→reference。支持从会话历史产物中自动提取图片。"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "topic": { "type": "string", "description": "视频需求描述，例如产品短片、活动宣传片、品牌动画、功能演示" },
                "aspect_ratio": {
                    "type": "string",
                    "description": "可选宽高比：16:9（默认）、9:16、1:1、4:3、3:4、21:9"
                },
                "seconds": {
                    "type": "integer",
                    "description": "视频时长，4-12 秒，默认 5 秒"
                },
                "mode": {
                    "type": "string",
                    "description": "生成模式：text（文生视频）、keyframe（首尾帧控制）、reference（多模态参考）。不传时根据图片数量自动选择"
                },
                "image_url": { "type": "string", "description": "参考图片 URL 或 data URL，可选；不传时自动使用本轮上传图片" },
                "image_urls": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "多图参考 URL / data URL 列表。1-2 张走 keyframe 模式，3+ 张走 reference 模式"
                },
                "audio_urls": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "音频参考 URL 列表。在 prompt 中用 <Audio N> 引用，用于根据音乐节奏设计动作和镜头切换"
                },
                "video_urls": {
                    "type": "array",
                    "description": "视频参考素材列表。可为字符串 URL 或对象 {url, start_seconds, require_audio}。在 prompt 中用 <Video N> 引用，用于视频续写、风格迁移",
                    "items": {
                        "oneOf": [
                            { "type": "string" },
                            {
                                "type": "object",
                                "properties": {
                                    "url": { "type": "string" },
                                    "start_seconds": { "type": "number", "description": "从参考视频的指定秒数开始读取" },
                                    "require_audio": { "type": "boolean", "description": "是否要求参考视频必须包含音轨" }
                                },
                                "required": ["url"]
                            }
                        ]
                    }
                },
                "negative_prompt": { "type": "string", "description": "负面提示词，描述不希望出现的内容" },
                "seed": { "type": "integer", "description": "随机种子，相同种子可提高可复现性" }
            },
            "required": ["topic"]
        })
    }

    async fn call(&self, input: serde_json::Value, ctx: &ToolContext) -> ToolResult {
        let topic = input
            .get("topic")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if topic.is_empty() {
            return ToolResult::err("topic 不能为空");
        }

        // ---- 参数解析 ----
        let config_aspect_ratio = ctx.get_config::<String>("aspect_ratio");
        let requested_aspect_ratio = normalize_aspect_ratio(
            config_aspect_ratio
                .as_deref()
                .or_else(|| input.get("aspect_ratio").and_then(|v| v.as_str()))
                .unwrap_or("16:9"),
        );

        let config_seconds = ctx.get_config::<String>("seconds");
        let config_duration = ctx.get_config::<String>("duration");
        let requested_seconds = config_seconds
            .as_deref()
            .map(parse_seconds)
            .or_else(|| {
                config_duration
                    .as_deref()
                    .map(parse_seconds)
            })
            .or_else(|| input.get("seconds").and_then(|v| v.as_u64()).map(|n| normalize_seconds(n as u8)))
            .unwrap_or(5);

        let image_inputs = collect_video_images(ctx, &input);
        let audio_inputs = collect_video_audios(&input, &ctx.prior_artifacts);
        let video_refs = collect_video_refs(&input, &ctx.prior_artifacts);

        let config_mode = ctx.get_config::<String>("mode");
        let input_mode = config_mode
            .as_deref()
            .or_else(|| input.get("mode").and_then(|v| v.as_str()))
            .unwrap_or("");
        let generation_mode = infer_mode(input_mode, image_inputs.len());

        let negative_prompt_input = input
            .get("negative_prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let seed_input = input.get("seed").and_then(|v| v.as_u64());

        tracing::info!("[VideoConfig] aspect_ratio={}, seconds={}, mode={}, image_count={}, audio_count={}, video_ref_count={}, tool_config={:?}",
            requested_aspect_ratio, requested_seconds, generation_mode, image_inputs.len(), audio_inputs.len(), video_refs.len(), ctx.tool_config);

        // ---- LLM 规划视频脚本 ----
        ctx.send(
            "state_update",
            json!({
                "phase": "running",
                "step": "规划视频脚本",
                "detail": format!("正在为《{topic}》生成视频提示词与镜头描述..."),
                "at": chrono::Utc::now().to_rfc3339(),
            }),
        );

        let system_prompt = r#"你是资深导演兼 Agnes Video V2.5 提示词专家。只输出严格 JSON，不要 markdown。
返回格式：
{
  "title": "视频标题",
  "description": "视频创意说明",
  "prompt": "可直接用于 Agnes Video V2.5 的英文提示词",
  "negative_prompt": "需要避免的内容",
  "aspect_ratio": "16:9",
  "seconds": 5,
  "mode": "text"
}
要求：
- 提示词遵循 [主体与场景] + [动作与变化] + [镜头语言] + [视觉风格] + [声音与节奏]
- text 模式：描述完整动态画面
- keyframe 模式：描述从首帧到尾帧的过渡运动，强调 smooth transition
- reference 模式：使用 <Picture N> 占位符指代输入图片，例如"以 <Picture 1> 中的角色为参考..."
- 音频参考：使用 <Audio N> 占位符指代音频，例如"根据 <Audio 1> 的节奏设计镜头切换"
- 视频参考：使用 <Video N> 占位符指代视频，例如"延续 <Video 1> 的画面风格和角色形象"
- 音画协同：可同时引用图片和音频，例如"以 <Picture 1> 为视觉主体，根据 <Audio 1> 的节奏设计动作"
- seconds 只能输出 4-12 的整数
- mode 只能输出 text / keyframe / reference
- aspect_ratio 只能输出 16:9 / 9:16 / 1:1 / 4:3 / 3:4 / 21:9"#;

        let reference_guidance = if image_inputs.is_empty() && audio_inputs.is_empty() && video_refs.is_empty() {
            "无参考素材，请按 text 模式生成完整动态画面。".to_string()
        } else {
            let mut parts = Vec::new();
            if !image_inputs.is_empty() {
                if image_inputs.len() <= 2 {
                    parts.push(format!("有 {} 张参考图，使用 keyframe 模式。首帧/尾帧由系统自动分配，请描述帧间过渡运动。", image_inputs.len()));
                } else {
                    parts.push(format!("有 {} 张参考图，使用 reference 模式。请在 prompt 中用 <Picture 1>、<Picture 2> 等占位符引用图片。", image_inputs.len()));
                }
            }
            if !audio_inputs.is_empty() {
                parts.push(format!("有 {} 个音频参考，请在 prompt 中用 <Audio 1>、<Audio 2> 等占位符引用音频，根据音乐节奏设计动作和镜头切换。", audio_inputs.len()));
            }
            if !video_refs.is_empty() {
                parts.push(format!("有 {} 个视频参考素材，请在 prompt 中用 <Video 1>、<Video 2> 等占位符引用视频，用于续写、风格迁移或动作模仿。", video_refs.len()));
            }
            parts.join(" ")
        };

        // ---- 提取历史产物中的文字内容作为视频文案素材 ----
        let text_refs = extract_text_content_from_artifacts(&ctx.prior_artifacts);
        let reference_text = if text_refs.is_empty() {
            String::new()
        } else {
            let refs = text_refs
                .iter()
                .map(|(title, content)| format!("【{}】\n{}", title, content))
                .collect::<Vec<_>>()
                .join("\n\n");
            format!("\n参考文档内容：\n{}\n请基于参考文档中的核心信息设计视频文案。", refs)
        };

        let user_prompt = format!(
            "需求：{topic}\n期望宽高比：{requested_aspect_ratio}\n期望时长：{requested_seconds} 秒\n生成模式：{generation_mode}\n参考图片数量：{}\n音频参考数量：{}\n视频参考数量：{}\n参考素材约束：{reference_guidance}{reference_text}\n请输出一套可直接用于 Agnes Video V2.5 的高质量视频生成方案。",
            image_inputs.len(), audio_inputs.len(), video_refs.len()
        );

        let planner = LlmClient::for_user(&ctx.user_id, ctx.preferred_model.as_deref()).await;
        let mut plan = match planner
            .chat(
                &[
                    ChatMessage {
                        role: "system".into(),
                        content: system_prompt.into(),
                        tool_calls: None,
                        tool_call_id: None,
                    },
                    ChatMessage {
                        role: "user".into(),
                        content: user_prompt,
                        tool_calls: None,
                        tool_call_id: None,
                    },
                ],
                None,
            )
            .await
        {
            Ok(response) => {
                let plan_content = response
                    .choices
                    .first()
                    .and_then(|choice| choice.message.content.as_deref())
                    .unwrap_or("");
                match parse_video_plan(plan_content) {
                    Ok(plan) => plan,
                    Err(err) => {
                        tracing::warn!("视频方案解析失败，使用默认方案兜底: {err}");
                        default_video_plan(topic, requested_aspect_ratio, requested_seconds, generation_mode)
                    }
                }
            }
            Err(err) => {
                tracing::warn!("视频方案规划失败，使用默认方案兜底: {err}");
                default_video_plan(topic, requested_aspect_ratio, requested_seconds, generation_mode)
            }
        };

        // 强制使用系统参数（防止 LLM 输出非法值）
        plan.aspect_ratio = requested_aspect_ratio.to_string();
        plan.seconds = normalize_seconds(plan.seconds);
        plan.mode = generation_mode.to_string();
        if !negative_prompt_input.is_empty() {
            plan.negative_prompt = negative_prompt_input.to_string();
        }

        let (size_label, width, height) = infer_size_and_dimensions(&plan.aspect_ratio);
        // 本地兜底用的帧参数（V2.5 不再使用 num_frames/frame_rate，但本地合成仍需）
        let frame_rate = 24u32;
        let num_frames = (plan.seconds as u32) * frame_rate;

        // ---- Agnes 凭证 ----
        let credentials = match resolve_video_credentials(&ctx.user_id) {
            Ok(credentials) => credentials,
            Err(err) => {
                return local_video_artifact(
                    ctx, topic, &plan, width, height, frame_rate, num_frames,
                    generation_mode, image_inputs.len(),
                    format!("Agnes 凭证不可用：{err}"),
                ).await;
            }
        };
        let video_model = agnes_video_model();
        let client = match http_client(Duration::from_secs(90)) {
            Ok(client) => client,
            Err(err) => {
                return local_video_artifact(
                    ctx, topic, &plan, width, height, frame_rate, num_frames,
                    generation_mode, image_inputs.len(),
                    format!("初始化 Agnes 客户端失败：{err}"),
                ).await;
            }
        };

        // ---- 构建 V2.5 请求体 ----
        ctx.send(
            "state_update",
            json!({
                "phase": "running",
                "step": "提交视频任务",
                "detail": format!("正在提交 Agnes V2.5 视频任务（{} / {}s / {}）...", plan.aspect_ratio, plan.seconds, plan.mode),
                "at": chrono::Utc::now().to_rfc3339(),
            }),
        );

        // Agnes V2.5 API 实际规则（通过 API 实测确认，与文档有差异）：
        // - text/reference 模式：mode 必须省略（传了会 400）
        // - keyframe 模式：mode 设为 "keyframes"（复数，API valid_modes 确认）
        let mut request_body = json!({
            "model": video_model.as_str(),
            "prompt": plan.prompt.clone(),
            "seconds": plan.seconds.to_string(),
            "size": size_label,
            "aspect_ratio": plan.aspect_ratio.clone(),
        });

        // 仅 keyframe 模式传 mode="keyframes"
        if generation_mode == "keyframe" {
            request_body["mode"] = json!("keyframes");
        }

        // 负面提示词（API 文档未列出该字段，暂注释以避免 400）
        // if !plan.negative_prompt.is_empty() {
        //     request_body["negative_prompt"] = json!(plan.negative_prompt);
        // }

        // 随机种子
        if let Some(seed) = seed_input {
            request_body["seed"] = json!(seed);
        }

        // ---- 模式专用参数 ----
        match generation_mode {
            "keyframe" => {
                // V2.5: first_frame / last_frame
                if image_inputs.len() >= 1 {
                    request_body["first_frame"] = json!(image_inputs[0]);
                }
                if image_inputs.len() >= 2 {
                    request_body["last_frame"] = json!(image_inputs[1]);
                }
                // V2.5: audios[] 也可以在 keyframe 模式下使用
                if !audio_inputs.is_empty() {
                    request_body["audios"] = json!(audio_inputs);
                }
            }
            "reference" => {
                // V2.5: images[] / audios[] / videos[]
                if !image_inputs.is_empty() {
                    request_body["images"] = json!(image_inputs);
                }
                if !audio_inputs.is_empty() {
                    request_body["audios"] = json!(audio_inputs);
                }
                if !video_refs.is_empty() {
                    let video_arr: Vec<serde_json::Value> = video_refs.iter().map(|v| v.to_json()).collect();
                    request_body["videos"] = json!(video_arr);
                }
                // 提示词中应该已包含 <Picture N> / <Audio N> / <Video N> 占位符
            }
            _ => {
                // text 模式也可以使用 audios[] 和 videos[]
                if !audio_inputs.is_empty() {
                    request_body["audios"] = json!(audio_inputs);
                }
                if !video_refs.is_empty() {
                    let video_arr: Vec<serde_json::Value> = video_refs.iter().map(|v| v.to_json()).collect();
                    request_body["videos"] = json!(video_arr);
                }
            }
        }

        // ---- 提交创建任务 ----
        let create_url = credentials.endpoint("/v1/videos");
        let create_response = match post_json::<CreateVideoResponse>(
            &client,
            &create_url,
            &credentials,
            &request_body,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => {
                return local_video_artifact(
                    ctx, topic, &plan, width, height, frame_rate, num_frames,
                    generation_mode, image_inputs.len(),
                    format!("Agnes V2.5 视频任务创建失败：{err}"),
                ).await;
            }
        };

        // V2.5: 使用 id 作为查询标识
        let video_task_id = create_response.id.clone();
        let poll_url = credentials.endpoint(&format!("/v1/videos/{}", urlencoding::encode(&video_task_id)));
        let deadline = Instant::now() + Duration::from_secs(480);
        let mut latest_progress = create_response.progress.unwrap_or(0);

        // ---- 轮询任务状态 ----
        loop {
            if Instant::now() >= deadline {
                return local_video_artifact(
                    ctx, topic, &plan, width, height, frame_rate, num_frames,
                    generation_mode, image_inputs.len(),
                    format!("视频生成超时：任务已创建（task_id: {video_task_id}），请稍后重试"),
                ).await;
            }

            let status_response = match get_json::<QueryVideoResponse>(
                &client,
                &poll_url,
                &credentials,
            )
            .await
            {
                Ok(response) => response,
                Err(err) => {
                    return local_video_artifact(
                        ctx, topic, &plan, width, height, frame_rate, num_frames,
                        generation_mode, image_inputs.len(),
                        format!("获取 Agnes V2.5 视频结果失败：{err}"),
                    ).await;
                }
            };

            latest_progress = status_response.progress.unwrap_or(latest_progress);
            ctx.send(
                "state_update",
                json!({
                    "phase": "running",
                    "step": "轮询视频结果",
                    "detail": format!("视频状态：{}（{}%）", status_response.status, latest_progress),
                    "at": chrono::Utc::now().to_rfc3339(),
                }),
            );

            match status_response.status.as_str() {
                "completed" => {
                    // API 实测：url 可能在顶层 url 字段，也可能在 metadata.url 中
                    let video_url = status_response
                        .url
                        .clone()
                        .filter(|url| !url.trim().is_empty())
                        .or_else(|| {
                            status_response
                                .metadata
                                .as_ref()
                                .and_then(|m| m.url.clone())
                                .filter(|url| !url.trim().is_empty())
                        })
                        .ok_or_else(|| "视频任务已完成，但没有拿到最终视频链接".to_string());
                    let video_url = match video_url {
                        Ok(url) => url,
                        Err(err) => {
                            return local_video_artifact(
                                ctx, topic, &plan, width, height, frame_rate, num_frames,
                                generation_mode, image_inputs.len(),
                                err,
                            ).await;
                        }
                    };
                    return ToolResult::ok(
                        format!("已为《{topic}》生成视频结果"),
                        vec![ToolArtifact {
                            kind: "video".into(),
                            title: plan.title.clone(),
                            content: json!({
                                "type": "generated_video",
                                "title": plan.title.clone(),
                                "description": plan.description.clone(),
                                "prompt": plan.prompt.clone(),
                                "negative_prompt": plan.negative_prompt.clone(),
                                "video_url": video_url,
                                "task_id": video_task_id,
                                "video_id": status_response.id,
                                "status": status_response.status,
                                "progress": status_response.progress.unwrap_or(100),
                                "seconds": status_response.seconds,
                                "size": status_response.size,
                                "aspect_ratio": plan.aspect_ratio.clone(),
                                "duration": format!("{}s", plan.seconds),
                                "generation_mode": generation_mode,
                                "reference_image_count": image_inputs.len(),
                                "frame_rate": frame_rate,
                                "num_frames": num_frames,
                                "provider": "agnes",
                                "model": status_response.model.unwrap_or_else(|| video_model.clone()),
                                "api_version": "v2.5",
                            }),
                        }],
                    );
                }
                "failed" | "error" | "cancelled" => {
                    let detail = status_response
                        .error
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "未知错误".to_string());
                    return local_video_artifact(
                        ctx, topic, &plan, width, height, frame_rate, num_frames,
                        generation_mode, image_inputs.len(),
                        format!("Agnes V2.5 视频生成失败：{detail}"),
                    ).await;
                }
                _ => sleep(Duration::from_secs(5)).await,
            }
        }
    }
}
