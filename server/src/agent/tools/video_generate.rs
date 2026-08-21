//! 视频生成（ch3-11 节 stub，完整实现在 3-11 节）
//
// 完整实现见第 3-11 节「视频生成」：

use async_trait::async_trait;

use crate::agent::tool::{OfficeTool, ToolContext, ToolResult};

pub struct VideoGenerateTool;

#[async_trait]
impl OfficeTool for VideoGenerateTool {
    fn name(&self) -> &str { "video_generate" }
    fn description(&self) -> &str { "视频生成（完整实现在 3-11 节）" }
    fn parameters(&self) -> serde_json::Value { serde_json::json!({}) }
    async fn call(&self, _input: serde_json::Value, _ctx: &ToolContext) -> ToolResult {
        ToolResult::err("video_generate 工具完整实现在 ch3-11 节：视频生成")
    }
}
