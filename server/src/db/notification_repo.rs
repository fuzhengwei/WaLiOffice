use super::DbPool;
use crate::error::AppResult;
use serde::{Deserialize, Serialize};
use sqlx::Row;

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

pub async fn list(
    pool: &DbPool,
    user_id: &str,
    unread_only: bool,
    limit: i64,
) -> AppResult<Vec<NotificationRow>> {
    let sql = if unread_only {
        "SELECT id, user_id, type, title, content, is_read, link, created_at FROM notifications WHERE user_id = ? AND is_read = 0 ORDER BY created_at DESC LIMIT ?"
    } else {
        "SELECT id, user_id, type, title, content, is_read, link, created_at FROM notifications WHERE user_id = ? ORDER BY created_at DESC LIMIT ?"
    };
    let rows = sqlx::query(sql)
        .bind(user_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

    let mut result = Vec::new();
    for row in rows {
        let is_read_val: i64 = row.try_get(5)?;
        result.push(NotificationRow {
            id: row.try_get(0)?,
            user_id: row.try_get(1)?,
            notif_type: row.try_get(2)?,
            title: row.try_get(3)?,
            content: row.try_get(4)?,
            is_read: is_read_val != 0,
            link: row.try_get(6)?,
            created_at: row.try_get(7)?,
        });
    }
    Ok(result)
}

pub async fn unread_count(pool: &DbPool, user_id: &str) -> AppResult<i64> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications WHERE user_id = ? AND is_read = 0"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(count)
}

pub async fn mark_as_read(pool: &DbPool, id: &str, user_id: &str) -> AppResult<bool> {
    let result = sqlx::query(
        "UPDATE notifications SET is_read = 1 WHERE id = ? AND user_id = ?"
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn mark_all_as_read(pool: &DbPool, user_id: &str) -> AppResult<()> {
    sqlx::query("UPDATE notifications SET is_read = 1 WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete(pool: &DbPool, id: &str, user_id: &str) -> AppResult<bool> {
    let result = sqlx::query(
        "DELETE FROM notifications WHERE id = ? AND user_id = ?"
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}
