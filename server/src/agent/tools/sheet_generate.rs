//! Excel 表格生成（ch3-4 节 stub，完整实现在 3-4 节）
//
// 完整实现见第 3-4 节「Excel 表格生成」：

use async_trait::async_trait;

use crate::agent::tool::{OfficeTool, ToolContext, ToolResult};

pub struct SheetGenerateTool;

#[async_trait]
impl OfficeTool for SheetGenerateTool {
    fn name(&self) -> &str { "sheet_generate" }
    fn description(&self) -> &str { "Excel 表格生成（完整实现在 3-4 节）" }
    fn parameters(&self) -> serde_json::Value { serde_json::json!({}) }
    async fn call(&self, _input: serde_json::Value, _ctx: &ToolContext) -> ToolResult {
        ToolResult::err("sheet_generate 工具完整实现在 ch3-4 节：Excel 表格生成")
    }
}
