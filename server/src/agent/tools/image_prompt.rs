//! 图片生成（ch3-9 节 stub，完整实现在 3-9 节）
//
// 完整实现见第 3-9 节「图片生成」：

use async_trait::async_trait;

use crate::agent::tool::{OfficeTool, ToolContext, ToolResult};

pub struct ImagePromptTool;

#[async_trait]
impl OfficeTool for ImagePromptTool {
    fn name(&self) -> &str { "image_prompt" }
    fn description(&self) -> &str { "图片生成（完整实现在 3-9 节）" }
    fn parameters(&self) -> serde_json::Value { serde_json::json!({}) }
    async fn call(&self, _input: serde_json::Value, _ctx: &ToolContext) -> ToolResult {
        ToolResult::err("image_prompt 工具完整实现在 ch3-9 节：图片生成")
    }
}
