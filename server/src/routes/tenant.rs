use axum::extract::Path;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;

use crate::auth::middleware::{ensure_role, AuthUser};
use crate::db::{tenant_repo, user_repo};
use crate::error::AppError;
use crate::models::{
    CreateTenantRequest, TenantMemberRequest, UpdateTenantRequest, UpdateUserRoleRequest,
};
use crate::state;

pub fn router() -> Router {
    Router::new()
        .route("/api/tenants", get(list_tenants).post(create_tenant))
        .route(
            "/api/tenants/:tenant_id",
            get(get_tenant).patch(update_tenant).delete(delete_tenant),
        )
        .route(
            "/api/tenants/:tenant_id/members",
            get(list_members).post(add_member),
        )
        .route(
            "/api/tenants/:tenant_id/members/:user_id",
            post(update_member_role).delete(remove_member),
        )
        .route("/api/users/:user_id/role", post(update_user_role))
}

/// 仅超级管理员可管理租户及全局用户角色
fn require_super_admin(user: &crate::models::User) -> Result<(), AppError> {
    ensure_role(user, &["super_admin"])
}

async fn list_tenants(user: AuthUser) -> Result<Json<serde_json::Value>, AppError> {
    require_super_admin(&user.0)?;
    let pool = state::db_pool();
    let tenants = tenant_repo::list(&pool).await?;
    let list: Vec<crate::models::Tenant> = tenants.into_iter().map(Into::into).collect();
    Ok(Json(json!({ "tenants": list })))
}

async fn create_tenant(
    user: AuthUser,
    Json(req): Json<CreateTenantRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_super_admin(&user.0)?;
    let name = req.name.trim().to_string();
    let slug = req.slug.trim().to_string();
    if name.is_empty() || slug.is_empty() {
        return Err(AppError::BadRequest("租户名称和标识不能为空".into()));
    }

    let pool = state::db_pool();
    if tenant_repo::find_by_slug(&pool, &slug).await?.is_some() {
        return Err(AppError::BadRequest("租户标识已存在".into()));
    }

    let tenant = tenant_repo::create(&pool, &name, &slug, req.plan.as_deref().unwrap_or("free"))
        .await?;
    Ok(Json(json!(crate::models::Tenant::from(tenant))))
}

async fn get_tenant(
    user: AuthUser,
    Path(tenant_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_super_admin(&user.0)?;
    let pool = state::db_pool();
    let tenant = tenant_repo::find_by_id(&pool, &tenant_id)
        .await?
        .ok_or(AppError::NotFound("租户不存在".into()))?;
    Ok(Json(json!(crate::models::Tenant::from(tenant))))
}

async fn update_tenant(
    user: AuthUser,
    Path(tenant_id): Path<String>,
    Json(req): Json<UpdateTenantRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_super_admin(&user.0)?;
    let pool = state::db_pool();
    let tenant = tenant_repo::update(
        &pool,
        &tenant_id,
        req.name.as_deref(),
        req.plan.as_deref(),
        req.status.as_deref(),
    )
    .await?
    .ok_or(AppError::NotFound("租户不存在".into()))?;
    Ok(Json(json!(crate::models::Tenant::from(tenant))))
}

async fn delete_tenant(
    user: AuthUser,
    Path(tenant_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_super_admin(&user.0)?;
    let pool = state::db_pool();
    let deleted = tenant_repo::delete(&pool, &tenant_id).await?;
    Ok(Json(json!({ "deleted": deleted })))
}

async fn list_members(
    user: AuthUser,
    Path(tenant_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_tenant_admin(&user.0, &tenant_id)?;
    let pool = state::db_pool();
    let members = user_repo::list_by_tenant(&pool, &tenant_id).await?;
    Ok(Json(json!({ "members": members })))
}

async fn add_member(
    user: AuthUser,
    Path(tenant_id): Path<String>,
    Json(req): Json<TenantMemberRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_tenant_admin(&user.0, &tenant_id)?;
    let pool = state::db_pool();
    let role = req.role.as_deref().unwrap_or("member");
    let updated = user_repo::assign_tenant(&pool, &req.user_id, Some(&tenant_id)).await?;
    let updated = user_repo::update_role(&pool, &req.user_id, role)
        .await?
        .or(updated)
        .ok_or(AppError::NotFound("用户不存在".into()))?;
    Ok(Json(json!(updated)))
}

async fn update_member_role(
    user: AuthUser,
    Path((tenant_id, member_id)): Path<(String, String)>,
    Json(req): Json<UpdateUserRoleRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_tenant_admin(&user.0, &tenant_id)?;
    let pool = state::db_pool();
    let member = user_repo::find_by_id(&pool, &member_id)
        .await?
        .ok_or(AppError::NotFound("用户不存在".into()))?;
    if member.tenant_id.as_deref() != Some(tenant_id.as_str()) {
        return Err(AppError::Forbidden);
    }
    let updated = user_repo::update_role(&pool, &member_id, &req.role)
        .await?
        .ok_or(AppError::NotFound("用户不存在".into()))?;
    Ok(Json(json!(updated)))
}

async fn remove_member(
    user: AuthUser,
    Path((tenant_id, member_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_tenant_admin(&user.0, &tenant_id)?;
    let pool = state::db_pool();
    // 禁止移除自己
    if member_id == user.0.id {
        return Err(AppError::BadRequest("不能移除自己".into()));
    }
    let member = user_repo::find_by_id(&pool, &member_id)
        .await?
        .ok_or(AppError::NotFound("用户不存在".into()))?;
    if member.tenant_id.as_deref() != Some(tenant_id.as_str()) {
        return Err(AppError::Forbidden);
    }
    // 逐出租户（清空 tenant_id，角色降为 member）
    let _ = user_repo::assign_tenant(&pool, &member_id, None).await?;
    let _ = user_repo::update_role(&pool, &member_id, "member").await?;
    Ok(Json(json!({ "removed": true })))
}

async fn update_user_role(
    user: AuthUser,
    Path(user_id): Path<String>,
    Json(req): Json<UpdateUserRoleRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_super_admin(&user.0)?;
    if !matches!(req.role.as_str(), "super_admin" | "tenant_admin" | "member") {
        return Err(AppError::BadRequest("非法角色".into()));
    }
    let pool = state::db_pool();
    let updated = user_repo::update_role(&pool, &user_id, &req.role)
        .await?
        .ok_or(AppError::NotFound("用户不存在".into()))?;
    Ok(Json(json!(updated)))
}

fn require_tenant_admin(user: &crate::models::User, tenant_id: &str) -> Result<(), AppError> {
    // 超管可管理所有租户
    if user.is_super_admin() {
        return Ok(());
    }
    // 租户管理员只能管理自己所属租户
    if user.tenant_id.as_deref() != Some(tenant_id) {
        return Err(AppError::Forbidden);
    }
    ensure_role(user, &["tenant_admin"])
}
