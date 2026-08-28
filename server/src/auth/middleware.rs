use axum::http::request::Parts;

use super::verify_token;
use crate::error::AppError;
use crate::models::User;

/// 从请求头解析当前用户
pub async fn extract_user(parts: &Parts) -> Result<User, AppError> {
    let auth_header = parts
        .headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::Unauthorized)?;

    let claims = verify_token(token).map_err(|_| AppError::Unauthorized)?;

    let pool = crate::state::db_pool();
    let user =
        crate::db::user_repo::find_by_id(&pool, &claims.sub).await?.ok_or(AppError::Unauthorized)?;

    Ok(user)
}

/// axum 提取器
#[derive(Clone)]
pub struct AuthUser(pub User);

/// 角色校验提取器：要求用户具备指定角色之一。
/// 用法：`user: RequireRole<&'static [&'static str]>` 或自定义。
#[derive(Clone)]
pub struct RequireRole<const N: usize>(pub User);

#[async_trait::async_trait]
impl<S, const N: usize> axum::extract::FromRequestParts<S> for RequireRole<N>
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user = extract_user(parts).await?;
        Ok(RequireRole(user))
    }
}

/// 通用角色校验函数：检查 user.role 是否在允许列表中。
pub fn ensure_role(user: &User, allowed: &[&str]) -> Result<(), AppError> {
    if allowed.iter().any(|r| user.role.as_str() == *r) {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

#[async_trait::async_trait]
impl<S> axum::extract::FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user = extract_user(parts).await?;
        Ok(AuthUser(user))
    }
}
