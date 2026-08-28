use super::DbPool;
use crate::error::AppResult;
use crate::models::User;

pub async fn find_by_username(pool: &DbPool, username: &str) -> AppResult<Option<(User, String)>> {
    let row = sqlx::query(
        "SELECT id, tenant_id, username, email, password_hash, avatar, role FROM users WHERE username = ?"
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let id: String = r.try_get(0)?;
            let tenant_id: Option<String> = r.try_get(1)?;
            let username: String = r.try_get(2)?;
            let email: Option<String> = r.try_get(3)?;
            let password_hash: String = r.try_get(4)?;
            let avatar: Option<String> = r.try_get(5)?;
            let role: String = r.try_get(6)?;
            Ok(Some((
                User { id, tenant_id, username, email, avatar, role },
                password_hash,
            )))
        }
        None => Ok(None),
    }
}

pub async fn count(pool: &DbPool) -> AppResult<i64> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(pool).await?;
    Ok(count)
}

pub async fn find_by_id(pool: &DbPool, id: &str) -> AppResult<Option<User>> {
    let row = sqlx::query(
        "SELECT id, tenant_id, username, email, avatar, role FROM users WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let user = User {
                id: r.try_get(0)?,
                tenant_id: r.try_get(1)?,
                username: r.try_get(2)?,
                email: r.try_get(3)?,
                avatar: r.try_get(4)?,
                role: r.try_get(5)?,
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
    create_with_tenant(pool, username, email, password_hash, None, "member").await
}

pub async fn create_with_tenant(
    pool: &DbPool,
    username: &str,
    email: Option<&str>,
    password_hash: &str,
    tenant_id: Option<&str>,
    role: &str,
) -> AppResult<User> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO users (id, tenant_id, username, email, password_hash, role, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(tenant_id)
    .bind(username)
    .bind(email)
    .bind(password_hash)
    .bind(role)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(User {
        id,
        tenant_id: tenant_id.map(str::to_string),
        username: username.to_string(),
        email: email.map(|s| s.to_string()),
        avatar: None,
        role: role.to_string(),
    })
}

pub async fn update_role(
    pool: &DbPool,
    user_id: &str,
    role: &str,
) -> AppResult<Option<User>> {
    let now = chrono::Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE users SET role = ?, updated_at = ? WHERE id = ?"
    )
    .bind(role)
    .bind(&now)
    .bind(user_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Ok(None);
    }
    find_by_id(pool, user_id).await
}

pub async fn assign_tenant(
    pool: &DbPool,
    user_id: &str,
    tenant_id: Option<&str>,
) -> AppResult<Option<User>> {
    let now = chrono::Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE users SET tenant_id = ?, updated_at = ? WHERE id = ?"
    )
    .bind(tenant_id)
    .bind(&now)
    .bind(user_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Ok(None);
    }
    find_by_id(pool, user_id).await
}

pub async fn list_by_tenant(pool: &DbPool, tenant_id: &str) -> AppResult<Vec<User>> {
    let rows = sqlx::query(
        "SELECT id, tenant_id, username, email, avatar, role FROM users WHERE tenant_id = ? ORDER BY created_at ASC"
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;

    let mut result = Vec::new();
    for r in rows {
        result.push(User {
            id: r.try_get(0)?,
            tenant_id: r.try_get(1)?,
            username: r.try_get(2)?,
            email: r.try_get(3)?,
            avatar: r.try_get(4)?,
            role: r.try_get(5)?,
        });
    }
    Ok(result)
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
