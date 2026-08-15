use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use axum::extract::{Path, Query};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;

pub fn router() -> Router {
    Router::new()
        .route("/api/files", get(list_files))
        .route("/api/files/search", get(search_files))
        .route("/api/files/stats", get(file_stats))
        .route("/api/files/upload", post(upload_file))
        .route("/api/files/:id", get(get_file).delete(delete_file))
        .route("/api/files/:id/download", get(download_file))
        .route("/api/files/folders/list", get(list_folders))
        .route("/api/folders", post(create_folder))
        .route("/api/folders/:id", delete(delete_folder))
}

#[derive(Deserialize)]
struct FileQuery {
    #[serde(default)]
    folder_id: Option<String>,
    #[serde(default)]
    q: Option<String>,
}

async fn list_files(
    _user: AuthUser,
    Query(_q): Query<FileQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({ "files": [] })))
}
async fn search_files(
    _user: AuthUser,
    Query(_q): Query<FileQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({ "files": [] })))
}
async fn file_stats(_user: AuthUser) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({ "total": 0, "size": 0 })))
}
async fn upload_file(_user: AuthUser) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({ "ok": false, "detail": "文件上传功能待实现" })))
}
async fn get_file(
    _user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({ "id": id })))
}
async fn download_file(
    _user: AuthUser,
    Path(_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({ "detail": "文件不存在" })))
}
async fn delete_file(
    _user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({ "deleted": false, "id": id })))
}
async fn list_folders(_user: AuthUser) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({ "folders": [] })))
}
async fn create_folder(
    _user: AuthUser,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({ "ok": false, "folder": payload })))
}
async fn delete_folder(
    _user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({ "deleted": false, "id": id })))
}
