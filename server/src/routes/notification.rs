use axum::extract::{Path, Query};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;

use crate::auth::middleware::AuthUser;
use crate::db::notification_repo;
use crate::error::AppError;
use crate::state;

pub fn router() -> Router {
    Router::new()
        .route("/api/notifications", get(list_notifications))
        .route("/api/notifications/unread", get(unread_count))
        .route("/api/notifications/:id/read", post(mark_as_read))
        .route("/api/notifications/read-all", post(mark_all_as_read))
        .route("/api/notifications/:id", delete(delete_notification))
}

#[derive(Deserialize)]
struct NotifQuery {
    #[serde(default)]
    unread_only: Option<bool>,
    #[serde(default)]
    page: Option<u32>,
    #[serde(default)]
    page_size: Option<u32>,
}

async fn list_notifications(
    user: AuthUser,
    Query(q): Query<NotifQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let unread = q.unread_only.unwrap_or(false);
    let limit = q.page_size.unwrap_or(50) as i64;
    let notifications = notification_repo::list(&pool, &user.0.id, unread, limit).await?;
    Ok(Json(json!({ "notifications": notifications })))
}

async fn unread_count(user: AuthUser) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let count = notification_repo::unread_count(&pool, &user.0.id).await?;
    Ok(Json(json!({ "count": count })))
}

async fn mark_as_read(
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let ok = notification_repo::mark_as_read(&pool, &id, &user.0.id).await?;
    Ok(Json(json!({ "ok": ok })))
}

async fn mark_all_as_read(user: AuthUser) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    notification_repo::mark_all_as_read(&pool, &user.0.id).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn delete_notification(
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let deleted = notification_repo::delete(&pool, &id, &user.0.id).await?;
    Ok(Json(json!({ "deleted": deleted })))
}
