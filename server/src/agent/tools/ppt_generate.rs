//! PPT 幻灯片生成（ch3-6 节 stub，完整实现在 3-6 节）
//
// 完整实现见第 3-6 节「PPT 幻灯片生成」：

use async_trait::async_trait;

use crate::agent::tool::{OfficeTool, ToolContext, ToolResult};

pub struct PptGenerateTool;

#[async_trait]
impl OfficeTool for PptGenerateTool {
    fn name(&self) -> &str { "ppt_generate" }
    fn description(&self) -> &str { "PPT 幻灯片生成（完整实现在 3-6 节）" }
    fn parameters(&self) -> serde_json::Value { serde_json::json!({}) }
    async fn call(&self, _input: serde_json::Value, _ctx: &ToolContext) -> ToolResult {
        ToolResult::err("ppt_generate 工具完整实现在 ch3-6 节：PPT 幻灯片生成")
    }
}
