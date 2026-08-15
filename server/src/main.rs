mod config;
mod error;
mod state;
mod models;
mod db;
mod auth;
mod llm;
mod agent;
mod render;
mod routes;

use std::net::SocketAddr;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,walioffice=debug".into()),
        )
        .init();

    // 加载配置
    let cfg = config::config();
    cfg.ensure_dirs()?;

    // 初始化数据库（触发 Lazy）
    let _pool = state::db_pool();

    // 注册 Agent 工具
    agent::tools::register_all_tools().await;

    // 构建路由
    let app = routes::build_router();

    // 启动服务
    let addr: SocketAddr = format!("{}:{}", cfg.host, cfg.port).parse()?;
    info!("🚀 {} running at http://{}", cfg.app_name, addr);
    info!("📝 LLM: {} @ {}", cfg.llm_model, cfg.llm_base_url);
    info!("📂 Projects: {}", cfg.projects_dir);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
