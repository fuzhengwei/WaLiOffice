use super::DbPool;
use crate::error::AppResult;
use crate::models::User;

pub async fn find_by_username(pool: &DbPool, username: &str) -> AppResult<Option<(User, String)>> {
    let row = sqlx::query(
        "SELECT id, username, email, password_hash, avatar, role FROM users WHERE username = ?"
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let id: String = r.try_get(0)?;
            let username: String = r.try_get(1)?;
            let email: Option<String> = r.try_get(2)?;
            let password_hash: String = r.try_get(3)?;
            let avatar: Option<String> = r.try_get(4)?;
            let role: String = r.try_get(5)?;
            Ok(Some((
                User { id, username, email, avatar, role },
                password_hash,
            )))
        }
        None => Ok(None),
    }
}

pub async fn find_by_id(pool: &DbPool, id: &str) -> AppResult<Option<User>> {
    let row = sqlx::query(
        "SELECT id, username, email, avatar, role FROM users WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let user = User {
                id: r.try_get(0)?,
                username: r.try_get(1)?,
                email: r.try_get(2)?,
                avatar: r.try_get(3)?,
                role: r.try_get(4)?,
            };
            Ok(Some(user))
        }
        None => Ok(None),
    }
}

pub async fn create(
    pool: &DbPool,
    username: &str,
    email: Option<&str>,
    password_hash: &str,
) -> AppResult<User> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, role, created_at, updated_at)
         VALUES (?, ?, ?, ?, 'user', ?, ?)"
    )
    .bind(&id)
    .bind(username)
    .bind(email)
    .bind(password_hash)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(User {
        id,
        username: username.to_string(),
        email: email.map(|s| s.to_string()),
        avatar: None,
        role: "user".to_string(),
    })
}

pub async fn find_or_create_external(pool: &DbPool, username: &str) -> AppResult<User> {
    if let Some((user, _)) = find_by_username(pool, username).await? {
        return Ok(user);
    }

    let password_hash = hash_password(&uuid::Uuid::new_v4().to_string())?;
    create(pool, username, None, &password_hash).await
}

pub fn verify_password(hash: &str, password: &str) -> bool {
    bcrypt::verify(password, hash).unwrap_or(false)
}

pub fn hash_password(password: &str) -> AppResult<String> {
    Ok(bcrypt::hash(password, 10)?)
}

use sqlx::Row;
