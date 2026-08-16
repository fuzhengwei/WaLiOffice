use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

use crate::agent::tool::{OfficeTool, ToolArtifact, ToolContext, ToolResult};
use crate::llm::LlmClient;
use crate::models::{ChatAttachment, ChatMessage};

use super::agnes_media::{agnes_image_model, http_client, post_json_url, resolve_image_credentials, AgnesCredentials};

pub struct ImagePromptTool;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ImagePromptPlan {
    title: String,
    description: String,
    prompts: Vec<ImageVariant>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ImageVariant {
    style: String,
    prompt: String,
}

#[derive(Debug, Deserialize)]
struct AgnesImageResponse {
    data: Vec<AgnesImageData>,
}

#[derive(Debug, Deserialize)]
struct AgnesImageData {
    url: Option<String>,
    b64_json: Option<String>,
    revised_prompt: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct ImageOutputSpec {
    size: &'static str,
    ratio: &'static str,
}

fn collect_image_inputs(ctx: &ToolContext, input: &serde_json::Value) -> Vec<String> {
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

fn infer_image_output_spec(topic: &str) -> ImageOutputSpec {
    let lower = topic.to_lowercase();
    if ["海报", "封面", "竖版", "手机", "poster", "小红书", "短视频"]
        .iter()
        .any(|keyword| lower.contains(keyword))
    {
        ImageOutputSpec {
            size: "1K",
            ratio: "2:3",
        }
    } else if ["头像", "logo", "方图", "icon", "图标", "社媒配图"]
        .iter()
        .any(|keyword| lower.contains(keyword))
    {
        ImageOutputSpec {
            size: "1K",
            ratio: "1:1",
        }
    } else {
        ImageOutputSpec {
            size: "1K",
            ratio: "3:2",
        }
    }
}

fn image_src_from_response(image: &AgnesImageData) -> Option<String> {
    image
        .url
        .as_deref()
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            image
                .b64_json
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| format!("data:image/png;base64,{value}"))
        })
}

