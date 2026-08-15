use async_trait::async_trait;
use serde::Serialize;
use serde_json::json;

use crate::agent::tool::{OfficeTool, ToolContext, ToolResult, ToolArtifact};
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

#[async_trait]
impl OfficeTool for ImagePromptTool {
    fn name(&self) -> &str { "image_prompt" }

    fn description(&self) -> &str {
        "生成图片提示词草案：根据用户需求生成多种风格的 AI 绘画提示词（含正向和负向 prompt）。"
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
        if topic.is_empty() {
            return ToolResult::err("topic 不能为空");
        }

        ctx.send("state_update", json!({
            "phase": "running",
            "step": "生成图片提示词",
            "detail": format!("正在为《{topic}》生成提示词..."),
            "at": chrono::Utc::now().to_rfc3339(),
        }));

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
要求：生成 3 种不同风格的提示词，每种 prompt 至少 50 词，细节丰富。"#;

        let user_prompt = format!("请为以下需求生成 AI 绘画提示词。\n需求：{topic}");

        let client = LlmClient::for_user(&ctx.user_id, ctx.preferred_model.as_deref());
        let messages = vec![
            ChatMessage { role: "system".into(), content: system_prompt.into(), tool_calls: None, tool_call_id: None },
            ChatMessage { role: "user".into(), content: user_prompt, tool_calls: None, tool_call_id: None },
        ];

        let resp = match client.chat(&messages, None).await {
            Ok(r) => r,
            Err(e) => return ToolResult::err(format!("图片提示词生成失败: {e}")),
        };

        let content = resp.choices.first().and_then(|c| c.message.content.as_deref()).unwrap_or("");
        let output = match LlmClient::extract_json(content) {
            Ok(v) => v,
            Err(e) => return ToolResult::err(format!("提示词解析失败: {e}")),
        };

        let style_count = output.get("prompts").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);

        ToolResult::ok(
            format!("已生成《{topic}》图片提示词，共 {style_count} 种风格"),
            vec![ToolArtifact {
                kind: "image".into(),
                title: topic.to_string(),
                content: json!({
                    "type": "image_prompt",
                    "title": topic,
                    "data": output,
                }),
            }],
        )
    }
}
