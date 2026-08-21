//! Agnes 多媒体生成（ch3-10 节 stub，完整实现在 3-10 节）
//
// 完整实现见第 3-10 节「Agnes 多媒体生成」：

use async_trait::async_trait;

use crate::agent::tool::{OfficeTool, ToolContext, ToolResult};

pub struct AgnesMediaTool;

#[async_trait]
impl OfficeTool for AgnesMediaTool {
    fn name(&self) -> &str { "agnes_media" }
    fn description(&self) -> &str { "Agnes 多媒体生成（完整实现在 3-10 节）" }
    fn parameters(&self) -> serde_json::Value { serde_json::json!({}) }
    async fn call(&self, _input: serde_json::Value, _ctx: &ToolContext) -> ToolResult {
        ToolResult::err("agnes_media 工具完整实现在 ch3-10 节：Agnes 多媒体生成")
    }
}
