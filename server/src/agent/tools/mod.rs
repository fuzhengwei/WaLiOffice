// 工具注册表（2-1 节 stub，完整实现在 2-4 节）
//
// 完整实现见 2-4 节「工具 Trait 与注册表」：
// - OfficeTool trait：name/description/parameters/call
// - ToolRegistry：RwLock<HashMap<String, Arc<dyn OfficeTool>>
// - register_all_tools()：遍历 tools/ 目录注册 10 个工具

/// 注册所有 Agent 工具（2-1 节暂不注册，完整实现见 2-4 节）
pub async fn register_all_tools() {
    // 完整实现：
    // REGISTRY.register(Arc::new(PptPlanTool::new())).await;
    // REGISTRY.register(Arc::new(PptGenerateTool::new())).await;
    // ... 共 10 个工具
    // tracing::info!("📦 已注册 X 个工具");
    tracing::info!("📦 Agent 工具注册表已就绪（工具实现在第 2-4 节接入）");
}
