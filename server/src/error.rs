use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("未认证")]
    Unauthorized,
    #[error("无权访问")]
    Forbidden,
    #[error("未找到: {0}")]
    NotFound(String),
    #[error("参数错误: {0}")]
    BadRequest(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),
    #[error("LLM 调用失败: {0}")]
    Llm(String),
    #[error("工具执行失败: {0}")]
    Tool(String),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("内部错误: {0}")]
    Internal(#[from] anyhow::Error),
}

impl From<bcrypt::BcryptError> for AppError {
    fn from(e: bcrypt::BcryptError) -> Self {
        AppError::Internal(anyhow::anyhow!("密码加密错误: {e}"))
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(e: jsonwebtoken::errors::Error) -> Self {
        AppError::Internal(anyhow::anyhow!("JWT 错误: {e}"))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "数据库错误".into()),
            AppError::Llm(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
            AppError::Tool(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            AppError::Io(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "内部错误".into()),
        };
        (status, Json(json!({ "detail": message }))).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Internal(anyhow::anyhow!("JSON 错误: {e}"))
    }
}
