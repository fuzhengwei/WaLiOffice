use axum::extract::Path;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde_json::json;

use crate::error::AppError;
use crate::auth::middleware::AuthUser;
use crate::state;
use crate::db::session_repo;

pub fn router() -> Router {
    Router::new()
        .route("/api/chat/sessions", get(list_sessions))
        .route("/api/chat/session/:session_id", get(get_session).delete(delete_session))
        .route("/api/chat/session/:session_id/messages", get(get_messages))
        .route("/api/chat/session/:session_id/clear", post(clear_session))
}

async fn list_sessions(user: AuthUser) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let sessions = session_repo::list_by_owner(&pool, &user.0.id, 50)?;
    Ok(Json(json!({ "sessions": sessions })))
}

async fn get_session(user: AuthUser, Path(session_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let session = session_repo::find_by_id(&pool, &session_id)?
        .ok_or(AppError::NotFound("会话不存在".into()))?;
    if session.owner_id != user.0.id {
        return Err(AppError::Forbidden);
    }
    Ok(Json(json!(session)))
}

async fn get_messages(user: AuthUser, Path(session_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let session = session_repo::find_by_id(&pool, &session_id)?
        .ok_or(AppError::NotFound("会话不存在".into()))?;
    if session.owner_id != user.0.id {
        return Err(AppError::Forbidden);
    }
    let messages = session_repo::get_messages(&pool, &session_id, 100)?;
    Ok(Json(json!({ "messages": messages })))
}

async fn delete_session(user: AuthUser, Path(session_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let deleted = session_repo::delete(&pool, &session_id, &user.0.id)?;
    Ok(Json(json!({ "deleted": deleted })))
}

async fn clear_session(user: AuthUser, Path(session_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let cleared = session_repo::clear_messages(&pool, &session_id, &user.0.id)?;
    Ok(Json(json!({ "cleared": cleared })))
}
