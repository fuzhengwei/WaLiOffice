use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;

pub fn router() -> Router {
    Router::new().route("/api/health", get(health))
}

async fn health() -> Json<serde_json::Value> {
    let cfg = crate::config::config();
    Json(json!({
        "status": "ok",
        "app": cfg.app_name,
        "version": "0.2.0",
        "llm_model": cfg.llm_model,
        "llm_provider": cfg.llm_provider,
    }))
}
