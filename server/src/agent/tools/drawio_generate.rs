use async_trait::async_trait;
use serde_json::json;

use crate::agent::tool::{OfficeTool, ToolContext, ToolResult, ToolArtifact};
use crate::llm::LlmClient;
use crate::models::ChatMessage;

pub struct DrawioGenerateTool;

#[async_trait]
impl OfficeTool for DrawioGenerateTool {
    fn name(&self) -> &str { "drawio_generate" }

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
        let diagram_type = input.get("diagram_type").and_then(|v| v.as_str()).unwrap_or("flowchart").to_string();
        if topic.is_empty() {
            return ToolResult::err("topic 不能为空");
        }

        ctx.send("state_update", json!({
            "phase": "running",
            "step": "生成图表",
            "detail": format!("正在生成《{topic}》{diagram_type}..."),
            "at": chrono::Utc::now().to_rfc3339(),
        }));

        let system_prompt = r#"你是 draw.io 图表设计专家。只输出 draw.io XML（mxGraphModel 格式），不要 markdown 代码块，不要解释。

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
- 图表完整、清晰"#;

        let user_prompt = format!("请生成一张{diagram_type}图表。\n需求：{topic}");

        let client = LlmClient::for_user(&ctx.user_id, ctx.preferred_model.as_deref());
        let messages = vec![
            ChatMessage { role: "system".into(), content: system_prompt.into(), tool_calls: None, tool_call_id: None },
            ChatMessage { role: "user".into(), content: user_prompt, tool_calls: None, tool_call_id: None },
        ];

        let resp = match client.chat(&messages, None).await {
            Ok(r) => r,
            Err(e) => return ToolResult::err(format!("图表生成失败: {e}")),
        };

        let content = resp.choices.first().and_then(|c| c.message.content.as_deref()).unwrap_or("");

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
