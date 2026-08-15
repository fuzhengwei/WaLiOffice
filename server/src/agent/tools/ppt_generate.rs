use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::agent::tool::{OfficeTool, ToolArtifact, ToolContext, ToolResult};
use crate::db::project_repo;
use crate::llm::LlmClient;
use crate::models::{ChatMessage, PptProject, Slide, SlideElement};

pub struct PptGenerateTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SlidePlan {
    title: String,
    #[serde(default)]
    layout: Option<String>,
    #[serde(default)]
    goal: Option<String>,
    #[serde(default)]
    points: Option<Vec<String>>,
    #[serde(default)]
    visual: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PresentationPlan {
    title: String,
    slides: Vec<SlidePlan>,
}

struct Palette {
    bg: String,
    card: String,
    primary: String,
    accent: String,
    dark: String,
}

fn get_theme_palettes(theme: &str) -> Vec<Palette> {
    match theme {
        "business" => vec![
            Palette {
                bg: "F8FAFC".into(),
                card: "FFFFFF".into(),
                primary: "1D4ED8".into(),
                accent: "60A5FA".into(),
                dark: "0F172A".into(),
            },
            Palette {
                bg: "EFF6FF".into(),
                card: "FFFFFF".into(),
                primary: "0F4C81".into(),
                accent: "93C5FD".into(),
                dark: "111827".into(),
            },
            Palette {
                bg: "F1F5F9".into(),
                card: "FFFFFF".into(),
                primary: "334155".into(),
                accent: "38BDF8".into(),
                dark: "0F172A".into(),
            },
        ],
        "tech" => vec![
            Palette {
                bg: "020617".into(),
                card: "0F172A".into(),
                primary: "22D3EE".into(),
                accent: "A78BFA".into(),
                dark: "F8FAFC".into(),
            },
            Palette {
                bg: "0B1120".into(),
                card: "111827".into(),
                primary: "38BDF8".into(),
                accent: "34D399".into(),
                dark: "E5E7EB".into(),
            },
            Palette {
                bg: "111827".into(),
                card: "1F2937".into(),
                primary: "818CF8".into(),
                accent: "22D3EE".into(),
                dark: "F9FAFB".into(),
            },
        ],
        "warm" => vec![
            Palette {
                bg: "FFF7ED".into(),
                card: "FFFFFF".into(),
                primary: "EA580C".into(),
                accent: "FDBA74".into(),
                dark: "431407".into(),
            },
            Palette {
                bg: "FEF3C7".into(),
                card: "FFFBEB".into(),
                primary: "D97706".into(),
                accent: "F59E0B".into(),
                dark: "422006".into(),
            },
            Palette {
                bg: "FFF1F2".into(),
                card: "FFFFFF".into(),
                primary: "E11D48".into(),
                accent: "FDA4AF".into(),
                dark: "4C0519".into(),
            },
        ],
        "minimal" => vec![
            Palette {
                bg: "FFFFFF".into(),
                card: "F8FAFC".into(),
                primary: "111827".into(),
                accent: "CBD5E1".into(),
                dark: "111827".into(),
            },
            Palette {
                bg: "FAFAFA".into(),
                card: "FFFFFF".into(),
                primary: "27272A".into(),
                accent: "A1A1AA".into(),
                dark: "18181B".into(),
            },
            Palette {
                bg: "F4F4F5".into(),
                card: "FFFFFF".into(),
                primary: "3F3F46".into(),
                accent: "D4D4D8".into(),
                dark: "18181B".into(),
            },
        ],
        _ => vec![
            Palette {
                bg: "F8FAFC".into(),
                card: "FFFFFF".into(),
                primary: "2563EB".into(),
                accent: "38BDF8".into(),
                dark: "0F172A".into(),
            },
            Palette {
                bg: "FFF7ED".into(),
                card: "FFFFFF".into(),
                primary: "EA580C".into(),
                accent: "FDBA74".into(),
                dark: "111827".into(),
            },
            Palette {
                bg: "F0FDFA".into(),
                card: "FFFFFF".into(),
                primary: "0F766E".into(),
                accent: "5EEAD4".into(),
                dark: "0F172A".into(),
            },
            Palette {
                bg: "F5F3FF".into(),
                card: "FFFFFF".into(),
                primary: "7C3AED".into(),
                accent: "C4B5FD".into(),
                dark: "111827".into(),
            },
        ],
    }
}

fn plan_to_slide(
    project_title: &str,
    plan: &SlidePlan,
    index: usize,
    _total: usize,
    theme: &str,
) -> Slide {
    let is_title = index == 0 || plan.layout.as_deref() == Some("title");
    let is_section = plan.layout.as_deref() == Some("section");
    let points: Vec<String> = plan
        .points
        .as_ref()
        .map(|p| p.iter().take(4).cloned().collect())
        .unwrap_or_default();
    let palettes = get_theme_palettes(theme);
    let c = &palettes[index % palettes.len()];

    if is_title {
        return Slide {
            id: Uuid::new_v4().to_string(),
            layout: "title".into(),
            title: Some(plan.title.clone()),
            background: c.bg.clone(),
            notes: Some(plan.goal.clone().unwrap_or_default()),
            elements: vec![
                shape(0.0, 0.0, 13.33, 7.5, &c.bg, "rect"),
                shape(0.75, 0.7, 11.83, 6.1, &c.card, "roundRect"),
                shape(0.75, 0.7, 0.16, 6.1, &c.primary, "roundRect"),
                shape(10.5, 0.95, 1.5, 1.5, &c.accent, "ellipse"),
                text(
                    1.35,
                    2.25,
                    10.4,
                    0.95,
                    &plan.title,
                    44.0,
                    &c.dark,
                    true,
                    "left",
                    "middle",
                ),
                text(
                    1.4,
                    3.35,
                    9.4,
                    0.55,
                    plan.goal
                        .as_deref()
                        .or(plan.visual.as_deref())
                        .unwrap_or("现代简洁演示文稿"),
                    22.0,
                    "475569",
                    false,
                    "left",
                    "middle",
                ),
                text(
                    1.4,
                    5.75,
                    5.4,
                    0.35,
                    &format!("AI PPT · {}", chrono::Utc::now().format("%Y")),
                    13.0,
                    "64748B",
                    false,
                    "left",
                    "middle",
                ),
            ],
        };
    }

    if is_section {
        return Slide {
            id: Uuid::new_v4().to_string(),
            layout: "section".into(),
            title: Some(plan.title.clone()),
            background: c.primary.clone(),
            notes: Some(plan.goal.clone().unwrap_or_default()),
            elements: vec![
                shape(0.0, 0.0, 13.33, 7.5, &c.primary, "rect"),
                shape(8.5, -0.5, 4.83, 4.0, &c.accent, "ellipse"),
                text(
                    1.0,
                    3.0,
                    11.33,
                    1.5,
                    &plan.title,
                    40.0,
                    &c.dark,
                    true,
                    "left",
                    "middle",
                ),
                text(
                    1.05,
                    4.6,
                    10.0,
                    0.5,
                    plan.goal.as_deref().unwrap_or(""),
                    20.0,
                    &c.accent,
                    false,
                    "left",
                    "middle",
                ),
            ],
        };
    }

    // content 页
    let mut elements = vec![
        shape(0.0, 0.0, 13.33, 7.5, &c.bg, "rect"),
        shape(0.75, 0.7, 11.83, 6.1, &c.card, "roundRect"),
        shape(0.75, 0.7, 11.83, 0.08, &c.primary, "rect"),
        text(
            1.1,
            0.95,
            11.0,
            0.7,
            &plan.title,
            32.0,
            &c.dark,
            true,
            "left",
            "middle",
        ),
    ];

    if !points.is_empty() {
        let points_text = points
            .iter()
            .map(|p| format!("• {p}"))
            .collect::<Vec<_>>()
            .join("\n");
        elements.push(text(
            1.2,
            2.0,
            11.0,
            4.5,
            &points_text,
            20.0,
            "334155",
            false,
            "left",
            "top",
        ));
    }

    Slide {
        id: Uuid::new_v4().to_string(),
        layout: "content".into(),
        title: Some(plan.title.clone()),
        background: c.bg.clone(),
        notes: Some(plan.goal.clone().unwrap_or_default()),
        elements,
    }
}

fn shape(x: f64, y: f64, w: f64, h: f64, fill: &str, shape_type: &str) -> SlideElement {
    SlideElement {
        element_type: "shape".into(),
        x,
        y,
        w,
        h,
        fill: Some(fill.to_string()),
        shape: Some(shape_type.to_string()),
        text: None,
        font_size: None,
        color: None,
        bold: None,
        italic: None,
        align: None,
        valign: None,
        path: None,
        rows: None,
        cols: None,
        table_data: None,
        chart_type: None,
        chart_data: None,
    }
}

fn text(
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    content: &str,
    font_size: f64,
    color: &str,
    bold: bool,
    align: &str,
    valign: &str,
) -> SlideElement {
    SlideElement {
        element_type: "text".into(),
        x,
        y,
        w,
        h,
        text: Some(content.to_string()),
        font_size: Some(font_size),
        color: Some(color.to_string()),
        bold: Some(bold),
        italic: None,
        align: Some(align.to_string()),
        valign: Some(valign.to_string()),
        fill: None,
        shape: None,
        path: None,
        rows: None,
        cols: None,
        table_data: None,
        chart_type: None,
        chart_data: None,
    }
}

#[async_trait]
impl OfficeTool for PptGenerateTool {
    fn name(&self) -> &str {
        "ppt_generate"
    }

