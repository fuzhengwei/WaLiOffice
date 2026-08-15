use async_trait::async_trait;
use serde_json::json;

use crate::agent::tool::{OfficeTool, ToolArtifact, ToolContext, ToolResult};
use crate::llm::LlmClient;
use crate::models::ChatMessage;

pub struct PptPlanTool;

fn infer_ppt_scene(topic: &str) -> &'static str {
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
        "当前更像产品方案场景，优先组织为：背景与机会、用户与问题、方案设计、核心流程、版本规划、收益与风险。"
    } else if [
        "运营", "增长", "拉新", "留存", "转化", "活动", "campaign", "gmv",
    ]
    .iter()
    .any(|keyword| lower.contains(keyword))
    {
        "当前更像运营复盘/增长场景，优先组织为：目标与结果、核心指标、问题拆解、关键动作、复盘结论、下阶段计划。"
    } else if [
        "销售", "客户", "商机", "渠道", "业绩", "回款", "签约", "线索",
    ]
    .iter()
    .any(|keyword| lower.contains(keyword))
    {
        "当前更像销售汇报场景，优先组织为：业绩概览、区域/客户分析、机会与风险、重点动作、预测与资源诉求。"
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
        "当前更像技术设计/架构汇报场景，优先组织为：建设背景、总体架构、模块分层、关键流程、稳定性与安全、落地计划。"
    } else if [
        "培训", "课程", "学习", "上手", "入门", "手册", "宣导", "workshop",
    ]
    .iter()
    .any(|keyword| lower.contains(keyword))
    {
        "当前更像培训课件场景，优先组织为：学习目标、核心概念、方法步骤、案例演示、常见误区、行动建议。"
    } else if ["项目", "排期", "里程碑", "实施", "交付", "风险", "计划"]
        .iter()
        .any(|keyword| lower.contains(keyword))
    {
        "当前更像项目实施/计划汇报场景，优先组织为：目标范围、阶段计划、里程碑、角色分工、风险依赖、验收标准。"
    } else {
        "未识别到强场景时，默认按商务化正式汇报组织，兼顾背景、分析、方案、价值与下一步。"
    }
}

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
        let topic = input.get("topic").and_then(|v| v.as_str()).unwrap_or("");

        if topic.is_empty() {
            return ToolResult::err("topic 不能为空");
        }

        let audience = input.get("audience").and_then(|v| v.as_str()).unwrap_or("");

        ctx.send(
            "state_update",
            json!({
                "phase": "running",
                "step": "规划 PPT 大纲",
                "detail": format!("正在为《{topic}》规划大纲..."),
                "at": chrono::Utc::now().to_rfc3339(),
            }),
        );

        let client = LlmClient::for_user(&ctx.user_id, ctx.preferred_model.as_deref());
        let scene_guide = infer_ppt_scene(topic);

        let prompt = format!(
            r#"你是资深演示文稿策划。请先规划 PPT，不要生成完整页面元素。
用户需求：{topic}
{}
场景偏好：{scene_guide}

请把它规划成“像正式汇报材料”的大纲，而不是松散提纲。只返回 JSON，不要 markdown，不要解释。格式：
{{"title":"PPT标题","slides":[{{"title":"页标题","layout":"title|content|two-column|section","goal":"本页目标","visual":"视觉建议","points":["要点1","要点2"]}}]}}

规划要求：
- 页数控制在 5-10 页，默认按完整汇报材料思路组织
- 首页必须适合作为封面；末页优先做结论、建议或下一步行动
- 中间页要形成清晰叙事链：背景/目标 → 分析/拆解 → 方案/价值 → 落地/结论
- 每页标题必须具体，避免“概述”“分析”“总结”这类空泛标题
- 每页 points 2-4 条，每条一句话，适合直接上屏，不要写成长段落
- visual 要明确页面结构，例如：封面大标题、双栏对比、流程箭头、三卡片、四象限、时间轴、数据摘要卡片
- 如用户没有给出受众、页数、风格，请自动做合理假设，优先输出商务化、专业化成品大纲
- 如果主题涉及方案、汇报、复盘、运营分析、产品设计，请尽量补出关键指标、角色、阶段、风险、收益等视角"#,
            if audience.is_empty() {
                String::new()
            } else {
                format!("目标听众：{audience}")
            }
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
