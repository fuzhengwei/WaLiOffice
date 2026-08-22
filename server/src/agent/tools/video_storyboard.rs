use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::tool::{OfficeTool, ToolArtifact, ToolContext, ToolResult};
use crate::llm::LlmClient;
use crate::models::ChatMessage;

use super::video_generate::{
    attachment_to_data_url, collect_video_audios, collect_video_refs,
    extract_image_urls_from_artifacts, extract_text_content_from_artifacts,
    extract_video_urls_from_artifacts, normalize_aspect_ratio, normalize_seconds,
};

pub struct VideoStoryboardTool;

/// 分镜方案
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoryboardShot {
    /// 镜头序号（从 1 开始）
    index: u32,
    /// 镜头标题
    title: String,
    /// 镜头描述（中文）
    description: String,
    /// Agnes Video V2.5 英文提示词
    prompt: String,
    /// 生成模式：text / keyframe / reference
    mode: String,
    /// 时长（4-12 秒）
    seconds: u8,
    /// 首帧图片 URL（keyframe 模式）
    first_frame: Option<String>,
    /// 尾帧图片 URL（keyframe 模式）
    last_frame: Option<String>,
    /// 参考图片 URL 列表（reference 模式）
    reference_images: Vec<String>,
    /// 音频参考 URL 列表
    #[serde(default)]
    audio_urls: Vec<String>,
    /// 转场建议（与下一镜头的衔接方式）
    transition: Option<String>,
}

/// 分镜方案 LLM 输出
#[derive(Debug, Serialize, Deserialize)]
struct StoryboardPlan {
    title: String,
    description: String,
    total_shots: u32,
    total_seconds: u32,
    aspect_ratio: String,
    shots: Vec<StoryboardShot>,
}

#[async_trait]
impl OfficeTool for VideoStoryboardTool {
    fn name(&self) -> &str {
        "video_storyboard"
    }

