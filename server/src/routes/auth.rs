use axum::routing::{get, post};
use axum::{Json, Router};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::Deserialize;

use crate::auth::middleware::AuthUser;
use crate::db::{tenant_repo, user_repo};
use crate::error::AppError;
use crate::models::{
    FeishuLoginRequest, InviteRequest, LoginRequest, RegisterRequest, TokenResponse,
    VerificationLoginRequest,
};
use crate::state;

pub fn router() -> Router {
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/verification-login", post(verification_login))
        .route("/api/auth/register", post(register))
        .route("/api/auth/register-by-invite", post(register_by_invite))
        .route("/api/auth/feishu/login", post(feishu_login))
        .route("/api/auth/feishu/config", get(feishu_config))
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
    let cfg = crate::config::config();
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
            if !cfg.allow_register {
                return Err(AppError::Forbidden);
            }
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

/// 邀请码注册：通过租户邀请码（tenant 的 invite_code）归属到指定租户
async fn register_by_invite(
    Json(req): Json<InviteRequest>,
) -> Result<Json<TokenResponse>, AppError> {
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

    // 校验邀请码，定位租户
    let tenant = tenant_repo::find_by_invite_code(&pool, &req.invite_code.trim())
        .await?
        .ok_or(AppError::BadRequest("邀请码无效".into()))?;

    let hash = user_repo::hash_password(&req.password)?;
    let user = user_repo::create_with_tenant(
        &pool,
        &req.username,
        req.email.as_deref(),
        &hash,
        Some(&tenant.id),
        "member",
    )
    .await?;
    let token = crate::auth::create_token(&user)?;

    Ok(Json(TokenResponse {
        access_token: token,
        token_type: "bearer".into(),
        user,
    }))
}

/// 飞书 OAuth 登录：code 换 token + open_id，自动建/找用户并签发 JWT
async fn feishu_login(
    Json(req): Json<FeishuLoginRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    let cfg = crate::config::config();
    if cfg.feishu_app_id.is_empty() || cfg.feishu_app_secret.is_empty() {
        return Err(AppError::BadRequest("服务未配置飞书登录".into()));
    }

    let code = req.code.trim();
    if code.is_empty() {
        return Err(AppError::BadRequest("缺少授权码".into()));
    }

    // 1. 换 user_access_token（飞书官方要求 client_id/client_secret/redirect_uri 放请求体）
    let client = reqwest::Client::new();
    let token_resp = client
        .post("https://open.feishu.cn/open-apis/authen/v2/oauth/token")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "grant_type": "authorization_code",
            "client_id": cfg.feishu_app_id,
            "client_secret": cfg.feishu_app_secret,
            "code": code,
            "redirect_uri": cfg.feishu_redirect_uri,
        }))
        .send()
        .await
        .map_err(|_| AppError::BadRequest("飞书授权服务不可用".into()))?;
    let token_json: serde_json::Value = token_resp
        .json()
        .await
        .map_err(|_| AppError::BadRequest("飞书授权响应异常".into()))?;
    tracing::info!("[Feishu] token exchange response: {}", token_json);
    let user_token = token_json
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("飞书授权失败".into()))?;

    // 2. 拉取用户信息（open_id / name）
    let user_resp = client
        .get("https://open.feishu.cn/open-apis/authen/v1/user_info")
        .header("Authorization", format!("Bearer {user_token}"))
        .send()
        .await
        .map_err(|_| AppError::BadRequest("飞书用户信息服务不可用".into()))?;
    let user_json: serde_json::Value = user_resp
        .json()
        .await
        .map_err(|_| AppError::BadRequest("飞书用户信息响应异常".into()))?;
    let data = user_json.get("data").cloned().unwrap_or_default();
    let open_id = data
        .get("open_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("未获取到飞书 open_id".into()))?;
    let name = data
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("feishu_user")
        .to_string();

    // 3. 按 open_id 找或建用户（username 形如 feishu_{open_id}）
    let pool = state::db_pool();
    let username = format!("feishu_{open_id}");
    let user = match user_repo::find_by_username(&pool, &username).await? {
        Some((user, _)) => user,
        None => {
            // 新建：若带 tenant_id 则归属该租户；否则自动建个人租户
            let hash = user_repo::hash_password(&uuid::Uuid::new_v4().to_string())?;
            if let Some(tenant_id) = req.tenant_id.as_deref() {
                user_repo::create_with_tenant(&pool, &username, None, &hash, Some(tenant_id), "member").await?
            } else {
                let count = user_repo::count(&pool).await.unwrap_or(0);
                if count == 0 {
                    user_repo::create_with_tenant(&pool, &username, None, &hash, None, "super_admin").await?
                } else {
                    let slug = format!("tenant-{}", uuid::Uuid::new_v4().simple());
                    let tenant = tenant_repo::create(&pool, &name, &slug, "free").await?;
                    user_repo::create_with_tenant(&pool, &username, None, &hash, Some(&tenant.id), "tenant_admin").await?
                }
            }
        }
    };

    let token = crate::auth::create_token(&user)?;
    Ok(Json(TokenResponse {
        access_token: token,
        token_type: "bearer".into(),
        user,
    }))
}

/// 飞书登录前端配置：返回 app_id 与 redirect_uri（供前端拼授权 URL）
async fn feishu_config() -> Result<Json<serde_json::Value>, AppError> {
    let cfg = crate::config::config();
    Ok(Json(serde_json::json!({
        "enabled": !cfg.feishu_app_id.is_empty(),
        "app_id": cfg.feishu_app_id,
        "redirect_uri": cfg.feishu_redirect_uri,
    })))
}
