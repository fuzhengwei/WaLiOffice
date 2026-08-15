use super::DbPool;
use crate::error::AppResult;
use crate::models::User;
use rusqlite::params;

pub fn find_by_username(pool: &DbPool, username: &str) -> AppResult<Option<(User, String)>> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let row = conn.query_row(
        "SELECT id, username, email, password_hash, avatar, role FROM users WHERE username = ?1",
        params![username],
        |row| {
            Ok((
                User {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    email: row.get(2)?,
                    avatar: row.get(4)?,
                    role: row.get(5)?,
                },
                row.get::<_, String>(3)?,
            ))
        },
    );
    match row {
        Ok((user, hash)) => Ok(Some((user, hash))),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn find_by_id(pool: &DbPool, id: &str) -> AppResult<Option<User>> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let row = conn.query_row(
        "SELECT id, username, email, avatar, role FROM users WHERE id = ?1",
        params![id],
        |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                email: row.get(2)?,
                avatar: row.get(3)?,
                role: row.get(4)?,
            })
        },
    );
    match row {
        Ok(user) => Ok(Some(user)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn create(
    pool: &DbPool,
    username: &str,
    email: Option<&str>,
    password_hash: &str,
) -> AppResult<User> {
    let conn = pool.get().map_err(|e| anyhow::anyhow!(e))?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO users (id, username, email, password_hash, role, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'user', ?5, ?6)",
        params![id, username, email, password_hash, &now, &now],
    )?;
    Ok(User {
        id,
        username: username.to_string(),
        email: email.map(|s| s.to_string()),
        avatar: None,
        role: "user".to_string(),
    })
}

pub fn verify_password(hash: &str, password: &str) -> bool {
    bcrypt::verify(password, hash).unwrap_or(false)
}

pub fn hash_password(password: &str) -> AppResult<String> {
    Ok(bcrypt::hash(password, 10)?)
}
