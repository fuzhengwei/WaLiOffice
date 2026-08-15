use async_trait::async_trait;
use serde_json::json;

use crate::agent::tool::{OfficeTool, ToolContext, ToolResult, ToolArtifact};
use crate::llm::LlmClient;
use crate::models::ChatMessage;

pub struct PptPlanTool;

#[async_trait]
impl OfficeTool for PptPlanTool {
    fn name(&self) -> &str {
        "ppt_plan"
    }

    fn description(&self) -> &str {
        "规划 PPT 大纲：根据用户需求生成 PPT 的页面规划（标题、布局、要点）。这是 PPT 生成的第一步，只产出规划，不生成最终幻灯片。"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "topic": {
                    "type": "string",
                    "description": "PPT 主题/用户需求"
                },
                "audience": {
                    "type": "string",
                    "description": "目标听众（可选）"
                }
            },
            "required": ["topic"]
        })
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn produces_artifact(&self) -> bool {
        false
    }

    async fn call(&self, input: serde_json::Value, ctx: &ToolContext) -> ToolResult {
        let topic = input
            .get("topic")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if topic.is_empty() {
            return ToolResult::err("topic 不能为空");
        }

        let audience = input.get("audience").and_then(|v| v.as_str()).unwrap_or("");

        ctx.send("state_update", json!({
            "phase": "running",
            "step": "规划 PPT 大纲",
            "detail": format!("正在为《{topic}》规划大纲..."),
            "at": chrono::Utc::now().to_rfc3339(),
        }));

        let client = LlmClient::for_user(&ctx.user_id, ctx.preferred_model.as_deref());

        let prompt = format!(
            r#"你是资深演示文稿策划。请先规划 PPT，不要生成完整页面元素。
用户需求：{topic}
{}

只返回 JSON，不要 markdown，不要解释。格式：
{{"title":"PPT标题","slides":[{{"title":"页标题","layout":"title|content|two-column|section","goal":"本页目标","visual":"视觉建议","points":["要点1","要点2"]}}]}}
要求：3-8页；每页points≤4；标题具体；visual说明卡片/分栏/流程/对比等结构。"#,
            if audience.is_empty() { String::new() } else { format!("目标听众：{audience}") }
        );

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "你只输出严格 JSON。".to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        match client.chat(&messages, None).await {
            Ok(resp) => {
                let content = resp
                    .choices
                    .first()
                    .and_then(|c| c.message.content.as_deref())
                    .unwrap_or("");

                let plan = match LlmClient::extract_json(content) {
                    Ok(v) => v,
                    Err(e) => {
                        return ToolResult::err(format!("PPT 大纲解析失败: {e}"));
                    }
                };

                let title = plan
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&topic)
                    .to_string();

                // 存入 scratchpad 供 ppt_generate 使用
                ctx.scratchpad
                    .lock()
                    .await
                    .insert("ppt_plan".to_string(), plan.clone());

                let slide_count = plan
                    .get("slides")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);

                ToolResult::ok(
                    format!("已规划 PPT《{title}》，共 {slide_count} 页大纲"),
                    vec![],
                )
                .with_data(plan)
            }
            Err(e) => ToolResult::err(format!("PPT 大纲生成失败: {e}")),
        }
    }
}
