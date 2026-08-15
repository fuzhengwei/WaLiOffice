pub mod doc_generate;
pub mod drawio_generate;
pub mod image_prompt;
pub mod md_generate;
pub mod ppt_generate;
pub mod ppt_plan;
pub mod sheet_generate;
pub mod web_search;

use super::registry::REGISTRY;
use std::sync::Arc;

pub async fn register_all_tools() {
    REGISTRY.register(Arc::new(ppt_plan::PptPlanTool)).await;
    REGISTRY
        .register(Arc::new(ppt_generate::PptGenerateTool))
        .await;
    REGISTRY
        .register(Arc::new(doc_generate::DocGenerateTool))
        .await;
    REGISTRY
        .register(Arc::new(md_generate::MarkdownGenerateTool))
        .await;
    REGISTRY
        .register(Arc::new(sheet_generate::SheetGenerateTool))
        .await;
    REGISTRY
        .register(Arc::new(drawio_generate::DrawioGenerateTool))
        .await;
    REGISTRY
        .register(Arc::new(image_prompt::ImagePromptTool))
        .await;
    REGISTRY.register(Arc::new(web_search::WebSearchTool)).await;

    let tools = REGISTRY.list().await;
    tracing::info!(
        "[AgentTools] 已注册 {} 个工具: {}",
        tools.len(),
        tools
            .iter()
            .map(|t| t.name())
            .collect::<Vec<_>>()
            .join(", ")
    );
}
