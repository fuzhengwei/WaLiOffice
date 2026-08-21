use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::tool::{OfficeTool, ToolArtifact, ToolContext, ToolResult};
use crate::llm::LlmClient;
use crate::models::ChatMessage;

pub struct ChartGenerateTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChartOutput {
    title: String,
    #[serde(default)]
    summary: Option<String>,
    chart_type: String,
    labels: Vec<String>,
    values: Vec<f64>,
    #[serde(default)]
    series_name: Option<String>,
}

#[async_trait]
impl OfficeTool for ChartGenerateTool {
    fn name(&self) -> &str {
        "chart_generate"
    }

    fn description(&self) -> &str {
        "生成可视化图表：根据用户需求输出可直接渲染的 ECharts 图表产物，适合普通对话中的趋势、占比、对比、排名、漏斗和指标可视化。"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "topic": { "type": "string", "description": "图表主题/用户想可视化的问题" },
                "chart_type": { "type": "string", "description": "图表类型，可选 line/bar/pie/gauge/funnel/scatter，不确定时默认 bar" }
            },
            "required": ["topic"]
        })
    }

    async fn call(&self, input: serde_json::Value, ctx: &ToolContext) -> ToolResult {
        let topic = input.get("topic").and_then(|v| v.as_str()).unwrap_or("");
        if topic.trim().is_empty() {
            return ToolResult::err("topic 不能为空");
        }
        let preferred_type = input
            .get("chart_type")
            .and_then(|v| v.as_str())
            .unwrap_or("bar");

        ctx.send(
            "state_update",
            json!({
                "phase": "running",
                "step": "生成图表",
                "detail": format!("正在生成《{topic}》图表..."),
                "at": chrono::Utc::now().to_rfc3339(),
            }),
        );

        let system_prompt = r#"你是数据可视化专家。只输出严格 JSON，不要 markdown。
返回格式：
{
  "title": "图表标题",
  "summary": "图表说明",
  "chart_type": "bar|line|pie|gauge|funnel|scatter",
  "labels": ["类目1", "类目2"],
  "values": [12, 35],
  "series_name": "指标名称"
}
要求：
- 根据用户主题选择合适图表类型；如果用户指定 chart_type，优先遵循
- labels 与 values 长度必须一致
- values 必须是数字，不要带百分号或单位
- 如果用户没有给具体数据，补出合理示例数据，但要贴合业务场景
- 不要输出占位符、解释文本或代码块"#;

        let user_prompt = format!("用户想在对话中增强可视化体验，请生成一个可直接渲染的图表。\n用户需求：{topic}\n偏好图表类型：{preferred_type}");
        let client = LlmClient::for_user(&ctx.user_id, ctx.preferred_model.as_deref()).await;
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
            Err(e) => return ToolResult::err(format!("图表生成失败: {e}")),
        };
        let content = resp
            .choices
            .first()
            .and_then(|c| c.message.content.as_deref())
            .unwrap_or("");

        let output: ChartOutput = match LlmClient::extract_json(content)
            .and_then(|v| serde_json::from_value::<ChartOutput>(v).map_err(|e| anyhow::anyhow!(e)))
        {
            Ok(o) => o,
            Err(e) => return ToolResult::err(format!("图表数据解析失败: {e}")),
        };

        ToolResult::ok(
            format!(
                "已生成《{}》{}图表，可在右侧直接查看。",
                output.title, output.chart_type
            ),
            vec![ToolArtifact {
                kind: "chart".into(),
                title: output.title.clone(),
                content: json!({
                    "type": output.chart_type,
                    "chart_type": output.chart_type,
                    "title": output.title,
                    "summary": output.summary,
                    "chart_data": {
                        "labels": output.labels,
                        "values": output.values,
                        "seriesName": output.series_name.unwrap_or_else(|| "数据".into())
                    }
                }),
            }],
        )
    }
}
