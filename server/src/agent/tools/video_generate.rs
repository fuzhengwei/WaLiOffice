use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tokio::time::{sleep, Instant};

use crate::agent::tool::{OfficeTool, ToolArtifact, ToolContext, ToolResult};
use crate::llm::LlmClient;
use crate::models::{ChatAttachment, ChatMessage};

use super::agnes_media::{
    get_json, http_client, post_json, resolve_api_key, AGNES_API_BASE, AGNES_VIDEO_MODEL,
};
use super::local_video;

pub struct VideoGenerateTool;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct VideoPlan {
    title: String,
    description: String,
    prompt: String,
    negative_prompt: String,
    aspect_ratio: String,
    duration: String,
}

#[derive(Debug, Deserialize)]
struct CreateVideoResponse {
    task_id: String,
    video_id: String,
    status: String,
    progress: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct QueryVideoResponse {
    id: Option<String>,
    video_id: Option<String>,
    model: Option<String>,
    status: String,
    progress: Option<u32>,
    seconds: Option<String>,
    size: Option<String>,
    remixed_from_video_id: Option<String>,
    url: Option<String>,
    error: Option<serde_json::Value>,
}

fn parse_video_plan(content: &str) -> Result<VideoPlan, String> {
    let value =
        LlmClient::extract_json(content).map_err(|err| format!("视频提示词解析失败: {err}"))?;
    serde_json::from_value::<VideoPlan>(value).map_err(|err| format!("视频提示词结构不正确: {err}"))
}

fn normalize_aspect_ratio(input: &str) -> &'static str {
    match input.trim() {
        "9:16" => "9:16",
        "1:1" => "1:1",
        "4:3" => "4:3",
        "3:4" => "3:4",
        _ => "16:9",
    }
}

fn infer_dimensions(aspect_ratio: &str) -> (u32, u32) {
    match aspect_ratio {
        "9:16" => (768, 1152),
        "1:1" => (960, 960),
        "4:3" => (1024, 768),
        "3:4" => (768, 1024),
        _ => (1152, 768),
    }
}

