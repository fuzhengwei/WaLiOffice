pub mod auth;
pub mod chat;
pub mod dashboard;
pub mod doc_export;
pub mod embed;
pub mod file;
pub mod health;
pub mod notification;
pub mod project;
pub mod session;
pub mod settings;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::compression::CompressionLayer;

pub fn build_router() -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let compression = CompressionLayer::new()
        .gzip(true)
        .quality(tower_http::CompressionLevel::Default);

    Router::new()
        .merge(auth::router())
        .merge(chat::router())
        .merge(session::router())
        .merge(project::router())
        .merge(notification::router())
        .merge(settings::router())
        .merge(file::router())
        .merge(dashboard::router())
        .merge(doc_export::router())
        .merge(health::router())
        .nest_service(
            "/outputs",
            ServeDir::new(crate::config::config().render_output_dir.clone()),
        )
        .fallback(embed::fallback_handler)
        .layer(compression)
        .layer(cors)
}
