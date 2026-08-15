use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::tool::{OfficeTool, ToolContext, ToolResult, ToolArtifact};
use crate::llm::LlmClient;
use crate::models::ChatMessage;

pub struct DocGenerateTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DocSection {
    heading: String,
    #[serde(default = "default_heading_level")]
    heading_level: u32,
    #[serde(default)]
    paragraphs: Vec<String>,
    #[serde(default)]
    bullets: Vec<String>,
    #[serde(default)]
    table: Option<DocTable>,
}

fn default_heading_level() -> u32 { 1 }

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DocTable {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DocOutput {
    title: String,
    sections: Vec<DocSection>,
    #[serde(default)]
    summary: Option<String>,
}

#[async_trait]
impl OfficeTool for DocGenerateTool {
    fn name(&self) -> &str { "doc_generate" }

    fn description(&self) -> &str {
        "生成结构化文档：根据用户需求生成可编辑文档，支持报告、计划、总结、文章、PRD 等类型，可导出为 Word (.docx)。"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "topic": { "type": "string", "description": "文档主题/用户需求" },
                "audience": { "type": "string", "description": "目标读者（可选）" },
                "format": { "type": "string", "description": "文档类型：report/plan/summary/article/prd", "enum": ["report", "plan", "summary", "article", "prd"] }
            },
            "required": ["topic"]
        })
    }

    async fn call(&self, input: serde_json::Value, ctx: &ToolContext) -> ToolResult {
        let topic = input.get("topic").and_then(|v| v.as_str()).unwrap_or("");
        let audience = input.get("audience").and_then(|v| v.as_str()).unwrap_or("");
        let format = input.get("format").and_then(|v| v.as_str()).unwrap_or("report").to_string();

        if topic.is_empty() {
            return ToolResult::err("topic 不能为空");
        }

        ctx.send("state_update", json!({
            "phase": "running",
            "step": "生成文档",
            "detail": format!("正在生成《{topic}》{format}..."),
            "at": chrono::Utc::now().to_rfc3339(),
        }));

        let format_guide = match format.as_str() {
            "report" => "生成一份结构完整的分析报告，包含背景、现状分析、数据对比、问题诊断、建议方案。",
            "plan" => "生成一份详细的工作计划，包含目标、时间表、资源需求、风险评估。",
            "summary" => "生成一份精炼的总结，包含核心结论、关键数据、经验教训、后续建议。",
            "article" => "生成一篇有深度的文章，包含引言、论述、案例、结论。",
            "prd" => "生成一份产品需求文档，包含背景、目标、功能列表、优先级、非功能需求。",
            _ => "生成一份结构完整的文档。",
        };

        let system_prompt = r#"你是资深文档顾问。只输出严格 JSON，不要 markdown 代码块。
返回格式：
{
  "title": "文档标题",
  "sections": [
    {
      "heading": "章节标题",
      "headingLevel": 1,
      "paragraphs": ["段落文本..."],
      "bullets": ["要点1", "要点2"],
      "table": { "headers": ["列1","列2"], "rows": [["值1","值2"]] }
    }
  ],
  "summary": "一句话说明产物价值"
}
要求：
- sections 数组至少 5 个章节，内容丰富完整
- 每个章节至少有 2-3 个 paragraphs 或 3-5 个 bullets
- 至少 2 个章节包含 table（如果内容适合表格展示）
- 使用 headingLevel 1-3 创建层次结构
- 段落内容具体、有深度，每段 50-150 字
- 段落中可使用 **粗体** 和 *斜体* 标记重点"#;

        let user_prompt = format!(
            "请根据用户需求生成一份完整、专业的结构化文档。要求：{format_guide}\n{}用户需求：{topic}",
            if audience.is_empty() { String::new() } else { format!("目标读者：{audience}\n") }
        );

        let client = LlmClient::for_user(&ctx.user_id, ctx.preferred_model.as_deref());
        let messages = vec![
            ChatMessage { role: "system".into(), content: system_prompt.into(), tool_calls: None, tool_call_id: None },
            ChatMessage { role: "user".into(), content: user_prompt, tool_calls: None, tool_call_id: None },
        ];

        let resp = match client.chat(&messages, None).await {
            Ok(r) => r,
            Err(e) => return ToolResult::err(format!("文档生成失败: {e}")),
        };

        let content = resp.choices.first().and_then(|c| c.message.content.as_deref()).unwrap_or("");

        let doc: DocOutput = match LlmClient::extract_json(content).and_then(|v| {
            serde_json::from_value::<DocOutput>(v).map_err(|e| anyhow::anyhow!(e))
        }) {
            Ok(d) => d,
            Err(e) => {
                // 降级草稿
                let fallback = DocOutput {
                    title: topic.chars().take(32).collect(),
                    sections: vec![
                        DocSection { heading: "需求原文".into(), heading_level: 1, paragraphs: vec![topic.to_string()], bullets: vec![], table: None },
                        DocSection { heading: "待补充".into(), heading_level: 1, paragraphs: vec![], bullets: vec!["目标与范围".into(), "关键内容".into(), "交付标准".into()], table: None },
                    ],
                    summary: Some(format!("文档《{topic}》已生成（降级草稿）: {e}")),
                };
                let markdown = sections_to_markdown(&fallback);
                return ToolResult::ok(
                    format!("文档《{}》已生成（降级草稿）", fallback.title),
                    vec![ToolArtifact {
                        kind: "document".into(),
                        title: fallback.title.clone(),
                        content: json!({
                            "type": "structured",
                            "title": fallback.title,
                            "sections": fallback.sections,
                            "markdown": markdown,
                            "format": format,
                            "generated_by": "fallback",
                        }),
                    }],
                );
            }
        };

        let markdown = sections_to_markdown(&doc);
        let section_count = doc.sections.len();

        ToolResult::ok(
            format!("{}，共 {section_count} 个章节", doc.summary.unwrap_or_else(|| format!("已生成《{}》", doc.title))),
            vec![ToolArtifact {
                kind: "document".into(),
                title: doc.title.clone(),
                content: json!({
                    "type": "structured",
                    "title": doc.title,
                    "sections": doc.sections,
                    "markdown": markdown,
                    "format": format,
                    "generated_by": "llm",
                }),
            }],
        )
    }
}

pub fn sections_to_markdown(doc: &DocOutput) -> String {
    let mut md = format!("# {}\n\n", doc.title);
    for section in &doc.sections {
        let prefix = "#".repeat(section.heading_level.min(6) as usize);
        md.push_str(&format!("{prefix} {}\n\n", section.heading));
        for p in &section.paragraphs {
            md.push_str(p);
            md.push_str("\n\n");
        }
        if !section.bullets.is_empty() {
            for b in &section.bullets {
                md.push_str(&format!("- {b}\n"));
            }
            md.push('\n');
        }
        if let Some(table) = &section.table {
            md.push_str(&format!("| {} |\n", table.headers.join(" | ")));
            md.push_str(&format!("| {} |\n", table.headers.iter().map(|_| "---").collect::<Vec<_>>().join(" | ")));
            for row in &table.rows {
                md.push_str(&format!("| {} |\n", row.join(" | ")));
            }
            md.push('\n');
        }
    }
    md
}