fn infer_num_frames(duration: &str) -> u32 {
    match duration.trim() {
        "short" => 81,
        "long" => 241,
        "max" => 441,
        _ => 121,
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

    if images.is_empty() {
        images.extend(ctx.attachments.iter().filter_map(attachment_to_data_url));
    }

    images
}

fn attachment_to_data_url(attachment: &ChatAttachment) -> Option<String> {
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

fn normalize_mode(input: &str, image_count: usize) -> &'static str {
    match input.trim() {
        "keyframes" => "keyframes",
        "image_to_video" | "img2video" | "ti2vid" => "ti2vid",
        _ if image_count > 0 => "ti2vid",
        _ => "text_to_video",
    }
}

fn default_video_plan(topic: &str, aspect_ratio: &str, duration: &str) -> VideoPlan {
    VideoPlan {
        title: topic.chars().take(24).collect::<String>(),
        description: format!("围绕“{topic}”生成一支具有明确主体运动和镜头变化的短视频。"),
        prompt: format!(
            "Create a polished short video about: {topic}. Use a clear main subject, visible motion, cinematic camera movement, warm lighting, and an engaging commercial visual style."
        ),
        negative_prompt: "low quality, blurry, distorted, flicker, watermark, text artifacts".into(),
        aspect_ratio: aspect_ratio.into(),
        duration: duration.into(),
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
                    "duration": plan.duration,
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
        "生成视频结果：基于 Agnes Video V2.0 创建视频任务并返回可直接预览的 mp4 视频链接。"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "topic": { "type": "string", "description": "视频需求描述，例如产品短片、活动宣传片、品牌动画、功能演示" },
                "aspect_ratio": {
                    "type": "string",
                    "description": "可选宽高比：16:9、9:16、1:1、4:3、3:4"
                },
                "duration": {
                    "type": "string",
                    "description": "可选时长档位：short、standard、long、max"
                },
                "mode": {
                    "type": "string",
                    "description": "生成模式：text_to_video、image_to_video、keyframes。上传图片并要求动起来时用 image_to_video；多张关键帧过渡用 keyframes"
                },
                "image_url": { "type": "string", "description": "图生视频参考图片 URL 或 data URL，可选；不传时自动使用本轮上传图片" },
                "image_urls": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "多图视频或关键帧图片 URL / data URL 列表"
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

        let requested_aspect_ratio = normalize_aspect_ratio(
            input
                .get("aspect_ratio")
                .and_then(|v| v.as_str())
                .unwrap_or("16:9"),
        );
        let requested_duration = input
            .get("duration")
            .and_then(|v| v.as_str())
            .unwrap_or("standard")
            .trim();
        let duration = if requested_duration.eq_ignore_ascii_case("short") {
            "short"
        } else if requested_duration.eq_ignore_ascii_case("long") {
            "long"
        } else if requested_duration.eq_ignore_ascii_case("max") {
            "max"
        } else {
            "standard"
        };
        let image_inputs = collect_video_images(ctx, &input);
        let generation_mode = normalize_mode(
            input.get("mode").and_then(|v| v.as_str()).unwrap_or(""),
            image_inputs.len(),
        );

        ctx.send(
            "state_update",
            json!({
                "phase": "running",
                "step": "规划视频脚本",
                "detail": format!("正在为《{topic}》生成视频提示词与镜头描述..."),
                "at": chrono::Utc::now().to_rfc3339(),
            }),
        );

        let system_prompt = r#"你是资深导演兼 Agnes Video V2.0 提示词专家。只输出严格 JSON，不要 markdown。
返回格式：
{
  "title": "视频标题",
  "description": "视频创意说明",
  "prompt": "可直接用于 Agnes Video V2.0 的英文提示词",
  "negative_prompt": "需要避免的内容",
  "aspect_ratio": "16:9",
  "duration": "standard"
}
要求：
- 提示词遵循 [主体] + [动作] + [场景] + [镜头运动] + [光线] + [风格]
- 文生视频要描述完整动态画面；图生视频要描述哪些内容运动、哪些主体元素保持稳定
- 多图视频要描述图片之间的关系和过渡；关键帧动画要强调 smooth transition between keyframes
- 要有明确运动感，不要只写静态画面
- 偏商业场景时，优先考虑产品演示、品牌宣传、活动短片、官网视频头图、社媒短视频
- aspect_ratio 只能输出 16:9 / 9:16 / 1:1 / 4:3 / 3:4
- duration 只能输出 short / standard / long / max"#;

        let reference_guidance = if image_inputs.is_empty() {
            "无参考图，请按文生视频生成完整动态画面。"
        } else {
            "有参考图：必须以参考图为首要视觉约束，保持主体身份、脸部特征、发型、服装主体、构图和关键背景稳定，只设计自然小幅动作与镜头运动；不要换脸、不要新增人物、不要重构场景。"
        };
        let user_prompt = format!(
            "需求：{topic}\n期望宽高比：{requested_aspect_ratio}\n期望时长档位：{duration}\n生成模式：{generation_mode}\n参考图片数量：{}\n参考图约束：{reference_guidance}\n请输出一套可直接用于 Agnes Video V2.0 的高质量视频生成方案。",
            image_inputs.len()
        );

        let planner = LlmClient::for_user(&ctx.user_id, ctx.preferred_model.as_deref());
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
                        default_video_plan(topic, requested_aspect_ratio, duration)
                    }
                }
            }
            Err(err) => {
                tracing::warn!("视频方案规划失败，使用默认方案兜底: {err}");
                default_video_plan(topic, requested_aspect_ratio, duration)
            }
        };

        plan.aspect_ratio = requested_aspect_ratio.to_string();
        plan.duration = duration.to_string();
        let (width, height) = infer_dimensions(&plan.aspect_ratio);
        let num_frames = infer_num_frames(&plan.duration);
        let frame_rate = 24u32;

        let api_key = match resolve_api_key(&ctx.user_id) {
            Ok(key) => key,
            Err(err) => {
                return local_video_artifact(
                    ctx,
                    topic,
                    &plan,
                    width,
                    height,
                    frame_rate,
                    num_frames,
                    generation_mode,
                    image_inputs.len(),
                    format!("Agnes 凭证不可用：{err}"),
                )
                .await;
            }
        };
        let client = match http_client(Duration::from_secs(90)) {
            Ok(client) => client,
            Err(err) => {
                return local_video_artifact(
                    ctx,
                    topic,
                    &plan,
                    width,
                    height,
                    frame_rate,
                    num_frames,
                    generation_mode,
                    image_inputs.len(),
                    format!("初始化 Agnes 客户端失败：{err}"),
                )
                .await;
            }
        };

        ctx.send(
            "state_update",
            json!({
                "phase": "running",
                "step": "提交视频任务",
                "detail": format!("正在提交 Agnes 视频生成任务（{} / {}）...", plan.aspect_ratio, plan.duration),
                "at": chrono::Utc::now().to_rfc3339(),
            }),
        );

        let mut request_body = json!({
            "model": AGNES_VIDEO_MODEL,
            "prompt": plan.prompt.clone(),
            "negative_prompt": plan.negative_prompt.clone(),
            "width": width,
            "height": height,
            "num_frames": num_frames,
            "frame_rate": frame_rate,
        });

        if generation_mode == "keyframes" && !image_inputs.is_empty() {
            request_body["extra_body"] = json!({
                "image": image_inputs.clone(),
                "mode": "keyframes"
            });
        } else if image_inputs.len() > 1 {
            request_body["extra_body"] = json!({
                "image": image_inputs.clone()
            });
        } else if let Some(image) = image_inputs.first() {
            request_body["image"] = json!(image);
        }

        let create_response =
            match post_json::<CreateVideoResponse>(&client, "/v1/videos", &api_key, &request_body)
                .await
            {
                Ok(response) => response,
                Err(err) => {
                    return local_video_artifact(
                        ctx,
                        topic,
                        &plan,
                        width,
                        height,
                        frame_rate,
                        num_frames,
                        generation_mode,
                        image_inputs.len(),
                        format!("Agnes 视频任务创建失败：{err}"),
                    )
                    .await;
                }
            };

        let video_id = create_response.video_id.clone();
        let task_id = create_response.task_id.clone();
        let poll_url = format!(
            "{AGNES_API_BASE}/agnesapi?video_id={}&model_name={}",
            urlencoding::encode(&video_id),
            urlencoding::encode(AGNES_VIDEO_MODEL),
        );
        let deadline = Instant::now() + Duration::from_secs(480);
        let mut latest_progress = create_response.progress.unwrap_or(0);

        loop {
            if Instant::now() >= deadline {
                return local_video_artifact(
                    ctx,
                    topic,
                    &plan,
                    width,
                    height,
                    frame_rate,
                    num_frames,
                    generation_mode,
                    image_inputs.len(),
                    "视频生成超时：任务已创建（task_id: {task_id}，video_id: {video_id}），请稍后重试"
                )
                .await;
            }

            let status_response =
                match get_json::<QueryVideoResponse>(&client, &poll_url, &api_key).await {
                    Ok(response) => response,
                    Err(err) => {
                        return local_video_artifact(
                            ctx,
                            topic,
                            &plan,
                            width,
                            height,
                            frame_rate,
                            num_frames,
                            generation_mode,
                            image_inputs.len(),
                            format!("获取 Agnes 视频结果失败：{err}"),
                        )
                        .await;
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
                    let video_url = status_response
                        .url
                        .clone()
                        .or_else(|| status_response.remixed_from_video_id.clone())
                        .filter(|url| !url.trim().is_empty())
                        .ok_or_else(|| "视频任务已完成，但没有拿到最终视频链接".to_string());
                    let video_url = match video_url {
                        Ok(url) => url,
                        Err(err) => {
                            return local_video_artifact(
                                ctx,
                                topic,
                                &plan,
                                width,
                                height,
                                frame_rate,
                                num_frames,
                                generation_mode,
                                image_inputs.len(),
                                err,
                            )
                            .await;
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
                                "task_id": task_id,
                                "video_id": status_response.video_id.or(Some(video_id)),
                                "status": status_response.status,
                                "progress": status_response.progress.unwrap_or(100),
                                "seconds": status_response.seconds,
                                "size": status_response.size,
                                "aspect_ratio": plan.aspect_ratio.clone(),
                                "duration": plan.duration.clone(),
                                "generation_mode": generation_mode,
                                "reference_image_count": image_inputs.len(),
                                "frame_rate": frame_rate,
                                "num_frames": num_frames,
                                "provider": "agnes",
                                "model": status_response.model.unwrap_or_else(|| AGNES_VIDEO_MODEL.to_string()),
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
                        ctx,
                        topic,
                        &plan,
                        width,
                        height,
                        frame_rate,
                        num_frames,
                        generation_mode,
                        image_inputs.len(),
                        format!("Agnes 视频生成失败：{detail}"),
                    )
                    .await;
                }
                _ => sleep(Duration::from_secs(5)).await,
            }
        }
    }
}
