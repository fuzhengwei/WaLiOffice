//! ECharts 图表生成（ch3-7 节 stub，完整实现在 3-7 节）
//
// 完整实现见第 3-7 节「ECharts 图表生成」：

use async_trait::async_trait;

use crate::agent::tool::{OfficeTool, ToolContext, ToolResult};

pub struct ChartGenerateTool;

#[async_trait]
impl OfficeTool for ChartGenerateTool {
    fn name(&self) -> &str { "chart_generate" }
    fn description(&self) -> &str { "ECharts 图表生成（完整实现在 3-7 节）" }
    fn parameters(&self) -> serde_json::Value { serde_json::json!({}) }
    async fn call(&self, _input: serde_json::Value, _ctx: &ToolContext) -> ToolResult {
        ToolResult::err("chart_generate 工具完整实现在 ch3-7 节：ECharts 图表生成")
    }
}