    fn description(&self) -> &str {
        "视频分镜规划：输入视频主题和可选参考图片，AI 自动拆分为多个镜头，每个镜头包含独立的提示词、生成模式、时长和首尾帧分配。适用于复杂视频需求（如多场景宣传片、故事短片）。规划完成后，可调用 video_batch_generate 一键批量生成所有镜头。"
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn produces_artifact(&self) -> bool {
        true
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "topic": {
                    "type": "string",
                    "description": "视频需求描述，例如'产品发布会宣传片，包含开场、产品展示、功能演示、结尾号召'"
                },
                "aspect_ratio": {
                    "type": "string",
                    "description": "宽高比：16:9（默认）、9:16、1:1、4:3、3:4、21:9"
                },
                "max_shots": {
                    "type": "integer",
                    "description": "最大镜头数，默认 3，建议 2-5 个镜头"
                },
                "seconds_per_shot": {
                    "type": "integer",
                    "description": "每镜头时长（4-12 秒），默认 5 秒"
                },
                "image_urls": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "可选参考图片 URL 列表，AI 会自动分配到各镜头"
                },
                "audio_urls": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "可选音频参考 URL 列表，AI 会分配到相关镜头，用 <Audio N> 占位符引用"
                },
                "video_urls": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "可选视频参考 URL 列表，AI 会分配到相关镜头，用 <Video N> 占位符引用"
                }
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
        let aspect_ratio = normalize_aspect_ratio(
            input
                .get("aspect_ratio")
                .and_then(|v| v.as_str())
                .unwrap_or("16:9"),
        );

        let max_shots = input
            .get("max_shots")
            .and_then(|v| v.as_u64())
            .map(|n| n.clamp(2, 6) as u32)
            .unwrap_or(3);

        let seconds_per_shot = normalize_seconds(
            input
                .get("seconds_per_shot")
                .and_then(|v| v.as_u64())
                .map(|n| n as u8)
                .unwrap_or(5),
        );

        // ---- 收集图片 ----
        let mut images: Vec<String> = input
            .get("image_urls")
            .and_then(|v| v.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if images.is_empty() {
            images.extend(ctx.attachments.iter().filter_map(attachment_to_data_url));
        }

        if images.is_empty() {
            images.extend(extract_image_urls_from_artifacts(&ctx.prior_artifacts));
        }

        let image_count = images.len();

        // ---- 收集音频和视频参考 ----
        let audio_urls = collect_video_audios(&input, &ctx.prior_artifacts);
        let video_refs = collect_video_refs(&input, &ctx.prior_artifacts);
        let video_ref_urls: Vec<String> = video_refs.iter().map(|v| v.url.clone()).collect();

        // 如果输入未显式提供视频参考，从历史产物中提取
        let video_ref_urls = if video_ref_urls.is_empty() {
            extract_video_urls_from_artifacts(&ctx.prior_artifacts)
        } else {
            video_ref_urls
        };

        // ---- LLM 规划分镜 ----
        ctx.send(
            "state_update",
            json!({
                "phase": "running",
                "step": "分镜规划",
                "detail": format!("正在为《{topic}》规划 {max_shots} 个镜头的分镜方案..."),
                "image_count": image_count,
                "at": chrono::Utc::now().to_rfc3339(),
            }),
        );

        let system_prompt = r#"你是资深导演兼视频分镜规划师。根据用户需求，将复杂视频拆分为多个镜头，每个镜头可独立生成后拼接。

只输出严格 JSON，不要 markdown。返回格式：
{
  "title": "视频总标题",
  "description": "整体创意说明",
  "total_shots": 3,
  "total_seconds": 15,
  "aspect_ratio": "16:9",
  "shots": [
    {
      "index": 1,
      "title": "镜头标题",
      "description": "镜头中文描述",
      "prompt": "Agnes Video V2.5 英文提示词，遵循 [主体与场景] + [动作与变化] + [镜头语言] + [视觉风格]",
      "mode": "text",
      "seconds": 5,
      "first_frame": null,
      "last_frame": null,
      "reference_images": [],
      "audio_urls": [],
      "transition": "与下一镜头的转场建议"
    }
  ]
}

分镜原则：
1. 每镜头 4-12 秒，总时长建议 10-30 秒
2. 镜头之间要有明确的叙事递进（开场→展开→高潮→结尾）
3. 根据图片数量自动分配模式：
   - 无图 → text 模式
   - 有图 → 将图片分配到相关镜头，1-2张用 keyframe 模式（设置 first_frame/last_frame）
   - 3+张用 reference 模式（设置 reference_images，prompt 中用 <Picture N> 占位符）
4. transition 描述与下一镜头的视觉衔接方式（如"淡入""硬切""缩放过渡"）
5. 最后一个镜头的 transition 设为 null
6. 每个镜头的 prompt 要独立完整，但可以指定与上一镜头的视觉连续性要求
7. mode 只能是 text / keyframe / reference
8. seconds 只能是 4-12 的整数

音频参考规则：
- 如果提供了音频 URL，分配到相关镜头的 audio_urls 字段
- 在 prompt 中用 <Audio N> 占位符引用，例如"根据 <Audio 1> 的节奏设计镜头切换"
- 适合 MV 风格视频、配乐驱动节奏的广告

视频参考规则：
- 如果提供了视频 URL，分配到相关镜头的 reference_images 或在 prompt 中说明
- 在 prompt 中用 <Video N> 占位符引用，例如"延续 <Video 1> 的画面风格"
- 适合视频续写、风格迁移、动作模仿

链式一致性策略（重要）：
- 首镜应建立主要角色/场景的视觉形象
- 后续镜头应在 prompt 中描述与首镜的视觉连续性（如"与镜头1相同的角色造型""延续镜头1的场景色调"）
- 相邻镜头之间应有明确的视觉衔接（色调一致、角色一致、场景过渡自然）
- transition 字段应具体描述视觉过渡方式，不仅笼统说"淡入""硬切""#;

        let image_guidance = if image_count == 0 {
            "无参考图片，所有镜头使用 text 模式。".to_string()
        } else {
            format!(
                "有 {} 张参考图片，请合理分配到各镜头：\n{}\n分配规则：1-2张→keyframe模式(first_frame/last_frame)，3+张→reference模式(reference_images + <Picture N>占位符)。",
                image_count,
                images
                    .iter()
                    .enumerate()
                    .map(|(i, url)| format!("  图片{}: {}", i + 1, if url.len() > 80 { format!("{}...", &url[..80]) } else { url.clone() }))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };

        let audio_guidance = if audio_urls.is_empty() {
            "无音频参考。".to_string()
        } else {
            format!(
                "有 {} 个音频参考，请分配到相关镜头的 audio_urls 字段，并在 prompt 中用 <Audio N> 占位符引用：\n{}",
                audio_urls.len(),
                audio_urls.iter().enumerate().map(|(i, url)| format!("  音频{}: {}", i + 1, if url.len() > 80 { format!("{}...", &url[..80]) } else { url.clone() })).collect::<Vec<_>>().join("\n")
            )
        };

        let video_guidance = if video_ref_urls.is_empty() {
            "无视频参考。".to_string()
        } else {
            format!(
                "有 {} 个视频参考素材，请在 prompt 中用 <Video N> 占位符引用，用于续写或风格迁移：\n{}",
                video_ref_urls.len(),
                video_ref_urls.iter().enumerate().map(|(i, url)| format!("  视频{}: {}", i + 1, if url.len() > 80 { format!("{}...", &url[..80]) } else { url.clone() })).collect::<Vec<_>>().join("\n")
            )
        };

        // ---- 提取历史产物中的文字内容作为视频文案素材 ----
        let text_refs = extract_text_content_from_artifacts(&ctx.prior_artifacts);
        let reference_text = if text_refs.is_empty() {
            "无参考文档。".to_string()
        } else {
            text_refs
                .iter()
                .map(|(title, content)| format!("【{}】\n{}", title, content))
                .collect::<Vec<_>>()
                .join("\n\n")
        };

        let user_prompt = format!(
            "视频需求：{topic}\n宽高比：{aspect_ratio}\n最大镜头数：{max_shots}\n每镜头时长：{seconds_per_shot} 秒\n参考图片：{image_guidance}\n音频参考：{audio_guidance}\n视频参考：{video_guidance}\n\n参考文档内容：\n{reference_text}\n\n请规划完整的分镜方案。每个镜头要有独立的创意方向，组合起来构成完整叙事。如果提供了参考文档内容，请基于其中的核心信息设计镜头文案。相邻镜头要注意视觉连续性（角色一致、色调一致、场景过渡自然）。",
        );

        let planner = LlmClient::for_user(&ctx.user_id, ctx.preferred_model.as_deref()).await;
        let plan = match planner
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
                let content = response
                    .choices
                    .first()
                    .and_then(|c| c.message.content.as_deref())
                    .unwrap_or("");
                match LlmClient::extract_json(content) {
                    Ok(value) => {
                        match serde_json::from_value::<StoryboardPlan>(value) {
                            Ok(plan) => plan,
                            Err(err) => {
                                return ToolResult::err(format!("分镜方案解析失败: {err}"));
                            }
                        }
                    }
                    Err(err) => {
                        return ToolResult::err(format!("分镜方案 JSON 提取失败: {err}"));
                    }
                }
            }
            Err(err) => {
                return ToolResult::err(format!("分镜规划 LLM 调用失败: {err}"));
            }
        };

        // ---- 构建分镜 artifact ----
        let shots_json: Vec<serde_json::Value> = plan
            .shots
            .iter()
            .map(|shot| {
                json!({
                    "index": shot.index,
                    "title": shot.title,
                    "description": shot.description,
                    "prompt": shot.prompt,
                    "mode": shot.mode,
                    "seconds": shot.seconds,
                    "first_frame": shot.first_frame,
                    "last_frame": shot.last_frame,
                    "reference_images": shot.reference_images,
                    "audio_urls": shot.audio_urls,
                    "transition": shot.transition,
                })
            })
            .collect();

        let storyboard_content = json!({
            "type": "video_storyboard",
            "title": plan.title,
            "description": plan.description,
            "total_shots": plan.total_shots,
            "total_seconds": plan.total_seconds,
            "aspect_ratio": plan.aspect_ratio,
            "shots": shots_json,
            "image_count": image_count,
            "audio_count": audio_urls.len(),
            "video_ref_count": video_ref_urls.len(),
            "usage_guide": "分镜方案已生成。接下来可调用 video_batch_generate 一键批量生成所有镜头（自动启用链式一致性，上一镜头视频自动作为下一镜头参考）。也可将每个镜头的 prompt/mode/seconds/first_frame/last_frame/reference_images/audio_urls 传入 video_generate 逐个生成。",
        });

        let summary = format!(
            "已为《{}》规划 {} 个镜头的分镜方案（总时长约 {} 秒）：\n{}",
            plan.title,
            plan.total_shots,
            plan.total_seconds,
            plan.shots
                .iter()
                .map(|s| format!(
                    "  镜头{}: {}（{}模式，{}s）— {}",
                    s.index, s.title, s.mode, s.seconds, s.description
                ))
                .collect::<Vec<_>>()
                .join("\n")
        );

        ToolResult::ok(
            summary,
            vec![ToolArtifact {
                kind: "video".into(),
                title: format!("分镜方案：{}", plan.title),
                content: storyboard_content,
            }],
        )
    }
}