    fn description(&self) -> &str {
        "生成完整 PPT 项目：根据主题和大纲生成幻灯片（含视觉设计），写入项目文件。需要先调用 ppt_plan 规划大纲。"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "PPT 标题" },
                "topic": { "type": "string", "description": "用户原始需求（如果没有大纲则用于生成）" },
                "theme": { "type": "string", "description": "主题：default/business/tech/warm/minimal" }
            },
            "required": ["title"]
        })
    }

    async fn call(&self, input: serde_json::Value, ctx: &ToolContext) -> ToolResult {
        let title = input
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("未命名演示文稿")
            .to_string();
        let theme = input
            .get("theme")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();
        let topic = input
            .get("topic")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        ctx.send(
            "state_update",
            json!({
                "phase": "running",
                "step": "生成 PPT",
                "detail": format!("正在生成《{title}》幻灯片..."),
                "at": chrono::Utc::now().to_rfc3339(),
            }),
        );

        // 从 scratchpad 读取已规划的大纲
        let plan_value = ctx.scratchpad.lock().await.get("ppt_plan").cloned();

        let plan: PresentationPlan = if let Some(plan_json) = plan_value {
            match serde_json::from_value::<PresentationPlan>(plan_json) {
                Ok(p) => p,
                Err(_) => fallback_plan(&title, &topic),
            }
        } else {
            // 没有预规划，用 LLM 生成
            let client = LlmClient::for_user(&ctx.user_id, ctx.preferred_model.as_deref());
            match generate_plan_with_llm(&client, &title, &topic).await {
                Ok(p) => p,
                Err(_) => fallback_plan(&title, &topic),
            }
        };

        let now = chrono::Utc::now().to_rfc3339();
        let project_id = Uuid::new_v4().to_string();
        let total = plan.slides.len();
        let mut project = PptProject {
            id: project_id.clone(),
            title: plan.title.clone(),
            theme: theme.clone(),
            slides: vec![],
            history: Some(vec![json!({
                "type": "create",
                "title": format!("AI 生成《{}》", plan.title),
                "detail": format!("共 {} 页", total),
                "created_at": now,
            })]),
            layout: "16x9".into(),
            created_at: now.clone(),
            updated_at: now,
            owner_id: ctx.user_id.clone(),
        };

        if let Err(e) = project_repo::save_ppt_project(&project) {
            return ToolResult::err(format!("保存 PPT 项目失败: {e}"));
        }

        ctx.send(
            "project_update",
            json!({
                "project_id": project_id,
                "title": project.title,
                "theme": project.theme,
                "slides": project.slides,
                "slide_count": 0,
                "total_slides": total,
                "history": project.history,
            }),
        );

        for (i, sp) in plan.slides.iter().enumerate() {
            let slide = plan_to_slide(&plan.title, sp, i, total, &theme);
            project.slides.push(slide.clone());
            project.updated_at = chrono::Utc::now().to_rfc3339();

            if let Some(history) = project.history.as_mut() {
                history.push(json!({
                    "type": "draw",
                    "title": format!("生成第 {} 页：{}", i + 1, slide.title.clone().unwrap_or_else(|| format!("第 {} 页", i + 1))),
                    "detail": slide.notes.clone().unwrap_or_else(|| "已完成页面布局与内容填充".into()),
                    "slide_index": i,
                    "slide_title": slide.title,
                    "created_at": project.updated_at,
                }));
            }

            if let Err(e) = project_repo::save_ppt_project(&project) {
                return ToolResult::err(format!("保存 PPT 项目失败: {e}"));
            }

            ctx.send(
                "slide_update",
                json!({
                    "project_id": project.id,
                    "title": project.title,
                    "theme": project.theme,
                    "slides": project.slides,
                    "slide_count": project.slides.len(),
                    "total_slides": total,
                    "history": project.history,
                    "current_index": i,
                }),
            );
        }

        let slide_count = project.slides.len();
        let content = json!({
            "project_id": project_id,
            "title": plan.title,
            "theme": project.theme,
            "slide_count": slide_count,
            "total_slides": total,
            "slides": project.slides,
            "history": project.history,
        });

        ToolResult::ok(
            format!("已生成 PPT《{}》，共 {slide_count} 页", plan.title),
            vec![ToolArtifact {
                kind: "ppt".into(),
                title: plan.title.clone(),
                content,
            }],
        )
    }
}

