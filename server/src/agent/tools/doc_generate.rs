//! Word 文档生成（ch3-2 节 stub，完整实现在 3-2 节）
//
// 完整实现见第 3-2 节「Word 文档生成」：

use async_trait::async_trait;

use crate::agent::tool::{OfficeTool, ToolContext, ToolResult};

pub struct DocGenerateTool;

#[async_trait]
impl OfficeTool for DocGenerateTool {
    fn name(&self) -> &str { "doc_generate" }
    fn description(&self) -> &str { "Word 文档生成（完整实现在 3-2 节）" }
    fn parameters(&self) -> serde_json::Value { serde_json::json!({}) }
    async fn call(&self, _input: serde_json::Value, _ctx: &ToolContext) -> ToolResult {
        ToolResult::err("doc_generate 工具完整实现在 ch3-2 节：Word 文档生成")
    }
}
