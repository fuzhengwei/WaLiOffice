//! 联网搜索（ch3-3 节 stub，完整实现在 3-3 节）
//
// 完整实现见第 3-3 节「联网搜索」：

use async_trait::async_trait;

use crate::agent::tool::{OfficeTool, ToolContext, ToolResult};

pub struct WebSearchTool;

#[async_trait]
impl OfficeTool for WebSearchTool {
    fn name(&self) -> &str { "web_search" }
    fn description(&self) -> &str { "联网搜索（完整实现在 3-3 节）" }
    fn parameters(&self) -> serde_json::Value { serde_json::json!({}) }
    async fn call(&self, _input: serde_json::Value, _ctx: &ToolContext) -> ToolResult {
        ToolResult::err("web_search 工具完整实现在 ch3-3 节：联网搜索")
    }
}
