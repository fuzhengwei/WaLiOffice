//! PPT 大纲规划（ch3-5 节 stub，完整实现在 3-5 节）
//
// 完整实现见第 3-5 节「PPT 大纲规划」：

use async_trait::async_trait;

use crate::agent::tool::{OfficeTool, ToolContext, ToolResult};

pub struct PptPlanTool;

#[async_trait]
impl OfficeTool for PptPlanTool {
    fn name(&self) -> &str { "ppt_plan" }
    fn description(&self) -> &str { "PPT 大纲规划（完整实现在 3-5 节）" }
    fn parameters(&self) -> serde_json::Value { serde_json::json!({}) }
    async fn call(&self, _input: serde_json::Value, _ctx: &ToolContext) -> ToolResult {
        ToolResult::err("ppt_plan 工具完整实现在 ch3-5 节：PPT 大纲规划")
    }
}
