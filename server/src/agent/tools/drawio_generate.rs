use async_trait::async_trait;
use serde_json::json;

use crate::agent::tool::{OfficeTool, ToolArtifact, ToolContext, ToolResult};
use crate::llm::LlmClient;
use crate::models::ChatMessage;

pub struct DrawioGenerateTool;

fn infer_diagram_scene(topic: &str) -> &'static str {
    let lower = topic.to_lowercase();

    if ["产品", "需求", "prd", "roadmap", "版本", "迭代", "feature"]
        .iter()
        .any(|keyword| lower.contains(keyword))
    {
        "当前更像产品方案场景，优先体现用户流程、功能模块、角色关系或版本路径。"
    } else if [
        "运营", "增长", "拉新", "留存", "转化", "活动", "campaign", "gmv",
    ]
    .iter()
    .any(|keyword| lower.contains(keyword))
    {
        "当前更像运营流程场景，优先体现渠道、动作链路、指标漏斗和复盘闭环。"
    } else if [
        "销售", "客户", "商机", "渠道", "业绩", "回款", "签约", "线索",
    ]
    .iter()
    .any(|keyword| lower.contains(keyword))
    {
        "当前更像销售流程/经营场景，优先体现线索到签约的阶段流转、角色协同和客户分层。"
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
        "当前更像技术架构场景，优先体现系统边界、模块层次、调用链路、数据流和基础设施。"
    } else if [
        "培训", "课程", "学习", "上手", "入门", "手册", "宣导", "workshop",
    ]
    .iter()
    .any(|keyword| lower.contains(keyword))
    {
        "当前更像培训流程场景，优先体现学习路径、步骤、角色分工和知识结构。"
    } else if ["项目", "排期", "里程碑", "实施", "交付", "风险", "计划"]
        .iter()
        .any(|keyword| lower.contains(keyword))
    {
        "当前更像项目实施场景，优先体现阶段、责任人、交付物和依赖关系。"
    } else {
        "默认按方案汇报图处理，兼顾层次、关系和对外讲解的清晰度。"
    }
}

#[async_trait]
impl OfficeTool for DrawioGenerateTool {
    fn name(&self) -> &str {
        "drawio_generate"
    }

    fn description(&self) -> &str {
        "生成 draw.io 可编辑图表：支持流程图、架构图、泳道图、拓扑图、ER图等，输出 draw.io XML 格式。"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "topic": { "type": "string", "description": "图表主题/用户需求" },
                "diagram_type": { "type": "string", "description": "图表类型：flowchart/architecture/swimlane/topology/er/mindmap", "enum": ["flowchart", "architecture", "swimlane", "topology", "er", "mindmap"] }
            },
            "required": ["topic"]
        })
    }

    async fn call(&self, input: serde_json::Value, ctx: &ToolContext) -> ToolResult {
        let topic = input.get("topic").and_then(|v| v.as_str()).unwrap_or("");
        let diagram_type = input
            .get("diagram_type")
            .and_then(|v| v.as_str())
            .unwrap_or("flowchart")
            .to_string();
        let scene_guide = infer_diagram_scene(topic);
        if topic.is_empty() {
            return ToolResult::err("topic 不能为空");
        }

        ctx.send(
            "state_update",
            json!({
                "phase": "running",
                "step": "生成图表",
                "detail": format!("正在生成《{topic}》{diagram_type}..."),
                "at": chrono::Utc::now().to_rfc3339(),
            }),
        );

        let system_prompt = r#"你是 draw.io 图表设计专家。只输出 draw.io XML（mxGraphModel 或 mxfile 格式），不要 markdown 代码块，不要解释。

XML 格式示例：
<mxGraphModel dx="800" dy="600" grid="1" gridSize="10" guides="1" tooltips="1" connect="1" arrows="1" fold="1" page="1" pageScale="1" pageWidth="850" pageHeight="600" math="0" shadow="0">
  <root>
    <mxCell id="0"/>
    <mxCell id="1" parent="0"/>
    <mxCell id="2" value="节点1" style="rounded=1;whiteSpace=wrap;html=1;fillColor=#dae8fc;strokeColor=#6c8ebf;" vertex="1" parent="1">
      <mxGeometry x="100" y="100" width="120" height="60" as="geometry"/>
    </mxCell>
  </root>
</mxGraphModel>

要求：
- 使用合理的布局坐标，节点不重叠
- 用箭头连接表示关系
- 使用不同的颜色和样式区分节点类型
- 中文标签
- 图表完整、清晰，适合继续编辑
- 根据图表类型选择合适结构：
  - flowchart：体现步骤流转、条件分支和结果
  - architecture：体现层次、模块边界、依赖关系、数据流
  - swimlane：体现角色、职责和跨角色流转
  - topology：体现节点、网络连接和部署关系
  - er：体现实体、字段主键和关系
  - mindmap：体现主题、分支和层级
- 优先生成“对外能讲清楚”的布局，不要把所有节点简单排成一行
- 如用户没有给出细节，请补足合理的模块、阶段、角色或系统组件，使图更像正式方案图"#;

        let user_prompt = format!("请生成一张{diagram_type}图表，要求既便于阅读，也便于后续在 draw.io 中继续编辑。\n场景偏好：{scene_guide}\n需求：{topic}");

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
            Err(e) => return ToolResult::err(format!("图表生成失败: {e}")),
        };

        let content = resp
            .choices
            .first()
            .and_then(|c| c.message.content.as_deref())
            .unwrap_or("");

        // 清理可能的 markdown fence
        let xml = content
            .trim()
            .trim_start_matches("```xml")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
            .to_string();

        if !xml.contains("<mxGraphModel") && !xml.contains("<mxfile") {
            return ToolResult::err("图表生成失败：模型未返回有效的 draw.io XML");
        }

        ToolResult::ok(
            format!("已生成《{topic}》{diagram_type}图表"),
            vec![ToolArtifact {
                kind: "drawio".into(),
                title: topic.to_string(),
                content: json!({
                    "type": "drawio",
                    "title": topic,
                    "diagram_type": diagram_type,
                    "xml": xml,
                }),
            }],
        )
    }
}