async fn generate_plan_with_llm(
    client: &LlmClient,
    title: &str,
    topic: &str,
) -> anyhow::Result<PresentationPlan> {
    let lower = topic.to_lowercase();
    let scene_guide = if [
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
    };

    let prompt = format!(
        r#"你是资深演示文稿策划。请规划一份可以直接拿去汇报的 PPT 大纲。
标题：{title}
需求：{topic}
场景偏好：{scene_guide}

只返回 JSON：{{"title":"...","slides":[{{"title":"...","layout":"title|content|section","goal":"...","visual":"...","points":["..."]}}]}}
要求：
- 5-10页，结构完整，有封面和结尾页
- 页面顺序要有叙事感，不要东拼西凑
- 标题必须具体，避免空话
- 每页 points 2-4 条，适合上屏展示
- visual 要写明页面版式：如封面、数据卡片、流程、对比、时间轴、结论页
- 如用户没说清楚，就按专业商务汇报默认处理"#
    );
    let messages = vec![
        ChatMessage {
            role: "system".into(),
            content: "你只输出严格 JSON。".into(),
            tool_calls: None,
            tool_call_id: None,
        },
        ChatMessage {
            role: "user".into(),
            content: prompt,
            tool_calls: None,
            tool_call_id: None,
        },
    ];
    let resp = client.chat(&messages, None).await?;
    let content = resp
        .choices
        .first()
        .and_then(|c| c.message.content.as_deref())
        .unwrap_or("");
    let plan = LlmClient::extract_json(content)?;
    Ok(serde_json::from_value(plan)?)
}

fn fallback_plan(title: &str, topic: &str) -> PresentationPlan {
    PresentationPlan {
        title: title.to_string(),
        slides: vec![
            SlidePlan {
                title: title.to_string(),
                layout: Some("title".into()),
                goal: Some(topic.to_string()),
                points: None,
                visual: Some("标题页".into()),
            },
            SlidePlan {
                title: "核心内容".into(),
                layout: Some("content".into()),
                goal: Some("展开说明".into()),
                points: Some(vec![
                    "核心观点".into(),
                    "关键价值".into(),
                    "应用场景".into(),
                    "下一步".into(),
                ]),
                visual: Some("卡片列表".into()),
            },
            SlidePlan {
                title: "总结".into(),
                layout: Some("content".into()),
                goal: Some("总结与展望".into()),
                points: Some(vec!["成果回顾".into(), "未来规划".into()]),
                visual: Some("要点总结".into()),
            },
        ],
    }
}
