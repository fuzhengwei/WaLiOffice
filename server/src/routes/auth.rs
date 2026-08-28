use axum::routing::{get, post};
use axum::{Json, Router};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::Deserialize;

use crate::auth::middleware::AuthUser;
use crate::db::user_repo;
use crate::error::AppError;
use crate::models::{LoginRequest, RegisterRequest, TokenResponse, VerificationLoginRequest};
use crate::state;

pub fn router() -> Router {
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/verification-login", post(verification_login))
        .route("/api/auth/register", post(register))
        .route("/api/auth/me", get(me))
}

async fn login(Json(req): Json<LoginRequest>) -> Result<Json<TokenResponse>, AppError> {
    let pool = state::db_pool();
    let (user, hash) = user_repo::find_by_username(&pool, &req.username)
        .await?
        .ok_or(AppError::BadRequest("用户名或密码错误".into()))?;

    if !user_repo::verify_password(&hash, &req.password) {
        return Err(AppError::BadRequest("用户名或密码错误".into()));
    }

    let token = crate::auth::create_token(&user)?;
    Ok(Json(TokenResponse {
        access_token: token,
        token_type: "bearer".into(),
        user,
    }))
}

#[derive(Debug, Deserialize)]
struct XApiLoginResponse {
    code: String,
    #[serde(default)]
    info: String,
    #[serde(default)]
    data: Option<String>,
}

async fn verification_login(
    Json(req): Json<VerificationLoginRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    login_with_verification(req).await
}

async fn login_with_verification(
    req: VerificationLoginRequest,
) -> Result<Json<TokenResponse>, AppError> {
    let code = req.code.trim();
    if code.is_empty() {
        return Err(AppError::BadRequest("请输入验证码".into()));
    }

    let cfg = crate::config::config();
    let resp = reqwest::Client::new()
        .post(&cfg.x_api_auth_login_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[("code", code)])
        .send()
        .await
        .map_err(|_| AppError::BadRequest("验证码服务暂不可用".into()))?;

    let result = resp
        .json::<XApiLoginResponse>()
        .await
        .map_err(|_| AppError::BadRequest("验证码服务响应异常".into()))?;

    if result.code != "0000" {
        return Err(AppError::BadRequest(if result.info.is_empty() {
            "验证码无效或已过期".into()
        } else {
            result.info
        }));
    }

    let pool = state::db_pool();
    let external_id = result
        .data
        .as_deref()
        .and_then(extract_openid_from_x_api_token)
        .unwrap_or_else(|| code.to_string());
    let username = format!("wx_{external_id}");
    let user = user_repo::find_or_create_external(&pool, &username).await?;
    let token = crate::auth::create_token(&user)?;

    Ok(Json(TokenResponse {
        access_token: token,
        token_type: "bearer".into(),
        user,
    }))
}

fn extract_openid_from_x_api_token(token: &str) -> Option<String> {
    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    value.get("openId")?.as_str().map(ToString::to_string)
}

async fn register(Json(req): Json<RegisterRequest>) -> Result<Json<TokenResponse>, AppError> {
    if req.username.len() < 3 {
        return Err(AppError::BadRequest("用户名至少 3 个字符".into()));
    }
    if req.password.len() < 6 {
        return Err(AppError::BadRequest("密码至少 6 个字符".into()));
    }

    let pool = state::db_pool();
    if user_repo::find_by_username(&pool, &req.username).await?.is_some() {
        return Err(AppError::BadRequest("用户名已存在".into()));
    }

    // 多租户：若指定 tenant_id 则归属该租户；否则为平台首个用户分配超管、其余自动创建个人租户
    let hash = user_repo::hash_password(&req.password)?;
    let user = if let Some(tenant_id) = req.tenant_id.as_deref() {
        // 归属指定租户（需校验租户存在，一般由租户管理员邀请）
        if crate::db::tenant_repo::find_by_id(&pool, tenant_id).await?.is_none() {
            return Err(AppError::BadRequest("租户不存在".into()));
        }
        user_repo::create_with_tenant(&pool, &req.username, req.email.as_deref(), &hash, Some(tenant_id), "member").await?
    } else {
        // 自动创建个人租户
        let count = crate::db::user_repo::count(&pool).await.unwrap_or(0);
        if count == 0 {
            // 平台首个用户 → 超级管理员（无租户归属）
            user_repo::create_with_tenant(&pool, &req.username, req.email.as_deref(), &hash, None, "super_admin").await?
        } else {
            let slug = format!("tenant-{}", uuid::Uuid::new_v4().simple());
            let tenant = crate::db::tenant_repo::create(&pool, &req.username, &slug, "free").await?;
            user_repo::create_with_tenant(&pool, &req.username, req.email.as_deref(), &hash, Some(&tenant.id), "tenant_admin").await?
        }
    };
    let token = crate::auth::create_token(&user)?;

    Ok(Json(TokenResponse {
        access_token: token,
        token_type: "bearer".into(),
        user,
    }))
}

async fn me(user: AuthUser) -> Result<Json<crate::models::User>, AppError> {
    Ok(Json(user.0))
}
