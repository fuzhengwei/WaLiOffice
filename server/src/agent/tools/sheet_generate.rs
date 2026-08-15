use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::tool::{OfficeTool, ToolContext, ToolResult, ToolArtifact};
use crate::llm::LlmClient;
use crate::models::ChatMessage;

pub struct SheetGenerateTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SheetTable {
    title: String,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    #[serde(default)]
    summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SheetOutput {
    title: String,
    tables: Vec<SheetTable>,
    #[serde(default)]
    summary: Option<String>,
}

#[async_trait]
impl OfficeTool for SheetGenerateTool {
    fn name(&self) -> &str { "sheet_generate" }

    fn description(&self) -> &str {
        "生成结构化表格：根据用户需求生成数据表格（可含多个 sheet），支持数据分析、排期、预算等，可导出为 Excel (.xlsx)。"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "topic": { "type": "string", "description": "表格主题/用户需求" },
                "sheets": { "type": "integer", "description": "需要的表格数量（可选，默认1）" }
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
            "step": "生成表格",
            "detail": format!("正在生成《{topic}》表格..."),
            "at": chrono::Utc::now().to_rfc3339(),
        }));

        let system_prompt = r#"你是数据分析专家。只输出严格 JSON，不要 markdown。
返回格式：
{
  "title": "表格组标题",
  "tables": [
    {
      "title": "表1标题",
      "headers": ["列1","列2","列3"],
      "rows": [["值1","值2","值3"]],
      "summary": "本表说明"
    }
  ],
  "summary": "整体说明"
}
要求：
- 每个 table 至少 3 列、5 行数据
- 数据要具体、真实、有意义，不要用占位符
- 如果需求适合多表，生成多个 table"#;

        let user_prompt = format!("请根据用户需求生成结构化表格数据。\n用户需求：{topic}");

        let client = LlmClient::for_user(&ctx.user_id, ctx.preferred_model.as_deref());
        let messages = vec![
            ChatMessage { role: "system".into(), content: system_prompt.into(), tool_calls: None, tool_call_id: None },
            ChatMessage { role: "user".into(), content: user_prompt, tool_calls: None, tool_call_id: None },
        ];

        let resp = match client.chat(&messages, None).await {
            Ok(r) => r,
            Err(e) => return ToolResult::err(format!("表格生成失败: {e}")),
        };

        let content = resp.choices.first().and_then(|c| c.message.content.as_deref()).unwrap_or("");

        let output: SheetOutput = match LlmClient::extract_json(content).and_then(|v| {
            serde_json::from_value::<SheetOutput>(v).map_err(|e| anyhow::anyhow!(e))
        }) {
            Ok(o) => o,
            Err(e) => {
                return ToolResult::err(format!("表格数据解析失败: {e}"));
            }
        };

        let table_count = output.tables.len();
        let total_rows: usize = output.tables.iter().map(|t| t.rows.len()).sum();

        ToolResult::ok(
            format!("{}，共 {table_count} 个表格、{total_rows} 行数据", output.summary.unwrap_or_else(|| format!("已生成《{}》", output.title))),
            vec![ToolArtifact {
                kind: "sheet".into(),
                title: output.title.clone(),
                content: json!({
                    "type": "sheet",
                    "title": output.title,
                    "tables": output.tables,
                }),
            }],
        )
    }
}
