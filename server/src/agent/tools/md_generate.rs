//! Markdown 文档生成（ch3-1 节 stub，完整实现在 3-1 节）
//
// 完整实现见第 3-1 节「Markdown 文档生成」：

use async_trait::async_trait;

use crate::agent::tool::{OfficeTool, ToolContext, ToolResult};

pub struct MarkdownGenerateTool;

#[async_trait]
impl OfficeTool for MarkdownGenerateTool {
    fn name(&self) -> &str { "md_generate" }
    fn description(&self) -> &str { "Markdown 文档生成（完整实现在 3-1 节）" }
    fn parameters(&self) -> serde_json::Value { serde_json::json!({}) }
    async fn call(&self, _input: serde_json::Value, _ctx: &ToolContext) -> ToolResult {
        ToolResult::err("md_generate 工具完整实现在 ch3-1 节：Markdown 文档生成")
    }
}
