mod agent;
mod auth;
mod config;
mod db;
mod error;
mod file_extract;
mod image_ocr;
mod llm;
mod models;
mod render;
mod routes;
mod state;

use std::net::SocketAddr;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, warn};

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

    // ── 启动 DSH Agent Engine 子进程 ──
    // Rust 作为主服务，自动拉起 DSH Node.js 引擎作为子进程
    // 对外只有一个端口 8000，DSH 在内部 port 3780 通信
    let agent_engine_dir = std::env::current_dir()
        .map(|p| p.parent().unwrap().join("agent-engine"))
        .unwrap_or_else(|_| std::path::PathBuf::from("../agent-engine"));

    let dsh_enabled = agent_engine_dir.exists()
        && std::env::var("DSH_DISABLED").is_err();

    let _dsh_child = if dsh_enabled {
        info!("🔌 Starting DSH Agent Engine from {:?}", agent_engine_dir);

        let serve_path = agent_engine_dir.join("serve.ts");
        let mut dsh_cmd = Command::new("node");
        dsh_cmd
            .arg("--import")
            .arg("tsx")
            .arg(&serve_path)
            .current_dir(&agent_engine_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);

        // 传递关键环境变量
        if let Ok(key) = std::env::var("LLM_TEXT_API_KEY") {
            dsh_cmd.env("LLM_TEXT_API_KEY", key);
        }
        if let Ok(url) = std::env::var("LLM_TEXT_BASE_URL") {
            dsh_cmd.env("LLM_TEXT_BASE_URL", url);
        }
        if let Ok(models) = std::env::var("LLM_TEXT_MODELS") {
            dsh_cmd.env("LLM_TEXT_MODELS", models);
        }
        dsh_cmd.env("LLM_TEXT_MODEL_DEFAULT", std::env::var("LLM_TEXT_MODELS").unwrap_or_else(|_| "deepseek-chat".into()));
        dsh_cmd.env("RUST_BACKEND_URL", format!("http://127.0.0.1:{}", cfg.port));
        dsh_cmd.env("DSH_PORT", "3780");

        match dsh_cmd.spawn() {
            Ok(child) => {
                info!("✅ DSH Agent Engine started (pid: {:?})", child.id());
                // 给 DSH 一点时间初始化
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                Some(child)
            }
            Err(e) => {
                warn!("⚠️ Failed to start DSH Agent Engine: {} — falling back to Rust-only mode", e);
                None
            }
        }
    } else {
        info!("📦 DSH Agent Engine disabled (agent-engine/ not found or DSH_DISABLED set)");
        None
    };

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
