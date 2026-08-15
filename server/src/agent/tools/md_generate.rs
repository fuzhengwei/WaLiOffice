use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::tool::{OfficeTool, ToolArtifact, ToolContext, ToolResult};
use crate::llm::LlmClient;
use crate::models::ChatMessage;

pub struct MarkdownGenerateTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MarkdownOutput {
    title: String,
    markdown: String,
    #[serde(default)]
    summary: Option<String>,
}

fn infer_markdown_scene(topic: &str) -> &'static str {
    let lower = topic.to_lowercase();

    if [
        "产品",
        "需求",
        "prd",
        "roadmap",
        "版本",
        "迭代",
        "用户故事",
        "feature",
    ]
    .iter()
    .any(|keyword| lower.contains(keyword))
    {
        "当前更像产品知识沉淀场景，请重点补足背景、用户场景、功能说明、流程、边界和 FAQ。"
    } else if [
        "运营", "增长", "拉新", "留存", "转化", "活动", "campaign", "gmv",
    ]
    .iter()
    .any(|keyword| lower.contains(keyword))
    {
        "当前更像运营复盘/方法论沉淀场景，请重点补足指标口径、动作拆解、案例和经验总结。"
    } else if [
        "销售", "客户", "商机", "渠道", "业绩", "回款", "签约", "线索",
    ]
    .iter()
    .any(|keyword| lower.contains(keyword))
    {
        "当前更像销售资料整理场景，请重点补足客户画像、销售流程、关键话术、阶段策略和常见问题。"
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
        "当前更像技术文档/README 场景，请重点补足架构说明、目录结构、安装步骤、配置示例、调用示例和排错说明。"
    } else if [
        "培训", "课程", "学习", "上手", "入门", "手册", "宣导", "workshop",
    ]
    .iter()
    .any(|keyword| lower.contains(keyword))
    {
        "当前更像培训讲义/操作手册场景，请重点补足学习路径、步骤、示例、练习建议和常见误区。"
    } else if ["项目", "排期", "里程碑", "实施", "交付", "风险", "计划"]
        .iter()
        .any(|keyword| lower.contains(keyword))
    {
        "当前更像项目执行手册场景，请重点补足实施阶段、责任分工、里程碑、依赖项和验收标准。"
    } else {
        "默认按正式知识文档处理，兼顾背景、核心说明、示例、FAQ 和后续建议。"
    }
}

#[async_trait]
impl OfficeTool for MarkdownGenerateTool {
    fn name(&self) -> &str {
        "md_generate"
    }

    fn description(&self) -> &str {
        "生成 Markdown 文档：适合知识库、README、说明文档、会议纪要、调研整理、操作手册等纯文本结构化内容，可在右侧直接渲染并下载 .md 文件。"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "topic": { "type": "string", "description": "Markdown 文档主题/用户需求" },
                "style": {
                    "type": "string",
                    "description": "文档风格，可选：knowledge_base/readme/guide/notes/research",
                    "enum": ["knowledge_base", "readme", "guide", "notes", "research"]
                },
                "audience": { "type": "string", "description": "目标读者（可选）" }
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
        let style = input
            .get("style")
            .and_then(|v| v.as_str())
            .unwrap_or("knowledge_base");
        let audience = input
            .get("audience")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let scene_guide = infer_markdown_scene(topic);

        if topic.is_empty() {
            return ToolResult::err("topic 不能为空");
        }

        ctx.send(
            "state_update",
            json!({
                "phase": "running",
                "step": "生成 Markdown 文档",
                "detail": format!("正在整理《{topic}》的 Markdown 内容..."),
                "at": chrono::Utc::now().to_rfc3339(),
            }),
        );

        let style_guide = match style {
            "readme" => "输出 README 风格文档，优先包含简介、核心能力、快速开始、使用步骤、目录结构、示例、注意事项，适合直接放仓库首页。",
            "guide" => "输出操作指南，优先包含适用场景、前置条件、步骤说明、关键截图说明位、常见问题和注意事项，适合直接给用户阅读。",
            "notes" => "输出会议纪要或整理笔记，优先包含会议背景、关键结论、待办事项、责任人、时间点和后续跟进。",
            "research" => "输出调研整理文档，优先包含背景、信息来源、关键信息摘要、对比、结论和建议，适合知识沉淀。",
            _ => "输出知识库风格文档，优先包含概览、核心说明、要点列表、示例、FAQ 和补充说明。",
        };

        let system_prompt = r##"你是资深技术写作者。只输出严格 JSON，不要 markdown 代码块。
返回格式：
{
  "title": "文档标题",
  "markdown": "# 标题\n\n## 小节\n- 要点",
  "summary": "一句话说明内容价值"
}
要求：
- markdown 必须是完整、可直接保存为 .md 文件的正文
- 使用标准 Markdown 语法，至少包含 4 个二级标题
- 适当使用列表、表格、引用、任务列表或代码块提升可读性
- 内容要具体，不要只写提纲，也不要输出“待补充”
- 如果用户没有给足细节，请根据场景自动补足合理示例、命令示例、步骤说明、表格字段和 FAQ
- README / 指南类内容要强调可操作性；调研类内容要强调结论与依据；纪要类内容要强调行动项
- 不要输出 JSON 之外的任何解释"##;

        let user_prompt = format!(
            "请根据以下需求生成一份适合保存为 Markdown 文件的正式内容，要求读起来像可直接发布的成品文档。要求：{style_guide}\n场景偏好：{scene_guide}\n{}用户需求：{topic}",
            if audience.is_empty() { String::new() } else { format!("目标读者：{audience}\n") }
        );

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
            Err(e) => return ToolResult::err(format!("Markdown 文档生成失败: {e}")),
        };

        let content = resp
            .choices
            .first()
            .and_then(|c| c.message.content.as_deref())
            .unwrap_or("");

        let output: MarkdownOutput = match LlmClient::extract_json(content).and_then(|v| {
            serde_json::from_value::<MarkdownOutput>(v).map_err(|e| anyhow::anyhow!(e))
        }) {
            Ok(doc) => doc,
            Err(e) => MarkdownOutput {
                title: topic.chars().take(32).collect(),
                markdown: format!("# {topic}\n\n## 待补充\n\n- 请补充核心内容\n- 请补充结构化说明\n\n> 当前为降级草稿：{e}"),
                summary: Some(format!("Markdown 文档《{topic}》已生成（降级草稿）")),
            },
        };

        ToolResult::ok(
            output
                .summary
                .clone()
                .unwrap_or_else(|| format!("已生成 Markdown 文档《{}》", output.title)),
            vec![ToolArtifact {
                kind: "markdown".into(),
                title: output.title.clone(),
                content: json!({
                    "type": "markdown",
                    "title": output.title,
                    "markdown": output.markdown,
                    "style": style,
                    "summary": output.summary,
                }),
            }],
        )
    }
}
