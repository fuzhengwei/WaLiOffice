use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::tool::{OfficeTool, ToolArtifact, ToolContext, ToolResult};
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

fn infer_sheet_scene(topic: &str) -> &'static str {
    let lower = topic.to_lowercase();

    if ["产品", "需求", "prd", "roadmap", "版本", "迭代", "feature"]
        .iter()
        .any(|keyword| lower.contains(keyword))
    {
        "当前更像产品管理场景，优先设计需求池、版本规划、优先级评估、验收清单等表格。"
    } else if [
        "运营", "增长", "拉新", "留存", "转化", "活动", "campaign", "gmv",
    ]
    .iter()
    .any(|keyword| lower.contains(keyword))
    {
        "当前更像运营分析场景，优先设计指标明细、渠道效果、活动复盘、周报汇总等表格。"
    } else if [
        "销售", "客户", "商机", "渠道", "业绩", "回款", "签约", "线索",
    ]
    .iter()
    .any(|keyword| lower.contains(keyword))
    {
        "当前更像销售管理场景，优先设计线索跟进、客户分层、商机漏斗、区域业绩等表格。"
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
        "当前更像技术项目场景，优先设计接口清单、服务台账、发布计划、风险清单或测试追踪表。"
    } else if [
        "培训", "课程", "学习", "上手", "入门", "手册", "宣导", "workshop",
    ]
    .iter()
    .any(|keyword| lower.contains(keyword))
    {
        "当前更像培训管理场景，优先设计课程安排、签到成绩、练习任务、反馈汇总等表格。"
    } else if ["项目", "排期", "里程碑", "实施", "交付", "风险", "计划"]
        .iter()
        .any(|keyword| lower.contains(keyword))
    {
        "当前更像项目管理场景，优先设计排期、里程碑、责任分工、风险跟踪和验收清单等表格。"
    } else {
        "默认按真实业务表格处理，兼顾明细、汇总、分析字段和执行字段。"
    }
}

#[async_trait]
impl OfficeTool for SheetGenerateTool {
    fn name(&self) -> &str {
        "sheet_generate"
    }

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
        let scene_guide = infer_sheet_scene(topic);
        if topic.is_empty() {
            return ToolResult::err("topic 不能为空");
        }

        ctx.send(
            "state_update",
            json!({
                "phase": "running",
                "step": "生成表格",
                "detail": format!("正在生成《{topic}》表格..."),
                "at": chrono::Utc::now().to_rfc3339(),
            }),
        );

        let system_prompt = r#"你是数据分析与表格设计专家。只输出严格 JSON，不要 markdown。
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
- 每个 table 至少 4 列、6 行数据，列设计要支持真实使用
- 数据要具体、真实、有意义，不要用占位符，不要整列都是“示例1/示例2”
- 表头要专业、可执行，例如负责人、阶段、金额、转化率、风险等级、开始日期、结束日期等
- 如果用户没有提供数据，请根据主题补出合理示例数据，保持同一张表内口径一致
- 如果需求适合多表，生成多个 table，例如“明细表 + 汇总表”“计划表 + 风险表”
- summary 要说明表格适合怎样使用或分析"#;

        let user_prompt = format!("请根据用户需求生成结构化表格数据，结果要更接近真实业务表格，而不是演示占位数据。\n场景偏好：{scene_guide}\n用户需求：{topic}");

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
            Err(e) => return ToolResult::err(format!("表格生成失败: {e}")),
        };

        let content = resp
            .choices
            .first()
            .and_then(|c| c.message.content.as_deref())
            .unwrap_or("");

        let output: SheetOutput = match LlmClient::extract_json(content)
            .and_then(|v| serde_json::from_value::<SheetOutput>(v).map_err(|e| anyhow::anyhow!(e)))
        {
            Ok(o) => o,
            Err(e) => {
                return ToolResult::err(format!("表格数据解析失败: {e}"));
            }
        };

        let table_count = output.tables.len();
        let total_rows: usize = output.tables.iter().map(|t| t.rows.len()).sum();

        ToolResult::ok(
            format!(
                "{}，共 {table_count} 个表格、{total_rows} 行数据",
                output
                    .summary
                    .unwrap_or_else(|| format!("已生成《{}》", output.title))
            ),
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
