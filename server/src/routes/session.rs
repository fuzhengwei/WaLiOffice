use axum::extract::Path;
use axum::extract::Query;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::middleware::AuthUser;
use crate::db::session_repo;
use crate::error::AppError;
use crate::state;

pub fn router() -> Router {
    Router::new()
        .route("/api/chat/sessions", get(list_sessions))
        .route(
            "/api/chat/session/:session_id",
            get(get_session)
                .patch(update_session)
                .delete(delete_session),
        )
        .route("/api/chat/session/:session_id/messages", get(get_messages))
        .route("/api/chat/session/:session_id/clear", post(clear_session))
}

#[derive(Deserialize)]
struct SessionListQuery {
    q: Option<String>,
}

#[derive(Deserialize)]
struct UpdateSessionPayload {
    title: Option<String>,
    project_id: Option<Value>,
    order_col: Option<i64>,
}

async fn list_sessions(
    user: AuthUser,
    Query(query): Query<SessionListQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let sessions = session_repo::list_by_owner(&pool, &user.0.id, 50, query.q.as_deref()).await?;
    Ok(Json(json!({ "sessions": sessions })))
}

async fn get_session(
    user: AuthUser,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let session = session_repo::get_session_detail(&pool, &session_id)
        .await?
        .ok_or(AppError::NotFound("会话不存在".into()))?;
    if session.owner_id != user.0.id {
        return Err(AppError::Forbidden);
    }
    Ok(Json(json!(session)))
}

async fn get_messages(
    user: AuthUser,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let session = session_repo::find_by_id(&pool, &session_id)
        .await?
        .ok_or(AppError::NotFound("会话不存在".into()))?;
    if session.owner_id != user.0.id {
        return Err(AppError::Forbidden);
    }
    let messages = session_repo::get_messages(&pool, &session_id, 100).await?;
    Ok(Json(json!({ "messages": messages })))
}

async fn update_session(
    user: AuthUser,
    Path(session_id): Path<String>,
    Json(payload): Json<UpdateSessionPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let mut updated = false;

    if let Some(title) = payload.title {
        let title = title.trim().to_string();
        if title.is_empty() {
            return Err(AppError::BadRequest("标题不能为空".into()));
        }
        updated |= session_repo::update_title(&pool, &session_id, &user.0.id, &title).await?;
    }

    if payload.project_id.is_some() || payload.order_col.is_some() {
        let session = session_repo::find_by_id(&pool, &session_id)
            .await?
            .ok_or(AppError::NotFound("会话不存在".into()))?;
        if session.owner_id != user.0.id {
            return Err(AppError::Forbidden);
        }
        let project_id_owned = match payload.project_id {
            Some(Value::Null) => None,
            Some(Value::String(value)) => {
                Some(value.trim().to_string()).filter(|value| !value.is_empty())
            }
            Some(_) => return Err(AppError::BadRequest("项目 ID 格式不正确".into())),
            None => session.project_id,
        };
        let order_col = payload.order_col.unwrap_or(session.order_col);
        updated |= session_repo::update_project_and_order(
            &pool,
            &session_id,
            &user.0.id,
            project_id_owned.as_deref(),
            order_col,
        ).await?;
    }

    Ok(Json(json!({ "updated": updated })))
}

async fn delete_session(
    user: AuthUser,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let deleted = session_repo::delete(&pool, &session_id, &user.0.id).await?;
    Ok(Json(json!({ "deleted": deleted })))
}

async fn clear_session(
    user: AuthUser,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let cleared = session_repo::clear_messages(&pool, &session_id, &user.0.id).await?;
    Ok(Json(json!({ "cleared": cleared })))
}
