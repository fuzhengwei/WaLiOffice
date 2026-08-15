use super::DbPool;
use crate::error::AppResult;
use crate::models::{Artifact, ChatMessage, PersistedChatMessage};
use rusqlite::params;
use serde::{Deserialize, Serialize};

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

pub fn create(
    pool: &DbPool,
    owner_id: &str,
    project_id: Option<&str>,
    tool_kind: Option<&str>,
    title: &str,
) -> AppResult<SessionRow> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO sessions (id, owner_id, project_id, tool_kind, title, message_count, order_col, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, strftime('%s','now'), ?6, ?7)",
        params![id, owner_id, project_id, tool_kind, title, &now, &now],
    )?;
    Ok(SessionRow {
        id,
        owner_id: owner_id.to_string(),
        project_id: project_id.map(String::from),
        tool_kind: tool_kind.map(String::from),
        title: title.to_string(),
        summary: None,
        message_count: 0,
        order_col: chrono::Utc::now().timestamp(),
        created_at: now.clone(),
        updated_at: now,
    })
}

pub fn find_by_id(pool: &DbPool, id: &str) -> AppResult<Option<SessionRow>> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let row = conn.query_row(
        "SELECT id, owner_id, project_id, tool_kind, title, summary, message_count, order_col, created_at, updated_at
         FROM sessions WHERE id = ?1",
        params![id],
        |row| Ok(SessionRow {
            id: row.get(0)?, owner_id: row.get(1)?, project_id: row.get(2)?, tool_kind: row.get(3)?,
            title: row.get(4)?, summary: row.get(5)?, message_count: row.get(6)?, order_col: row.get(7)?,
            created_at: row.get(8)?, updated_at: row.get(9)?,
        }),
    );
    match row {
        Ok(s) => Ok(Some(s)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn list_by_owner(
    pool: &DbPool,
    owner_id: &str,
    limit: i64,
    query: Option<&str>,
) -> AppResult<Vec<SessionRow>> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let q = query
        .map(|item| format!("%{}%", item.trim()))
        .filter(|item| item != "%%");
    let mut result = Vec::new();
    if let Some(ref qv) = q {
        let mut stmt = conn.prepare(
            "SELECT id, owner_id, project_id, tool_kind, title, summary, message_count, order_col, created_at, updated_at
             FROM sessions
             WHERE owner_id = ?1 AND (title LIKE ?2 OR COALESCE(summary, '') LIKE ?2)
             ORDER BY order_col ASC, updated_at DESC LIMIT ?3"
        )?;
        let rows = stmt.query_map(params![owner_id, qv, limit], |row| {
            Ok(SessionRow {
                id: row.get(0)?,
                owner_id: row.get(1)?,
                project_id: row.get(2)?,
                tool_kind: row.get(3)?,
                title: row.get(4)?,
                summary: row.get(5)?,
                message_count: row.get(6)?,
                order_col: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;
        for row in rows {
            result.push(row?);
        }
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, owner_id, project_id, tool_kind, title, summary, message_count, order_col, created_at, updated_at
             FROM sessions WHERE owner_id = ?1 ORDER BY order_col ASC, updated_at DESC LIMIT ?2"
        )?;
        let rows = stmt.query_map(params![owner_id, limit], |row| {
            Ok(SessionRow {
                id: row.get(0)?,
                owner_id: row.get(1)?,
                project_id: row.get(2)?,
                tool_kind: row.get(3)?,
                title: row.get(4)?,
                summary: row.get(5)?,
                message_count: row.get(6)?,
                order_col: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;
        for row in rows {
            result.push(row?);
        }
    }
    Ok(result)
}

pub fn update_summary(pool: &DbPool, session_id: &str, summary: &str) -> AppResult<()> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE sessions SET summary = ?1, updated_at = ?2 WHERE id = ?3",
        params![summary, &now, session_id],
    )?;
    Ok(())
}

pub fn update_title(
    pool: &DbPool,
    session_id: &str,
    owner_id: &str,
    title: &str,
) -> AppResult<bool> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let now = chrono::Utc::now().to_rfc3339();
    let affected = conn.execute(
        "UPDATE sessions SET title = ?1, updated_at = ?2 WHERE id = ?3 AND owner_id = ?4",
        params![title, &now, session_id, owner_id],
    )?;
    Ok(affected > 0)
}

pub fn update_project_and_order(
    pool: &DbPool,
    session_id: &str,
    owner_id: &str,
    project_id: Option<&str>,
    order_col: i64,
) -> AppResult<bool> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let affected = conn.execute(
        "UPDATE sessions SET project_id = ?1, order_col = ?2 WHERE id = ?3 AND owner_id = ?4",
        params![project_id, order_col, session_id, owner_id],
    )?;
    Ok(affected > 0)
}

pub fn save_artifacts(pool: &DbPool, session_id: &str, artifacts: &[Artifact]) -> AppResult<()> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let now = chrono::Utc::now().to_rfc3339();
    let payload = serde_json::to_string(artifacts)?;
    conn.execute(
        "INSERT INTO session_artifacts (id, session_id, payload, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(session_id) DO UPDATE SET
           payload = excluded.payload,
           updated_at = excluded.updated_at",
        params![
            uuid::Uuid::new_v4().to_string(),
            session_id,
            payload,
            &now,
            &now
        ],
    )?;
    Ok(())
}

pub fn get_artifacts(pool: &DbPool, session_id: &str) -> AppResult<Vec<Artifact>> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let row = conn.query_row(
        "SELECT payload FROM session_artifacts WHERE session_id = ?1",
        params![session_id],
        |row| row.get::<_, String>(0),
    );
    match row {
        Ok(payload) => Ok(serde_json::from_str::<Vec<Artifact>>(&payload)?),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(vec![]),
        Err(e) => Err(e.into()),
    }
}

pub fn get_session_detail(pool: &DbPool, session_id: &str) -> AppResult<Option<SessionDetail>> {
    let session = match find_by_id(pool, session_id)? {
        Some(session) => session,
        None => return Ok(None),
    };
    let messages = get_persisted_messages(pool, session_id, 100)?;
    let artifacts = get_artifacts(pool, session_id)?;
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

pub fn add_message(pool: &DbPool, session_id: &str, msg: &ChatMessage) -> AppResult<()> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let tool_name: Option<String> = None;
    let tool_input: Option<String> = msg
        .tool_calls
        .as_ref()
        .map(|t| serde_json::to_string(t).unwrap_or_default());
    let tool_output: Option<String> = msg.tool_call_id.clone();
    conn.execute(
        "INSERT INTO messages (id, session_id, role, content, tool_name, tool_input, tool_output, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, session_id, msg.role, msg.content, tool_name, tool_input, tool_output, &now],
    )?;
    conn.execute(
        "UPDATE sessions SET message_count = message_count + 1, updated_at = ?1 WHERE id = ?2",
        params![&now, session_id],
    )?;
    Ok(())
}

pub fn get_messages(pool: &DbPool, session_id: &str, limit: i64) -> AppResult<Vec<ChatMessage>> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let mut stmt = conn.prepare(
        "SELECT role, content, tool_input, tool_output FROM messages
         WHERE session_id = ?1 ORDER BY created_at ASC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![session_id, limit], |row| {
        let role: String = row.get(0)?;
        let content: String = row.get(1)?;
        let tool_input: Option<String> = row.get(2)?;
        let tool_output: Option<String> = row.get(3)?;
        let tool_calls = tool_input.and_then(|s| serde_json::from_str(&s).ok());
        let tool_call_id = tool_output;
        Ok(ChatMessage {
            role,
            content,
            tool_calls,
            tool_call_id,
        })
    })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn get_persisted_messages(
    pool: &DbPool,
    session_id: &str,
    limit: i64,
) -> AppResult<Vec<PersistedChatMessage>> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let mut stmt = conn.prepare(
        "SELECT role, content, tool_input, tool_output, created_at FROM messages
         WHERE session_id = ?1 ORDER BY created_at ASC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![session_id, limit], |row| {
        let role: String = row.get(0)?;
        let content: String = row.get(1)?;
        let tool_input: Option<String> = row.get(2)?;
        let tool_output: Option<String> = row.get(3)?;
        let created_at: String = row.get(4)?;
        let tool_calls = tool_input.and_then(|s| serde_json::from_str(&s).ok());
        Ok(PersistedChatMessage {
            role,
            content,
            tool_calls,
            tool_call_id: tool_output,
            created_at,
        })
    })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn delete(pool: &DbPool, session_id: &str, owner_id: &str) -> AppResult<bool> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let affected = conn.execute(
        "DELETE FROM sessions WHERE id = ?1 AND owner_id = ?2",
        params![session_id, owner_id],
    )?;
    Ok(affected > 0)
}

pub fn clear_messages(pool: &DbPool, session_id: &str, owner_id: &str) -> AppResult<bool> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    // 验证所有权
    let session: Option<String> = conn
        .query_row(
            "SELECT id FROM sessions WHERE id = ?1 AND owner_id = ?2",
            params![session_id, owner_id],
            |r| r.get(0),
        )
        .ok();
    if session.is_none() {
        return Ok(false);
    }
    conn.execute(
        "DELETE FROM messages WHERE session_id = ?1",
        params![session_id],
    )?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE sessions SET message_count = 0, updated_at = ?1 WHERE id = ?2",
        params![&now, session_id],
    )?;
    Ok(true)
}
