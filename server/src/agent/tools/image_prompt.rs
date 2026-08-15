use async_trait::async_trait;
use serde::Serialize;
use serde_json::json;

use crate::agent::tool::{OfficeTool, ToolArtifact, ToolContext, ToolResult};
use crate::llm::LlmClient;
use crate::models::ChatMessage;

pub struct ImagePromptTool;

#[derive(Serialize)]
struct ImagePromptOutput {
    title: String,
    description: String,
    prompts: Vec<PromptVariant>,
}

#[derive(Serialize)]
struct PromptVariant {
    style: String,
    prompt: String,
    negative_prompt: String,
}

fn infer_image_size(topic: &str) -> &'static str {
    let lower = topic.to_lowercase();
    if ["海报", "封面", "竖版", "手机", "poster"]
        .iter()
        .any(|keyword| lower.contains(keyword))
    {
        "portrait_16_9"
    } else if ["首屏", "横幅", "banner", "大屏", "官网"]
        .iter()
        .any(|keyword| lower.contains(keyword))
    {
        "landscape_16_9"
    } else {
        "landscape_4_3"
    }
}

fn infer_image_scene(topic: &str) -> &'static str {
    let lower = topic.to_lowercase();

    if ["产品", "需求", "prd", "roadmap", "版本", "迭代", "feature"]
        .iter()
        .any(|keyword| lower.contains(keyword))
    {
        "当前更像产品发布/功能表达场景，画面要突出产品价值、核心能力和使用情境。"
    } else if [
        "运营", "增长", "拉新", "留存", "转化", "活动", "campaign", "gmv",
    ]
    .iter()
    .any(|keyword| lower.contains(keyword))
    {
        "当前更像运营传播场景，画面要突出活动氛围、转化感、数据增长和品牌调性。"
    } else if [
        "销售", "客户", "商机", "渠道", "业绩", "回款", "签约", "线索",
    ]
    .iter()
    .any(|keyword| lower.contains(keyword))
    {
        "当前更像销售/商务传播场景，画面要突出信任感、专业感、合作关系和业务增长。"
    } else if [
        "技术",
        "架构",
        "系统",
        "平台",
        "接口",
        "部署",
        "微服务",
        "数据库",
        "agent",
        "ai",
    ]
    .iter()
    .any(|keyword| lower.contains(keyword))
    {
        "当前更像科技产品/技术品牌场景，画面要突出系统感、数据流、智能协同和未来感。"
    } else if [
        "培训", "课程", "学习", "上手", "入门", "手册", "宣导", "workshop",
    ]
    .iter()
    .any(|keyword| lower.contains(keyword))
    {
        "当前更像培训宣传/课程封面场景，画面要突出学习氛围、人物参与感和知识传递。"
    } else if ["项目", "排期", "里程碑", "实施", "交付", "风险", "计划"]
        .iter()
        .any(|keyword| lower.contains(keyword))
    {
        "当前更像项目交付/实施汇报场景，画面要突出协作、进展、目标达成和执行感。"
    } else {
        "默认按商业视觉场景处理，兼顾品牌感、清晰主体和真实使用场景。"
    }
}

#[async_trait]
impl OfficeTool for ImagePromptTool {
    fn name(&self) -> &str {
        "image_prompt"
    }

    fn description(&self) -> &str {
        "生成图片结果：根据用户需求生成多种风格的 AI 绘画提示词，并给出可直接预览的图片结果。"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "topic": { "type": "string", "description": "图片需求描述" },
                "styles": { "type": "array", "items": {"type": "string"}, "description": "期望的风格（可选），如 photorealistic, illustration, 3d-render" }
            },
            "required": ["topic"]
        })
    }

    async fn call(&self, input: serde_json::Value, ctx: &ToolContext) -> ToolResult {
        let topic = input.get("topic").and_then(|v| v.as_str()).unwrap_or("");
        let scene_guide = infer_image_scene(topic);
        if topic.is_empty() {
            return ToolResult::err("topic 不能为空");
        }

        ctx.send(
            "state_update",
            json!({
                "phase": "running",
                "step": "生成图片提示词",
                "detail": format!("正在为《{topic}》生成提示词..."),
                "at": chrono::Utc::now().to_rfc3339(),
            }),
        );

        let system_prompt = r#"你是 AI 绘画提示词专家。只输出严格 JSON，不要 markdown。
返回格式：
{
  "title": "图片标题",
  "description": "整体设计说明",
  "prompts": [
    {
      "style": "风格名（如写实/插画/3D）",
      "prompt": "正向提示词（英文为主，含主体、构图、光线、色调等细节）",
      "negative_prompt": "负向提示词（要避免的元素）"
    }
  ]
}
要求：
- 生成 3 种不同风格的提示词，每种 prompt 至少 60 词，细节丰富
- prompt 以英文为主，但可保留必要中文品牌词或专有名词
- 必须写清主体、场景、镜头/构图、光线、材质、色彩、氛围、画面重点
- 如果需求偏商业用途，优先考虑官网首屏、产品海报、发布会 KV、演示文稿封面这类真实场景
- negative_prompt 要有针对性，避免模糊、低清晰度、畸形、杂乱背景、水印、文字错误等常见问题
- description 要说明 3 种风格各自适合什么使用场景"#;

        let user_prompt = format!("请为以下需求生成可直接用于出图的高质量 AI 绘画提示词。\n场景偏好：{scene_guide}\n需求：{topic}");

        let client = LlmClient::for_user(&ctx.user_id, ctx.preferred_model.as_deref());
        let messages = vec![
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
        ];

        let resp = match client.chat(&messages, None).await {
            Ok(r) => r,
            Err(e) => return ToolResult::err(format!("图片提示词生成失败: {e}")),
        };

        let content = resp
            .choices
            .first()
            .and_then(|c| c.message.content.as_deref())
            .unwrap_or("");
        let output = match LlmClient::extract_json(content) {
            Ok(v) => v,
            Err(e) => return ToolResult::err(format!("提示词解析失败: {e}")),
        };

        let prompt_variants = output
            .get("prompts")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let image_size = infer_image_size(topic);
        let images = prompt_variants
            .iter()
            .filter_map(|item| item.get("prompt").and_then(|value| value.as_str()))
            .take(3)
            .map(|prompt| {
                format!(
                    "https://coresg-normal.trae.ai/api/ide/v1/text_to_image?prompt={}&image_size={}",
                    urlencoding::encode(prompt),
                    image_size
                )
            })
            .collect::<Vec<_>>();
        let style_count = prompt_variants.len();

        ToolResult::ok(
            format!("已生成《{topic}》图片结果，共 {style_count} 种风格"),
            vec![ToolArtifact {
                kind: "image".into(),
                title: topic.to_string(),
                content: json!({
                    "type": "generated_image",
                    "title": topic,
                    "prompt": prompt_variants
                        .first()
                        .and_then(|item| item.get("prompt"))
                        .and_then(|value| value.as_str())
                        .unwrap_or(""),
                    "image_size": image_size,
                    "images": images,
                    "data": output,
                }),
            }],
        )
    }
}
