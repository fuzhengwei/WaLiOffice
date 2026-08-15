use axum::http::request::Parts;

use crate::error::AppError;
use crate::models::User;
use super::verify_token;

/// 从请求头解析当前用户
pub fn extract_user(parts: &Parts) -> Result<User, AppError> {
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
    let user = crate::db::user_repo::find_by_id(&pool, &claims.sub)?
        .ok_or(AppError::Unauthorized)?;

    Ok(user)
}

/// axum 提取器
#[derive(Clone)]
pub struct AuthUser(pub User);

#[async_trait::async_trait]
impl<S> axum::extract::FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user = extract_user(parts)?;
        Ok(AuthUser(user))
    }
}
