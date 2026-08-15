pub mod auth;
pub mod chat;
pub mod session;
pub mod project;
pub mod task;
pub mod notification;
pub mod settings;
pub mod file;
pub mod dashboard;
pub mod doc_export;
pub mod health;
pub mod embed;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};

pub fn build_router() -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .merge(auth::router())
        .merge(chat::router())
        .merge(session::router())
        .merge(project::router())
        .merge(task::router())
        .merge(notification::router())
        .merge(settings::router())
        .merge(file::router())
        .merge(dashboard::router())
        .merge(doc_export::router())
        .merge(health::router())
        .fallback(embed::fallback_handler)
        .layer(cors)
}
