//! draw.io 图表生成（ch3-8 节 stub，完整实现在 3-8 节）
//
// 完整实现见第 3-8 节「draw.io 图表生成」：

use async_trait::async_trait;

use crate::agent::tool::{OfficeTool, ToolContext, ToolResult};

pub struct DrawioGenerateTool;

#[async_trait]
impl OfficeTool for DrawioGenerateTool {
    fn name(&self) -> &str { "drawio_generate" }
    fn description(&self) -> &str { "draw.io 图表生成（完整实现在 3-8 节）" }
    fn parameters(&self) -> serde_json::Value { serde_json::json!({}) }
    async fn call(&self, _input: serde_json::Value, _ctx: &ToolContext) -> ToolResult {
        ToolResult::err("drawio_generate 工具完整实现在 ch3-8 节：draw.io 图表生成")
    }
}
