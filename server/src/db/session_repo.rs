use super::DbPool;
use crate::error::AppResult;
use crate::models::{Artifact, ChatMessage, PersistedChatMessage};
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRow {
    pub id: String,
    pub owner_id: String,
    pub project_id: Option<String>,
    pub tool_kind: Option<String>,
    pub title: String,
    pub summary: Option<String>,
    pub message_count: i64,
    pub order_col: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDetail {
    pub id: String,
    pub owner_id: String,
    pub project_id: Option<String>,
    pub tool_kind: Option<String>,
    pub title: String,
    pub summary: Option<String>,
    pub message_count: i64,
    pub order_col: i64,
    pub created_at: String,
    pub updated_at: String,
    pub messages: Vec<PersistedChatMessage>,
    pub artifacts: Vec<Artifact>,
}

pub async fn create(
    pool: &DbPool,
    owner_id: &str,
    project_id: Option<&str>,
    tool_kind: Option<&str>,
    title: &str,
) -> AppResult<SessionRow> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let order_val = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO sessions (id, owner_id, project_id, tool_kind, title, message_count, order_col, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, 0, ?, ?, ?)"
    )
    .bind(&id)
    .bind(owner_id)
    .bind(project_id)
    .bind(tool_kind)
    .bind(title)
    .bind(order_val)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(SessionRow {
        id,
        owner_id: owner_id.to_string(),
        project_id: project_id.map(String::from),
        tool_kind: tool_kind.map(String::from),
        title: title.to_string(),
        summary: None,
        message_count: 0,
        order_col: order_val,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub async fn find_by_id(pool: &DbPool, id: &str) -> AppResult<Option<SessionRow>> {
    let row = sqlx::query(
        "SELECT id, owner_id, project_id, tool_kind, title, summary, message_count, order_col, created_at, updated_at
         FROM sessions WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(Some(SessionRow {
            id: r.try_get(0)?,
            owner_id: r.try_get(1)?,
            project_id: r.try_get(2)?,
            tool_kind: r.try_get(3)?,
            title: r.try_get(4)?,
            summary: r.try_get(5)?,
            message_count: r.try_get(6)?,
            order_col: r.try_get(7)?,
            created_at: r.try_get(8)?,
            updated_at: r.try_get(9)?,
        })),
        None => Ok(None),
    }
}

pub async fn list_by_owner(
    pool: &DbPool,
    owner_id: &str,
    limit: i64,
    query: Option<&str>,
) -> AppResult<Vec<SessionRow>> {
    let q = query
        .map(|item| format!("%{}%", item.trim()))
        .filter(|item| item != "%%");

    let rows = if let Some(ref qv) = q {
        sqlx::query(
            "SELECT id, owner_id, project_id, tool_kind, title, summary, message_count, order_col, created_at, updated_at
             FROM sessions
             WHERE owner_id = ? AND (title LIKE ? OR COALESCE(summary, '') LIKE ?)
             ORDER BY order_col ASC, updated_at DESC LIMIT ?"
        )
        .bind(owner_id)
        .bind(qv)
        .bind(qv)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT id, owner_id, project_id, tool_kind, title, summary, message_count, order_col, created_at, updated_at
             FROM sessions WHERE owner_id = ? ORDER BY order_col ASC, updated_at DESC LIMIT ?"
        )
        .bind(owner_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    let mut result = Vec::new();
    for row in rows {
        result.push(SessionRow {
            id: row.try_get(0)?,
            owner_id: row.try_get(1)?,
            project_id: row.try_get(2)?,
            tool_kind: row.try_get(3)?,
            title: row.try_get(4)?,
            summary: row.try_get(5)?,
            message_count: row.try_get(6)?,
            order_col: row.try_get(7)?,
            created_at: row.try_get(8)?,
            updated_at: row.try_get(9)?,
        });
    }
    Ok(result)
}

pub async fn update_summary(pool: &DbPool, session_id: &str, summary: &str) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE sessions SET summary = ?, updated_at = ? WHERE id = ?"
    )
    .bind(summary)
    .bind(&now)
    .bind(session_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_title(
    pool: &DbPool,
    session_id: &str,
    owner_id: &str,
    title: &str,
) -> AppResult<bool> {
    let now = chrono::Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE sessions SET title = ?, updated_at = ? WHERE id = ? AND owner_id = ?"
    )
    .bind(title)
    .bind(&now)
    .bind(session_id)
    .bind(owner_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn update_project_and_order(
    pool: &DbPool,
    session_id: &str,
    owner_id: &str,
    project_id: Option<&str>,
    order_col: i64,
) -> AppResult<bool> {
    let result = sqlx::query(
        "UPDATE sessions SET project_id = ?, order_col = ? WHERE id = ? AND owner_id = ?"
    )
    .bind(project_id)
    .bind(order_col)
    .bind(session_id)
    .bind(owner_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn save_artifacts(pool: &DbPool, session_id: &str, artifacts: &[Artifact]) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let payload = serde_json::to_string(artifacts)?;
    let id = uuid::Uuid::new_v4().to_string();

    // 使用 UPSERT：SQLite 用 ON CONFLICT，MySQL 用 ON DUPLICATE KEY UPDATE
    let cfg = crate::config::config();
    if cfg.is_mysql() {
        sqlx::query(
            "INSERT INTO session_artifacts (id, session_id, payload, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?)
             ON DUPLICATE KEY UPDATE payload = VALUES(payload), updated_at = VALUES(updated_at)"
        )
        .bind(&id)
        .bind(session_id)
        .bind(&payload)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
    } else {
        sqlx::query(
            "INSERT INTO session_artifacts (id, session_id, payload, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(session_id) DO UPDATE SET payload = excluded.payload, updated_at = excluded.updated_at"
        )
        .bind(&id)
        .bind(session_id)
        .bind(&payload)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn get_artifacts(pool: &DbPool, session_id: &str) -> AppResult<Vec<Artifact>> {
    let row = sqlx::query(
        "SELECT payload FROM session_artifacts WHERE session_id = ?"
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let payload: String = r.try_get(0)?;
            Ok(serde_json::from_str::<Vec<Artifact>>(&payload)?)
        }
        None => Ok(vec![]),
    }
}

pub async fn get_session_detail(pool: &DbPool, session_id: &str) -> AppResult<Option<SessionDetail>> {
    let session = match find_by_id(pool, session_id).await? {
        Some(session) => session,
        None => return Ok(None),
    };
    let messages = get_persisted_messages(pool, session_id, 100).await?;
    let artifacts = get_artifacts(pool, session_id).await?;
    Ok(Some(SessionDetail {
        id: session.id,
        owner_id: session.owner_id,
        project_id: session.project_id,
        tool_kind: session.tool_kind,
        title: session.title,
        summary: session.summary,
        message_count: session.message_count,
        order_col: session.order_col,
        created_at: session.created_at,
        updated_at: session.updated_at,
        messages,
        artifacts,
    }))
}

pub async fn add_message(pool: &DbPool, session_id: &str, msg: &ChatMessage) -> AppResult<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let tool_name: Option<String> = None;
    let tool_input: Option<String> = msg
        .tool_calls
        .as_ref()
        .map(|t| serde_json::to_string(t).unwrap_or_default());
    let tool_output: Option<String> = msg.tool_call_id.clone();
    sqlx::query(
        "INSERT INTO messages (id, session_id, role, content, tool_name, tool_input, tool_output, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(session_id)
    .bind(&msg.role)
    .bind(&msg.content)
    .bind(&tool_name)
    .bind(&tool_input)
    .bind(&tool_output)
    .bind(&now)
    .execute(pool)
    .await?;

    sqlx::query(
        "UPDATE sessions SET message_count = message_count + 1, updated_at = ? WHERE id = ?"
    )
    .bind(&now)
    .bind(session_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_messages(pool: &DbPool, session_id: &str, limit: i64) -> AppResult<Vec<ChatMessage>> {
    let rows = sqlx::query(
        "SELECT role, content, tool_input, tool_output FROM messages
         WHERE session_id = ? ORDER BY created_at ASC LIMIT ?"
    )
    .bind(session_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut result = Vec::new();
    for row in rows {
        let role: String = row.try_get(0)?;
        let content: String = row.try_get(1)?;
        let tool_input: Option<String> = row.try_get(2)?;
        let tool_output: Option<String> = row.try_get(3)?;
        let tool_calls = tool_input.and_then(|s| serde_json::from_str(&s).ok());
        let tool_call_id = tool_output;
        result.push(ChatMessage {
            role,
            content,
            tool_calls,
            tool_call_id,
        });
    }
    Ok(result)
}

pub async fn get_persisted_messages(
    pool: &DbPool,
    session_id: &str,
    limit: i64,
) -> AppResult<Vec<PersistedChatMessage>> {
    let rows = sqlx::query(
        "SELECT role, content, tool_input, tool_output, created_at FROM messages
         WHERE session_id = ? ORDER BY created_at ASC LIMIT ?"
    )
    .bind(session_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut result = Vec::new();
    for row in rows {
        let role: String = row.try_get(0)?;
        let content: String = row.try_get(1)?;
        let tool_input: Option<String> = row.try_get(2)?;
        let tool_output: Option<String> = row.try_get(3)?;
        let created_at: String = row.try_get(4)?;
        let tool_calls = tool_input.and_then(|s| serde_json::from_str(&s).ok());
        result.push(PersistedChatMessage {
            role,
            content,
            tool_calls,
            tool_call_id: tool_output,
            created_at,
        });
    }
    Ok(result)
}

pub async fn delete(pool: &DbPool, session_id: &str, owner_id: &str) -> AppResult<bool> {
    let result = sqlx::query(
        "DELETE FROM sessions WHERE id = ? AND owner_id = ?"
    )
    .bind(session_id)
    .bind(owner_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn clear_messages(pool: &DbPool, session_id: &str, owner_id: &str) -> AppResult<bool> {
    // 验证所有权
    let session = sqlx::query(
        "SELECT id FROM sessions WHERE id = ? AND owner_id = ?"
    )
    .bind(session_id)
    .bind(owner_id)
    .fetch_optional(pool)
    .await?;

    if session.is_none() {
        return Ok(false);
    }

    sqlx::query("DELETE FROM messages WHERE session_id = ?")
        .bind(session_id)
        .execute(pool)
        .await?;

    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE sessions SET message_count = 0, updated_at = ? WHERE id = ?"
    )
    .bind(&now)
    .bind(session_id)
    .execute(pool)
    .await?;
    Ok(true)
}
