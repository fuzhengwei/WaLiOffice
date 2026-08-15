use crate::models::PptProject;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use crate::error::AppResult;
use super::DbPool;

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

pub fn create(pool: &DbPool, title: &str, tool_kind: &str, owner_id: &str) -> AppResult<ProjectRow> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO projects (id, title, tool_kind, owner_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, title, tool_kind, owner_id, &now, &now],
    )?;
    Ok(ProjectRow {
        id, title: title.to_string(), description: None,
        tool_kind: tool_kind.to_string(), owner_id: owner_id.to_string(),
        created_at: now.clone(), updated_at: now,
    })
}

pub fn list_by_owner(pool: &DbPool, owner_id: &str, query: Option<&str>) -> AppResult<Vec<ProjectRow>> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let q = query.map(|item| format!("%{}%", item.trim())).filter(|item| item != "%%");
    let mut result = Vec::new();
    if let Some(ref qv) = q {
        let mut stmt = conn.prepare(
            "SELECT id, title, description, tool_kind, owner_id, created_at, updated_at
             FROM projects
             WHERE owner_id = ?1 AND (title LIKE ?2 OR COALESCE(description, '') LIKE ?2)
             ORDER BY updated_at DESC"
        )?;
        let rows = stmt.query_map(params![owner_id, qv], |row| Ok(ProjectRow {
            id: row.get(0)?, title: row.get(1)?, description: row.get(2)?, tool_kind: row.get(3)?,
            owner_id: row.get(4)?, created_at: row.get(5)?, updated_at: row.get(6)?,
        }))?;
        for row in rows {
            result.push(row?);
        }
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, title, description, tool_kind, owner_id, created_at, updated_at
             FROM projects WHERE owner_id = ?1 ORDER BY updated_at DESC"
        )?;
        let rows = stmt.query_map(params![owner_id], |row| Ok(ProjectRow {
            id: row.get(0)?, title: row.get(1)?, description: row.get(2)?, tool_kind: row.get(3)?,
            owner_id: row.get(4)?, created_at: row.get(5)?, updated_at: row.get(6)?,
        }))?;
        for row in rows {
            result.push(row?);
        }
    }
    Ok(result)
}

pub fn find_by_id(pool: &DbPool, id: &str, owner_id: &str) -> AppResult<Option<ProjectRow>> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let row = conn.query_row(
        "SELECT id, title, description, tool_kind, owner_id, created_at, updated_at
         FROM projects WHERE id = ?1 AND owner_id = ?2",
        params![id, owner_id],
        |row| Ok(ProjectRow {
            id: row.get(0)?,
            title: row.get(1)?,
            description: row.get(2)?,
            tool_kind: row.get(3)?,
            owner_id: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        }),
    );
    match row {
        Ok(project) => Ok(Some(project)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn update(
    pool: &DbPool,
    id: &str,
    owner_id: &str,
    title: Option<&str>,
    description: Option<Option<&str>>,
    tool_kind: Option<&str>,
) -> AppResult<Option<ProjectRow>> {
    let current = match find_by_id(pool, id, owner_id)? {
        Some(project) => project,
        None => return Ok(None),
    };

    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let now = chrono::Utc::now().to_rfc3339();
    let next_title = title.unwrap_or(&current.title);
    let next_description = description
        .map(|value| value.map(|item| item.to_string()))
        .unwrap_or(current.description.clone());
    let next_tool_kind = tool_kind.unwrap_or(&current.tool_kind);

    conn.execute(
        "UPDATE projects
         SET title = ?1, description = ?2, tool_kind = ?3, updated_at = ?4
         WHERE id = ?5 AND owner_id = ?6",
        params![next_title, next_description, next_tool_kind, &now, id, owner_id],
    )?;

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

pub fn delete(pool: &DbPool, id: &str, owner_id: &str) -> AppResult<bool> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let affected = conn.execute(
        "DELETE FROM projects WHERE id = ?1 AND owner_id = ?2",
        params![id, owner_id],
    )?;
    Ok(affected > 0)
}

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
    if !path.exists() { return Ok(None); }
    let json = fs::read_to_string(path)?;
    Ok(Some(serde_json::from_str(&json)?))
}

pub fn list_ppt_projects(owner_id: Option<&str>) -> AppResult<Vec<PptProject>> {
    let dir = &crate::config::config().projects_dir;
    if !std::path::Path::new(dir).exists() { return Ok(vec![]); }
    let mut projects = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") { continue; }
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
