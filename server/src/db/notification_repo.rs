use rusqlite::params;
use serde::{Deserialize, Serialize};
use crate::error::AppResult;
use super::DbPool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRow {
    pub id: String,
    pub user_id: String,
    #[serde(rename = "type")]
    pub notif_type: String,
    pub title: String,
    pub content: Option<String>,
    pub is_read: bool,
    pub link: Option<String>,
    pub created_at: String,
}

pub fn list(pool: &DbPool, user_id: &str, unread_only: bool, limit: i64) -> AppResult<Vec<NotificationRow>> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let sql = if unread_only {
        "SELECT id, user_id, type, title, content, is_read, link, created_at FROM notifications WHERE user_id = ?1 AND is_read = 0 ORDER BY created_at DESC LIMIT ?2"
    } else {
        "SELECT id, user_id, type, title, content, is_read, link, created_at FROM notifications WHERE user_id = ?1 ORDER BY created_at DESC LIMIT ?2"
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![user_id, limit], |row| Ok(NotificationRow {
        id: row.get(0)?, user_id: row.get(1)?, notif_type: row.get(2)?, title: row.get(3)?,
        content: row.get(4)?, is_read: row.get::<_, i64>(5)? != 0, link: row.get(6)?, created_at: row.get(7)?,
    }))?;
    let mut result = Vec::new();
    for row in rows { result.push(row?); }
    Ok(result)
}

pub fn unread_count(pool: &DbPool, user_id: &str) -> AppResult<i64> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM notifications WHERE user_id = ?1 AND is_read = 0",
        params![user_id], |r| r.get(0),
    )?;
    Ok(count)
}

pub fn mark_as_read(pool: &DbPool, id: &str, user_id: &str) -> AppResult<bool> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let affected = conn.execute(
        "UPDATE notifications SET is_read = 1 WHERE id = ?1 AND user_id = ?2",
        params![id, user_id],
    )?;
    Ok(affected > 0)
}

pub fn mark_all_as_read(pool: &DbPool, user_id: &str) -> AppResult<()> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    conn.execute("UPDATE notifications SET is_read = 1 WHERE user_id = ?1", params![user_id])?;
    Ok(())
}

pub fn delete(pool: &DbPool, id: &str, user_id: &str) -> AppResult<bool> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let affected = conn.execute(
        "DELETE FROM notifications WHERE id = ?1 AND user_id = ?2",
        params![id, user_id],
    )?;
    Ok(affected > 0)
}
