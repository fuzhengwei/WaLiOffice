use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;

use crate::error::AppError;
use crate::auth::middleware::AuthUser;
use crate::state;
use crate::db::{notification_repo, project_repo, session_repo, task_repo};

pub fn router() -> Router {
    Router::new().route("/api/dashboard/stats", get(dashboard_stats))
}

async fn dashboard_stats(user: AuthUser) -> Result<Json<serde_json::Value>, AppError> {
    let pool = state::db_pool();
    let projects = project_repo::list_by_owner(&pool, &user.0.id)?;
    let sessions = session_repo::list_by_owner(&pool, &user.0.id, 100)?;
    let task_stats = task_repo::stats(&pool, &user.0.id)?;
    let ppt_projects = project_repo::list_ppt_projects(Some(&user.0.id))?;
    let unread_notifications = notification_repo::unread_count(&pool, &user.0.id)?;

    let mut by_kind = serde_json::Map::new();
    for project in &projects {
        let count = by_kind
            .entry(project.tool_kind.clone())
            .or_insert_with(|| json!(0));
        *count = json!(count.as_i64().unwrap_or(0) + 1);
    }
    if !ppt_projects.is_empty() {
        let ppt_count = by_kind.entry("ppt".to_string()).or_insert_with(|| json!(0));
        *ppt_count = json!(ppt_count.as_i64().unwrap_or(0) + ppt_projects.len() as i64);
    }

    let recent_projects: Vec<_> = projects.iter().take(5).map(|project| {
        json!({
            "id": project.id,
            "title": project.title,
            "description": project.description,
            "tool_kind": project.tool_kind,
            "created_at": project.created_at,
            "updated_at": project.updated_at,
        })
    }).collect();

    Ok(Json(json!({
        "tasks": task_stats,
        "projects": {
            "total": projects.len() + ppt_projects.len(),
            "by_kind": by_kind,
        },
        "files": {
            "total": 0,
            "by_type": {},
            "total_size": 0,
        },
        "notifications": {
            "unread": unread_notifications,
        },
        "recent_sessions": sessions.into_iter().take(5).map(|session| {
            json!({
                "id": session.id,
                "project_id": session.project_id,
                "tool_kind": session.tool_kind,
                "title": session.title,
                "summary": session.summary,
                "message_count": session.message_count,
                "created_at": session.created_at,
                "updated_at": session.updated_at,
            })
        }).collect::<Vec<_>>(),
        "recent_projects": recent_projects,
    })))
}
