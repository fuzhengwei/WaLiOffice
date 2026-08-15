use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;

use crate::auth::middleware::AuthUser;
use crate::db::user_repo;
use crate::error::AppError;
use crate::models::{LoginRequest, RegisterRequest, TokenResponse};
use crate::state;

pub fn router() -> Router {
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/register", post(register))
        .route("/api/auth/me", get(me))
        .route("/api/auth/demo-accounts", get(demo_accounts))
}

async fn login(Json(req): Json<LoginRequest>) -> Result<Json<TokenResponse>, AppError> {
    let pool = state::db_pool();
    let (user, hash) = user_repo::find_by_username(&pool, &req.username)?
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

async fn register(Json(req): Json<RegisterRequest>) -> Result<Json<TokenResponse>, AppError> {
    if req.username.len() < 3 {
        return Err(AppError::BadRequest("用户名至少 3 个字符".into()));
    }
    if req.password.len() < 6 {
        return Err(AppError::BadRequest("密码至少 6 个字符".into()));
    }

    let pool = state::db_pool();
    if user_repo::find_by_username(&pool, &req.username)?.is_some() {
        return Err(AppError::BadRequest("用户名已存在".into()));
    }

    let hash = user_repo::hash_password(&req.password)?;
    let user = user_repo::create(&pool, &req.username, req.email.as_deref(), &hash)?;
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

async fn demo_accounts() -> Json<serde_json::Value> {
    let cfg = crate::config::config();
    Json(json!({
        "accounts": [
            {
                "username": cfg.admin_username,
                "password": cfg.admin_password,
                "role": "admin",
                "description": "管理员账号"
            }
        ]
    }))
}