async fn generate_agnes_image(
    client: &reqwest::Client,
    endpoint: &str,
    credentials: &AgnesCredentials,
    body: &serde_json::Value,
) -> Result<AgnesImageResponse, String> {
    let mut latest_error = String::new();
    for attempt in 1..=2 {
        match post_json_url::<AgnesImageResponse>(client, endpoint, credentials, body).await {
            Ok(response) => return Ok(response),
            Err(err) => {
                latest_error = err.to_string();
                if attempt == 1 {
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }
    Err(latest_error)
}

fn infer_scene_guide(topic: &str) -> &'static str {
    let lower = topic.to_lowercase();
    if ["产品", "发布", "官网", "功能", "品牌", "营销"]
        .iter()
        .any(|keyword| lower.contains(keyword))
    {
        "偏商业视觉表达，强调主体、场景、品牌调性、构图层次与高信息密度。"
    } else if ["技术", "架构", "ai", "agent", "智能体", "数据", "系统"]
        .iter()
        .any(|keyword| lower.contains(keyword))
    {
        "偏科技视觉表达，强调系统感、未来感、数据流、光线层次和精确语义对齐。"
    } else if ["活动", "运营", "增长", "拉新", "转化", "campaign"]
        .iter()
        .any(|keyword| lower.contains(keyword))
    {
        "偏传播海报与活动主视觉，强调视觉冲击、情绪氛围、转化感和层次清晰。"
    } else {
        "默认按可直接商用的高质量视觉方案处理，强调主体、背景、次要细节、光照和构图约束。"
    }
}

fn parse_prompt_plan(content: &str) -> Result<ImagePromptPlan, String> {
    let value = LlmClient::extract_json(content).map_err(|err| format!("提示词解析失败: {err}"))?;
    serde_json::from_value::<ImagePromptPlan>(value)
        .map_err(|err| format!("提示词结构不正确: {err}"))
}

#[async_trait]
impl OfficeTool for ImagePromptTool {
    fn name(&self) -> &str {
        "image_prompt"
    }

    fn description(&self) -> &str {
        "生成图片结果：基于 Agnes Image 2.1 Flash 生成高质量图片，并返回可直接预览的图片链接。"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "topic": { "type": "string", "description": "图片需求描述，例如海报、主视觉、封面、插画、品牌配图；如基于图片修改，请说明修改目标和保留元素" },
                "styles": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "期望风格（可选），如写实、插画、3D、极简商务、赛博科技"
                },
                "mode": {
                    "type": "string",
                    "description": "生成模式：text_to_image 或 image_to_image。用户上传图片并要求改图/换风格/基于参考图生成时使用 image_to_image"
                },
                "image_url": { "type": "string", "description": "图生图参考图片 URL 或 data URL，可选；不传时自动使用本轮上传图片" },
                "image_urls": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "多张图生图参考图片 URL 或 data URL，可选"
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
        let styles = input
            .get("styles")
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

        if topic.is_empty() {
            return ToolResult::err("topic 不能为空");
        }

        let scene_guide = infer_scene_guide(topic);
        let output_spec = infer_image_output_spec(topic);
        let image_inputs = collect_image_inputs(ctx, &input);
        let wants_image_to_image = input
            .get("mode")
            .and_then(|v| v.as_str())
            .map(|mode| {
                mode.eq_ignore_ascii_case("image_to_image") || mode.eq_ignore_ascii_case("img2img")
            })
            .unwrap_or(false)
            || !image_inputs.is_empty();

        ctx.send(
            "state_update",
            json!({
                "phase": "running",
                "step": "规划出图方案",
                "detail": format!("正在为《{topic}》生成高质量出图提示词..."),
                "at": chrono::Utc::now().to_rfc3339(),
            }),
        );

        let system_prompt = r#"你是资深视觉创意总监兼 Agnes 出图提示词专家。只输出严格 JSON，不要 markdown。
返回格式：
{
  "title": "图片标题",
  "description": "整体设计说明，说明这些风格分别适合什么业务场景",
  "prompts": [
    {
      "style": "风格名",
      "prompt": "一条可直接用于 Agnes Image 2.1 Flash 的高质量英文提示词"
    }
  ]
}
要求：
- 默认输出 3 种风格
- 提示词必须清晰包含：主要主体、背景环境、重要次要细节、风格与光照、构图约束
- 文生图遵循：[主体] + [场景/环境] + [风格] + [光照] + [构图] + [质量要求]
- 图生图遵循：[修改要求] + [新风格/新场景] + [添加/移除的元素] + [需要保留的元素]
- 图生图必须明确 preserving the original composition / main subject identity / important layout，除非用户要求大幅重绘
- 提示词要偏成品图，而不是抽象概念
- 商业场景要突出可用于官网首屏、活动海报、发布会 KV、培训封面、社媒配图等真实用途
- 文案使用英文为主，必要时可夹带品牌名或专有名词
- 不要输出多余字段"#;

        let reference_guidance = if wants_image_to_image {
            "参考图约束：必须以用户上传的参考图为首要视觉约束，保持主体身份、脸部特征、年龄感、发型、身体比例、原始构图和关键背景稳定；只修改用户要求的内容。不要换脸、不要新增人物、不要生成乱码文字。\n"
        } else {
            ""
        };

        let user_prompt = if styles.is_empty() {
            format!(
                "需求：{topic}\n场景指导：{scene_guide}\n输出尺寸建议：{}，宽高比 {}\n生成模式：{}\n{reference_guidance}请给出 3 套差异明确且可直接出图的 Agnes 提示词。",
                output_spec.size,
                output_spec.ratio,
                if wants_image_to_image { "图生图，基于用户参考图修改或再创作" } else { "文生图" }
            )
        } else {
            format!(
                "需求：{topic}\n场景指导：{scene_guide}\n输出尺寸建议：{}，宽高比 {}\n生成模式：{}\n{reference_guidance}指定风格：{}\n请优先按这些风格输出 3 套可直接出图的 Agnes 提示词。",
                output_spec.size,
                output_spec.ratio,
                if wants_image_to_image { "图生图，基于用户参考图修改或再创作" } else { "文生图" },
                styles.join(" / ")
            )
        };

        let planner = LlmClient::for_user(&ctx.user_id, ctx.preferred_model.as_deref());
        let plan_response = match planner
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
            Ok(response) => response,
            Err(err) => return ToolResult::err(format!("图片方案规划失败: {err}")),
        };

        let plan_content = plan_response
            .choices
            .first()
            .and_then(|choice| choice.message.content.as_deref())
            .unwrap_or("");
        let plan = match parse_prompt_plan(plan_content) {
            Ok(plan) => plan,
            Err(err) => return ToolResult::err(err),
        };

        let credentials = match resolve_image_credentials(&ctx.user_id) {
            Ok(credentials) => credentials,
            Err(err) => return ToolResult::err(err.to_string()),
        };
        let image_model = agnes_image_model();
        let endpoint = credentials.endpoint("/v1/images/generations");
        let client = match http_client(Duration::from_secs(240)) {
            Ok(client) => client,
            Err(err) => return ToolResult::err(format!("初始化 Agnes 客户端失败: {err}")),
        };

        let variants = plan.prompts.into_iter().take(3).collect::<Vec<_>>();
        if variants.is_empty() {
            return ToolResult::err("没有生成可用的图片提示词");
        }

        let mut generated_images = Vec::new();
        let mut generated_variants = Vec::new();
        let mut failed_variants = Vec::new();

        for (index, variant) in variants.iter().enumerate() {
            ctx.send(
                "state_update",
                json!({
                    "phase": "running",
                    "step": "Agnes 图像生成",
                    "detail": format!("正在生成第 {} / {} 张图片（{}）...", index + 1, variants.len(), variant.style),
                    "at": chrono::Utc::now().to_rfc3339(),
                }),
            );

            let request_body = if wants_image_to_image {
                json!({
                    "model": image_model.as_str(),
                    "prompt": variant.prompt,
                    "size": output_spec.size,
                    "ratio": output_spec.ratio,
                    "extra_body": {
                        "response_format": "url",
                        "image": image_inputs.clone()
                    }
                })
            } else {
                json!({
                    "model": image_model.as_str(),
                    "prompt": variant.prompt,
                    "size": output_spec.size,
                    "ratio": output_spec.ratio,
                    "extra_body": {
                        "response_format": "url"
                    }
                })
            };

            let response =
                match generate_agnes_image(&client, &endpoint, &credentials, &request_body)
                    .await
                {
                    Ok(response) => response,
                    Err(err) => {
                        let detail = format!(
                            "第 {} 张图片生成失败（{}）：{}",
                            index + 1,
                            variant.style,
                            err
                        );
                        failed_variants.push(detail.clone());
                        ctx.send(
                            "state_update",
                            json!({
                                "phase": "running",
                                "step": "Agnes 图像生成",
                                "detail": detail,
                                "at": chrono::Utc::now().to_rfc3339(),
                            }),
                        );
                        continue;
                    }
                };

            let image = response.data.into_iter().next();
            let image_src = image.as_ref().and_then(image_src_from_response);
            if let Some(src) = image_src {
                generated_images.push(src.clone());
                generated_variants.push(json!({
                    "style": variant.style,
                    "prompt": variant.prompt,
                    "url": src,
                    "revised_prompt": image.and_then(|item| item.revised_prompt),
                }));
            } else {
                failed_variants.push(format!(
                    "第 {} 张图片响应成功，但没有返回 url 或 b64_json",
                    index + 1
                ));
            }
        }

        if generated_images.is_empty() {
            ctx.send(
                "state_update",
                json!({
                    "phase": "running",
                    "step": "Agnes 图像生成",
                    "detail": "常规提示词未拿到图片，正在使用简化提示词兜底重试...",
                    "at": chrono::Utc::now().to_rfc3339(),
                }),
            );
            let fallback_prompt = format!(
                "Create a clear, high quality image for this user request: {topic}. Use a simple strong composition, recognizable main subject, polished lighting, and appealing visual details."
            );
            let fallback_body = if wants_image_to_image {
                json!({
                    "model": image_model.as_str(),
                    "prompt": fallback_prompt,
                    "size": output_spec.size,
                    "ratio": output_spec.ratio,
                    "extra_body": {
                        "response_format": "url",
                        "image": image_inputs.clone()
                    }
                })
            } else {
                json!({
                    "model": image_model.as_str(),
                    "prompt": fallback_prompt,
                    "size": "1K",
                    "ratio": "1:1",
                    "extra_body": {
                        "response_format": "url"
                    }
                })
            };
            match generate_agnes_image(&client, &endpoint, &credentials, &fallback_body)
                .await
            {
                Ok(response) => {
                    if let Some(image) = response.data.into_iter().next() {
                        if let Some(src) = image_src_from_response(&image) {
                            generated_images.push(src.clone());
                            generated_variants.push(json!({
                                "style": "兜底生成",
                                "prompt": fallback_prompt,
                                "url": src,
                                "revised_prompt": image.revised_prompt,
                            }));
                        }
                    }
                }
                Err(err) => failed_variants.push(format!("兜底图片生成失败：{err}")),
            }
        }

        if generated_images.is_empty() {
            let detail = if failed_variants.is_empty() {
                "Agnes 已返回响应，但没有拿到可预览的图片链接".to_string()
            } else {
                format!("Agnes 图片生成失败：{}", failed_variants.join("；"))
            };
            return ToolResult::err(detail);
        }

        let observation = if failed_variants.is_empty() {
            format!("已为《{topic}》生成 {} 张图片结果", generated_images.len())
        } else {
            format!(
                "已为《{topic}》生成 {} 张图片结果；部分候选失败：{}",
                generated_images.len(),
                failed_variants.join("；")
            )
        };

        ToolResult::ok(
            observation,
            vec![ToolArtifact {
                kind: "image".into(),
                title: plan.title.clone(),
                content: json!({
                    "type": "generated_image",
                    "title": plan.title,
                    "description": plan.description,
                    "prompt": generated_variants
                        .first()
                        .and_then(|item| item.get("prompt"))
                        .and_then(|value| value.as_str())
                        .unwrap_or(""),
                    "image_size": output_spec.size,
                    "image_ratio": output_spec.ratio,
                    "generation_mode": if wants_image_to_image { "image_to_image" } else { "text_to_image" },
                    "reference_image_count": image_inputs.len(),
                    "images": generated_images,
                    "variants": generated_variants,
                    "provider": "agnes",
                    "model": image_model,
                }),
            }],
        )
    }
}
