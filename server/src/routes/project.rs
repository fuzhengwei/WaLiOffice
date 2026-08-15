use axum::extract::Path;
use axum::extract::Query;
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;

use crate::auth::middleware::AuthUser;
use crate::db::{project_repo, session_repo};
use crate::error::AppError;
use crate::state;

pub fn router() -> Router {
    Router::new()
        .route("/api/projects", get(list_projects).post(create_project))
        .route("/api/projects/:project_id", get(get_project).patch(update_project).delete(delete_project))
        .route("/api/projects/:project_id/sessions", get(get_project_sessions))
        .route("/api/ppt/projects", get(list_ppt_projects))
        .route("/api/ppt/project/:project_id", get(get_ppt_project))
        .route("/api/ppt/project", post(create_ppt_project))
        .route("/api/ppt/project/:project_id/delete", post(delete_ppt_project))
}

#[derive(Deserialize)]
struct ProjectListQuery {
    q: Option<String>,
}

async fn list_projects(user: AuthUser, Query(query): Query<ProjectListQuery>) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let projects = project_repo::list_by_owner(&pool, &user.0.id, query.q.as_deref())?;
    Ok(Json(json!({ "projects": projects })))
}

#[derive(Deserialize)]
struct CreateGenericProjectReq {
    title: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tool_kind: Option<String>,
}

async fn create_project(user: AuthUser, Json(req): Json<CreateGenericProjectReq>) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let project = project_repo::create(&pool, &req.title, req.tool_kind.as_deref().unwrap_or("general"), &user.0.id)?;
    if req.description.is_some() {
        let updated = project_repo::update(
            &pool,
            &project.id,
            &user.0.id,
            None,
            Some(req.description.as_deref()),
            None,
        )?;
        return Ok(Json(json!(updated.unwrap_or(project))));
    }
    Ok(Json(json!(project)))
}

async fn get_project(user: AuthUser, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let project = project_repo::find_by_id(&pool, &project_id, &user.0.id)?
        .ok_or(AppError::NotFound("项目不存在".into()))?;
    Ok(Json(json!(project)))
}

#[derive(Deserialize)]
struct UpdateProjectReq {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tool_kind: Option<String>,
}

async fn update_project(
    user: AuthUser,
    Path(project_id): Path<String>,
    Json(req): Json<UpdateProjectReq>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let project = project_repo::update(
        &pool,
        &project_id,
        &user.0.id,
        req.title.as_deref(),
        Some(req.description.as_deref()),
        req.tool_kind.as_deref(),
    )?
    .ok_or(AppError::NotFound("项目不存在".into()))?;
    Ok(Json(json!(project)))
}

async fn delete_project(user: AuthUser, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let deleted = project_repo::delete(&pool, &project_id, &user.0.id)?;
    Ok(Json(json!({ "deleted": deleted })))
}

async fn get_project_sessions(user: AuthUser, Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let project = project_repo::find_by_id(&pool, &project_id, &user.0.id)?
        .ok_or(AppError::NotFound("项目不存在".into()))?;
    let sessions = session_repo::list_by_owner(&pool, &user.0.id, 100, None)?
        .into_iter()
        .filter(|item| item.project_id.as_deref() == Some(project.id.as_str()))
        .collect::<Vec<_>>();
    Ok(Json(json!({ "sessions": sessions })))
}

async fn list_ppt_projects(user: AuthUser) -> Result<Json<serde_json::Value>, AppError> {
    let projects = project_repo::list_ppt_projects(Some(&user.0.id))?;
    Ok(Json(json!({ "projects": projects })))
}

async fn get_ppt_project(Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let project = project_repo::load_ppt_project(&project_id)?
        .ok_or(AppError::NotFound("项目不存在".into()))?;
    Ok(Json(json!(project)))
}

#[derive(Deserialize)]
struct CreatePptProjectReq {
    title: String,
    #[serde(default)]
    theme: Option<String>,
}

async fn create_ppt_project(user: AuthUser, Json(req): Json<CreatePptProjectReq>) -> Result<Json<serde_json::Value>, AppError> {
    use crate::models::PptProject;
    let now = chrono::Utc::now().to_rfc3339();
    let project = PptProject {
        id: uuid::Uuid::new_v4().to_string(),
        title: req.title,
        theme: req.theme.unwrap_or_else(|| "default".into()),
        slides: vec![],
        history: Some(vec![]),
        layout: "16x9".into(),
        created_at: now.clone(),
        updated_at: now,
        owner_id: user.0.id,
    };
    project_repo::save_ppt_project(&project)?;
    Ok(Json(json!(project)))
}

async fn delete_ppt_project(Path(project_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    let deleted = project_repo::delete_ppt_project(&project_id)?;
    Ok(Json(json!({ "deleted": deleted })))
}
