use super::DbPool;
use crate::error::AppResult;
use crate::models::Tenant;
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct TenantRow {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub plan: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

fn map(r: &sqlx::any::AnyRow) -> AppResult<TenantRow> {
    Ok(TenantRow {
        id: r.try_get(0)?,
        name: r.try_get(1)?,
        slug: r.try_get(2)?,
        plan: r.try_get(3)?,
        status: r.try_get(4)?,
        created_at: r.try_get(5)?,
        updated_at: r.try_get(6)?,
    })
}

const COLS: &str = "id, name, slug, plan, status, created_at, updated_at";

pub async fn create(
    pool: &DbPool,
    name: &str,
    slug: &str,
    plan: &str,
) -> AppResult<TenantRow> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO tenants (id, name, slug, plan, status, created_at, updated_at)
         VALUES (?, ?, ?, ?, 'active', ?, ?)",
    )
    .bind(&id)
    .bind(name)
    .bind(slug)
    .bind(plan)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(TenantRow {
        id,
        name: name.to_string(),
        slug: slug.to_string(),
        plan: plan.to_string(),
        status: "active".to_string(),
        created_at: now.clone(),
        updated_at: now,
    })
}

pub async fn find_by_id(pool: &DbPool, id: &str) -> AppResult<Option<TenantRow>> {
    let sql = format!("SELECT {COLS} FROM tenants WHERE id = ?");
    let row = sqlx::query(&sql).bind(id).fetch_optional(pool).await?;
    match row {
        Some(r) => Ok(Some(map(&r)?)),
        None => Ok(None),
    }
}

pub async fn find_by_slug(pool: &DbPool, slug: &str) -> AppResult<Option<TenantRow>> {
    let sql = format!("SELECT {COLS} FROM tenants WHERE slug = ?");
    let row = sqlx::query(&sql).bind(slug).fetch_optional(pool).await?;
    match row {
        Some(r) => Ok(Some(map(&r)?)),
        None => Ok(None),
    }
}

pub async fn list(pool: &DbPool) -> AppResult<Vec<TenantRow>> {
    let sql = format!("SELECT {COLS} FROM tenants ORDER BY created_at ASC");
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    let mut result = Vec::new();
    for r in rows {
        result.push(map(&r)?);
    }
    Ok(result)
}

pub async fn update(
    pool: &DbPool,
    id: &str,
    name: Option<&str>,
    plan: Option<&str>,
    status: Option<&str>,
) -> AppResult<Option<TenantRow>> {
    let current = match find_by_id(pool, id).await? {
        Some(t) => t,
        None => return Ok(None),
    };
    let next_name = name.unwrap_or(&current.name);
    let next_plan = plan.unwrap_or(&current.plan);
    let next_status = status.unwrap_or(&current.status);
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "UPDATE tenants SET name = ?, plan = ?, status = ?, updated_at = ? WHERE id = ?",
    )
    .bind(next_name)
    .bind(next_plan)
    .bind(next_status)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(Some(TenantRow {
        id: id.to_string(),
        name: next_name.to_string(),
        slug: current.slug,
        plan: next_plan.to_string(),
        status: next_status.to_string(),
        created_at: current.created_at,
        updated_at: now,
    }))
}

pub async fn delete(pool: &DbPool, id: &str) -> AppResult<bool> {
    let result = sqlx::query("DELETE FROM tenants WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

impl From<TenantRow> for Tenant {
    fn from(t: TenantRow) -> Self {
        Tenant {
            id: t.id,
            name: t.name,
            slug: t.slug,
            plan: t.plan,
            status: t.status,
            created_at: t.created_at,
            updated_at: t.updated_at,
        }
    }
}
