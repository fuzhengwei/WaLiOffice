use super::DbPool;
use crate::error::AppResult;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRow {
    pub id: String,
    pub owner_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub due_date: Option<String>,
    pub project_id: Option<String>,
    pub tags: Option<String>,
    pub order_col: i64,
    pub created_at: String,
    pub updated_at: String,
}

pub fn list_by_owner(pool: &DbPool, owner_id: &str) -> AppResult<Vec<TaskRow>> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let mut stmt = conn.prepare(
        "SELECT id, owner_id, title, description, status, priority, due_date, project_id, tags, order_col, created_at, updated_at
         FROM tasks WHERE owner_id = ?1 ORDER BY order_col ASC, created_at DESC",
    )?;
    let rows = stmt.query_map(params![owner_id], |row| {
        Ok(TaskRow {
            id: row.get(0)?,
            owner_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            status: row.get(4)?,
            priority: row.get(5)?,
            due_date: row.get(6)?,
            project_id: row.get(7)?,
            tags: row.get(8)?,
            order_col: row.get(9)?,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn stats(pool: &DbPool, owner_id: &str) -> AppResult<serde_json::Value> {
    let tasks = list_by_owner(pool, owner_id)?;
    let mut by_status = serde_json::Map::from_iter([
        ("todo".to_string(), serde_json::json!(0)),
        ("in_progress".to_string(), serde_json::json!(0)),
        ("done".to_string(), serde_json::json!(0)),
        ("archived".to_string(), serde_json::json!(0)),
    ]);
    let mut by_priority = serde_json::Map::from_iter([
        ("low".to_string(), serde_json::json!(0)),
        ("medium".to_string(), serde_json::json!(0)),
        ("high".to_string(), serde_json::json!(0)),
        ("urgent".to_string(), serde_json::json!(0)),
    ]);

    let today = chrono::Utc::now().date_naive();
    let due_limit = today + chrono::Days::new(3);
    let mut due_soon = 0_i64;

    for task in &tasks {
        let status_count = by_status
            .entry(task.status.clone())
            .or_insert_with(|| serde_json::json!(0));
        *status_count = serde_json::json!(status_count.as_i64().unwrap_or(0) + 1);

        let priority_count = by_priority
            .entry(task.priority.clone())
            .or_insert_with(|| serde_json::json!(0));
        *priority_count = serde_json::json!(priority_count.as_i64().unwrap_or(0) + 1);

        if task.status != "done" && task.status != "archived" {
            if let Some(due_date) = task.due_date.as_deref() {
                let parsed_due = chrono::DateTime::parse_from_rfc3339(due_date)
                    .map(|dt| dt.date_naive())
                    .or_else(|_| chrono::NaiveDate::parse_from_str(due_date, "%Y-%m-%d"));
                if let Ok(date) = parsed_due {
                    if date >= today && date <= due_limit {
                        due_soon += 1;
                    }
                }
            }
        }
    }

    Ok(serde_json::json!({
        "total": tasks.len(),
        "by_status": by_status,
        "by_priority": by_priority,
        "due_soon": due_soon,
    }))
}

pub fn create(
    pool: &DbPool,
    owner_id: &str,
    title: &str,
    description: Option<&str>,
    priority: &str,
    due_date: Option<&str>,
    project_id: Option<&str>,
    tags: Option<&str>,
) -> AppResult<TaskRow> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let max_order: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(order_col), 0) FROM tasks WHERE owner_id = ?1",
            params![owner_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    conn.execute(
        "INSERT INTO tasks (id, owner_id, title, description, status, priority, due_date, project_id, tags, order_col, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'todo', ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![id, owner_id, title, description, priority, due_date, project_id, tags, max_order + 1, &now, &now],
    )?;
    Ok(TaskRow {
        id,
        owner_id: owner_id.to_string(),
        title: title.to_string(),
        description: description.map(String::from),
        status: "todo".into(),
        priority: priority.to_string(),
        due_date: due_date.map(String::from),
        project_id: project_id.map(String::from),
        tags: tags.map(String::from),
        order_col: max_order + 1,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub fn update(
    pool: &DbPool,
    id: &str,
    owner_id: &str,
    title: Option<&str>,
    description: Option<&str>,
    status: Option<&str>,
    priority: Option<&str>,
    due_date: Option<&str>,
    tags: Option<&str>,
    order: Option<i64>,
) -> AppResult<Option<TaskRow>> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let now = chrono::Utc::now().to_rfc3339();
    let mut updates: Vec<String> = vec!["updated_at = ?1".to_string()];
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(now.clone())];
    let mut idx = 2;
    if let Some(v) = title {
        updates.push(format!("title = ?{idx}"));
        params_vec.push(Box::new(v.to_string()));
        idx += 1;
    }
    if let Some(v) = description {
        updates.push(format!("description = ?{idx}"));
        params_vec.push(Box::new(v.to_string()));
        idx += 1;
    }
    if let Some(v) = status {
        updates.push(format!("status = ?{idx}"));
        params_vec.push(Box::new(v.to_string()));
        idx += 1;
    }
    if let Some(v) = priority {
        updates.push(format!("priority = ?{idx}"));
        params_vec.push(Box::new(v.to_string()));
        idx += 1;
    }
    if let Some(v) = due_date {
        updates.push(format!("due_date = ?{idx}"));
        params_vec.push(Box::new(v.to_string()));
        idx += 1;
    }
    if let Some(v) = tags {
        updates.push(format!("tags = ?{idx}"));
        params_vec.push(Box::new(v.to_string()));
        idx += 1;
    }
    if let Some(v) = order {
        updates.push(format!("order_col = ?{idx}"));
        params_vec.push(Box::new(v.to_string()));
        idx += 1;
    }
    updates.push(format!("owner_id = ?{idx}"));
    params_vec.push(Box::new(owner_id.to_string()));
    idx += 1;
    let sql = format!(
        "UPDATE tasks SET {} WHERE id = ?{}",
        updates.join(", "),
        idx
    );
    params_vec.push(Box::new(id.to_string()));
    let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let affected = conn.execute(&sql, param_refs.as_slice())?;
    if affected == 0 {
        return Ok(None);
    }
    find_by_id(pool, id)
}

pub fn find_by_id(pool: &DbPool, id: &str) -> AppResult<Option<TaskRow>> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let row = conn.query_row(
        "SELECT id, owner_id, title, description, status, priority, due_date, project_id, tags, order_col, created_at, updated_at
         FROM tasks WHERE id = ?1",
        params![id],
        |row| Ok(TaskRow {
            id: row.get(0)?, owner_id: row.get(1)?, title: row.get(2)?, description: row.get(3)?,
            status: row.get(4)?, priority: row.get(5)?, due_date: row.get(6)?, project_id: row.get(7)?,
            tags: row.get(8)?, order_col: row.get(9)?, created_at: row.get(10)?, updated_at: row.get(11)?,
        }),
    );
    match row {
        Ok(t) => Ok(Some(t)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn delete(pool: &DbPool, id: &str, owner_id: &str) -> AppResult<bool> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let affected = conn.execute(
        "DELETE FROM tasks WHERE id = ?1 AND owner_id = ?2",
        params![id, owner_id],
    )?;
    Ok(affected > 0)
}

pub fn reorder(pool: &DbPool, owner_id: &str, orders: &[(String, i64)]) -> AppResult<()> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let now = chrono::Utc::now().to_rfc3339();
    for (id, order) in orders {
        conn.execute(
            "UPDATE tasks SET order_col = ?1, updated_at = ?2 WHERE id = ?3 AND owner_id = ?4",
            params![order, &now, id, owner_id],
        )?;
    }
    Ok(())
}
