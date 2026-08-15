use axum::extract::Path;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;

use crate::auth::middleware::AuthUser;
use crate::db::task_repo;
use crate::error::AppError;
use crate::state;

pub fn router() -> Router {
    Router::new()
        .route("/api/tasks", get(list_tasks).post(create_task))
        .route("/api/tasks/stats", get(task_stats))
        .route("/api/tasks/reorder", post(reorder_tasks))
        .route(
            "/api/tasks/:task_id",
            get(get_task).patch(update_task).delete(delete_task),
        )
}

async fn list_tasks(user: AuthUser) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let tasks = task_repo::list_by_owner(&pool, &user.0.id)?;
    Ok(Json(json!({ "tasks": tasks })))
}

async fn task_stats(user: AuthUser) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let stats = task_repo::stats(&pool, &user.0.id)?;
    Ok(Json(stats))
}

#[derive(Deserialize)]
struct CreateTaskReq {
    title: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    due_date: Option<String>,
    #[serde(default)]
    project_id: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
}

async fn create_task(
    user: AuthUser,
    Json(req): Json<CreateTaskReq>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let tags = req.tags.map(|t| t.join(","));
    let task = task_repo::create(
        &pool,
        &user.0.id,
        &req.title,
        req.description.as_deref(),
        req.priority.as_deref().unwrap_or("medium"),
        req.due_date.as_deref(),
        req.project_id.as_deref(),
        tags.as_deref(),
    )?;
    Ok(Json(json!(task)))
}

async fn get_task(
    user: AuthUser,
    Path(task_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let task =
        task_repo::find_by_id(&pool, &task_id)?.ok_or(AppError::NotFound("任务不存在".into()))?;
    if task.owner_id != user.0.id {
        return Err(AppError::Forbidden);
    }
    Ok(Json(json!(task)))
}

#[derive(Deserialize)]
struct UpdateTaskReq {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    due_date: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    order: Option<i64>,
}

async fn update_task(
    user: AuthUser,
    Path(task_id): Path<String>,
    Json(req): Json<UpdateTaskReq>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let tags = req.tags.map(|t| t.join(","));
    let task = task_repo::update(
        &pool,
        &task_id,
        &user.0.id,
        req.title.as_deref(),
        req.description.as_deref(),
        req.status.as_deref(),
        req.priority.as_deref(),
        req.due_date.as_deref(),
        tags.as_deref(),
        req.order,
    )?
    .ok_or(AppError::NotFound("任务不存在".into()))?;
    Ok(Json(json!(task)))
}

async fn delete_task(
    user: AuthUser,
    Path(task_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let deleted = task_repo::delete(&pool, &task_id, &user.0.id)?;
    Ok(Json(json!({ "deleted": deleted })))
}

#[derive(Deserialize)]
struct ReorderReq {
    orders: Vec<ReorderItem>,
}

#[derive(Deserialize)]
struct ReorderItem {
    id: String,
    order: i64,
}

async fn reorder_tasks(
    user: AuthUser,
    Json(req): Json<ReorderReq>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let orders: Vec<(String, i64)> = req.orders.into_iter().map(|o| (o.id, o.order)).collect();
    task_repo::reorder(&pool, &user.0.id, &orders)?;
    Ok(Json(json!({ "ok": true })))
}
