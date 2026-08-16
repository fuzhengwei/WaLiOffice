use super::DbPool;
use crate::error::AppResult;
use crate::models::PptProject;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRow {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub tool_kind: String,
    pub owner_id: String,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn create(
    pool: &DbPool,
    title: &str,
    tool_kind: &str,
    owner_id: &str,
) -> AppResult<ProjectRow> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO projects (id, title, tool_kind, owner_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(title)
    .bind(tool_kind)
    .bind(owner_id)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(ProjectRow {
        id,
        title: title.to_string(),
        description: None,
        tool_kind: tool_kind.to_string(),
        owner_id: owner_id.to_string(),
        created_at: now.clone(),
        updated_at: now,
    })
}

pub async fn list_by_owner(
    pool: &DbPool,
    owner_id: &str,
    query: Option<&str>,
) -> AppResult<Vec<ProjectRow>> {
    let q = query
        .map(|item| format!("%{}%", item.trim()))
        .filter(|item| item != "%%");

    let rows = if let Some(ref qv) = q {
        sqlx::query(
            "SELECT id, title, description, tool_kind, owner_id, created_at, updated_at
             FROM projects
             WHERE owner_id = ? AND (title LIKE ? OR COALESCE(description, '') LIKE ?)
             ORDER BY updated_at DESC"
        )
        .bind(owner_id)
        .bind(qv)
        .bind(qv)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT id, title, description, tool_kind, owner_id, created_at, updated_at
             FROM projects WHERE owner_id = ? ORDER BY updated_at DESC"
        )
        .bind(owner_id)
        .fetch_all(pool)
        .await?
    };

    let mut result = Vec::new();
    for row in rows {
        result.push(ProjectRow {
            id: row.try_get(0)?,
            title: row.try_get(1)?,
            description: row.try_get(2)?,
            tool_kind: row.try_get(3)?,
            owner_id: row.try_get(4)?,
            created_at: row.try_get(5)?,
            updated_at: row.try_get(6)?,
        });
    }
    Ok(result)
}

pub async fn find_by_id(pool: &DbPool, id: &str, owner_id: &str) -> AppResult<Option<ProjectRow>> {
    let row = sqlx::query(
        "SELECT id, title, description, tool_kind, owner_id, created_at, updated_at
         FROM projects WHERE id = ? AND owner_id = ?"
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(Some(ProjectRow {
            id: r.try_get(0)?,
            title: r.try_get(1)?,
            description: r.try_get(2)?,
            tool_kind: r.try_get(3)?,
            owner_id: r.try_get(4)?,
            created_at: r.try_get(5)?,
            updated_at: r.try_get(6)?,
        })),
        None => Ok(None),
    }
}

pub async fn update(
    pool: &DbPool,
    id: &str,
    owner_id: &str,
    title: Option<&str>,
    description: Option<Option<&str>>,
    tool_kind: Option<&str>,
) -> AppResult<Option<ProjectRow>> {
    let current = match find_by_id(pool, id, owner_id).await? {
        Some(project) => project,
        None => return Ok(None),
    };

    let now = chrono::Utc::now().to_rfc3339();
    let next_title = title.unwrap_or(&current.title);
    let next_description = description
        .map(|value| value.map(|item| item.to_string()))
        .unwrap_or(current.description.clone());
    let next_tool_kind = tool_kind.unwrap_or(&current.tool_kind);

    sqlx::query(
        "UPDATE projects
         SET title = ?, description = ?, tool_kind = ?, updated_at = ?
         WHERE id = ? AND owner_id = ?"
    )
    .bind(next_title)
    .bind(&next_description)
    .bind(next_tool_kind)
    .bind(&now)
    .bind(id)
    .bind(owner_id)
    .execute(pool)
    .await?;

    Ok(Some(ProjectRow {
        id: current.id,
        title: next_title.to_string(),
        description: next_description,
        tool_kind: next_tool_kind.to_string(),
        owner_id: current.owner_id,
        created_at: current.created_at,
        updated_at: now,
    }))
}

pub async fn delete(pool: &DbPool, id: &str, owner_id: &str) -> AppResult<bool> {
    let result = sqlx::query(
        "DELETE FROM projects WHERE id = ? AND owner_id = ?"
    )
    .bind(id)
    .bind(owner_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

// ── PPT Project file-based storage (unchanged) ──

fn project_path(project_id: &str) -> PathBuf {
    PathBuf::from(&crate::config::config().projects_dir).join(format!("{project_id}.json"))
}

pub fn save_ppt_project(project: &PptProject) -> AppResult<()> {
    let dir = &crate::config::config().projects_dir;
    fs::create_dir_all(dir)?;
    let path = project_path(&project.id);
    let json = serde_json::to_string_pretty(project)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_ppt_project(project_id: &str) -> AppResult<Option<PptProject>> {
    let path = project_path(project_id);
    if !path.exists() {
        return Ok(None);
    }
    let json = fs::read_to_string(path)?;
    Ok(Some(serde_json::from_str(&json)?))
}

pub fn list_ppt_projects(owner_id: Option<&str>) -> AppResult<Vec<PptProject>> {
    let dir = &crate::config::config().projects_dir;
    if !std::path::Path::new(dir).exists() {
        return Ok(vec![]);
    }
    let mut projects = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if let Ok(json) = fs::read_to_string(&path) {
            if let Ok(proj) = serde_json::from_str::<PptProject>(&json) {
                if owner_id.map_or(true, |oid| proj.owner_id == oid) {
                    projects.push(proj);
                }
            }
        }
    }
    projects.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(projects)
}

pub fn delete_ppt_project(project_id: &str) -> AppResult<bool> {
    let path = project_path(project_id);
    if path.exists() {
        fs::remove_file(path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}
