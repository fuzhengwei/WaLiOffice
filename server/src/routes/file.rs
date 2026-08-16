use crate::auth::middleware::AuthUser;
use crate::db::file_repo;
use crate::error::AppError;
use crate::file_extract;
use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use std::path::{Path as FsPath, PathBuf};

const MAX_UPLOAD_BYTES: usize = 50 * 1024 * 1024;

pub fn router() -> Router {
    Router::new()
        .route("/api/files", get(list_files))
        .route("/api/files/search", get(search_files))
        .route("/api/files/stats", get(file_stats))
        .route("/api/files/extract", post(extract_file_text))
        .route("/api/files/upload", post(upload_file))
        .route("/api/files/:id/content", get(get_file_content))
        .route("/api/files/:id/thumbnail", get(get_file_thumbnail))
        .route("/api/files/:id/preview", get(get_file_preview))
        .route("/api/files/:id/stream", get(stream_file))
        .route("/api/files/:id", get(get_file).delete(delete_file))
        .route("/api/files/:id/download", get(download_file))
        .route("/api/files/folders/list", get(list_folders))
        .route("/api/folders", post(create_folder))
        .route("/api/folders/:id", delete(delete_folder))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_BYTES))
}

#[derive(Deserialize)]
struct FileQuery {
    #[serde(default)]
    folder_id: Option<String>,
    #[serde(default)]
    q: Option<String>,
}

#[derive(Deserialize)]
struct FolderQuery {
    #[serde(default)]
    parent_id: Option<String>,
}

#[derive(Deserialize)]
struct StreamQuery {
    /// 认证 token，用于 <video>/<img> 标签的 src URL 认证
    #[serde(default)]
    token: Option<String>,
}

#[derive(Deserialize)]
struct CreateFolderReq {
    name: String,
    #[serde(default)]
    parent_id: Option<String>,
}

async fn list_files(
    user: AuthUser,
    Query(q): Query<FileQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = crate::state::db_pool();
    if let Some(folder_id) = q.folder_id.as_deref() {
        ensure_folder_owner(&pool, &user.0.id, folder_id)?;
    }
    let files = file_repo::list_files(&pool, &user.0.id, q.folder_id.as_deref())?;
    Ok(Json(json!({ "files": files })))
}

async fn search_files(
    user: AuthUser,
    Query(q): Query<FileQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = crate::state::db_pool();
    let files = file_repo::search_files(&pool, &user.0.id, q.q.as_deref())?;
    Ok(Json(json!({ "files": files })))
}

async fn file_stats(user: AuthUser) -> Result<Json<serde_json::Value>, AppError> {
    let pool = crate::state::db_pool();
    let stats = file_repo::stats(&pool, &user.0.id)?;
    Ok(Json(json!({
        "by_type": stats.by_type,
        "total_size": stats.total_size,
        "total_files": stats.total_files,
        "total": stats.total_files,
        "size": stats.total_size,
    })))
}

async fn upload_file(
    user: AuthUser,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = crate::state::db_pool();
    let folder_id = header_value(&headers, "x-folder-id");
    let description = header_value(&headers, "x-description");
    if let Some(folder_id) = folder_id.as_deref() {
        ensure_folder_owner(&pool, &user.0.id, folder_id)?;
    }

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("上传表单解析失败: {e}")))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let original_name = field
            .file_name()
            .map(str::to_string)
            .or_else(|| header_value(&headers, "x-filename"))
            .unwrap_or_else(|| "upload.bin".to_string());
        let mime_type = field.content_type().map(str::to_string).unwrap_or_else(|| {
            mime_guess::from_path(&original_name)
                .first_or_octet_stream()
                .to_string()
        });
        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("读取上传文件失败: {e}")))?;
        if data.is_empty() {
            return Err(AppError::BadRequest("上传文件为空".into()));
        }
        if data.len() > MAX_UPLOAD_BYTES {
            return Err(AppError::BadRequest("单文件不能超过 50MB".into()));
        }

        let safe_name = sanitize_filename(&original_name);
        let file_type = infer_file_type(&safe_name, &mime_type);
        let extracted =
            file_extract::extract_text_from_bytes(&safe_name, &mime_type, data.as_ref());
        let extension = FsPath::new(&safe_name)
            .extension()
            .and_then(|value| value.to_str())
            .filter(|value| !value.trim().is_empty())
            .map(|value| format!(".{value}"))
            .unwrap_or_default();
        let storage_name = format!("{}{}", uuid::Uuid::new_v4(), extension);
        let storage_dir = user_file_dir(&user.0.id);
        tokio::fs::create_dir_all(&storage_dir).await?;
        let storage_path = storage_dir.join(storage_name);
        tokio::fs::write(&storage_path, data.as_ref()).await?;

        let file = file_repo::create_file(
            &pool,
            &user.0.id,
            &safe_name,
            &storage_path.to_string_lossy(),
            &file_type,
            data.len() as i64,
            folder_id.as_deref(),
            description.as_deref(),
            Some(json!({
                "mime_type": mime_type,
                "text_parser": extracted.parser,
                "text_truncated": extracted.truncated,
                "text_chars": extracted.text.chars().count(),
                "extracted_text": extracted.text,
            })),
        )?;
        return Ok(Json(json!({ "ok": true, "file": file })));
    }

    Err(AppError::BadRequest("没有找到 file 字段".into()))
}

async fn extract_file_text(
    _user: AuthUser,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, AppError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("上传表单解析失败: {e}")))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let original_name = field
            .file_name()
            .map(str::to_string)
            .or_else(|| header_value(&headers, "x-filename"))
            .unwrap_or_else(|| "upload.bin".to_string());
        let mime_type = field.content_type().map(str::to_string).unwrap_or_else(|| {
            mime_guess::from_path(&original_name)
                .first_or_octet_stream()
                .to_string()
        });
        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("读取上传文件失败: {e}")))?;
        if data.is_empty() {
            return Err(AppError::BadRequest("上传文件为空".into()));
        }
        if data.len() > MAX_UPLOAD_BYTES {
            return Err(AppError::BadRequest("单文件不能超过 50MB".into()));
        }
        let safe_name = sanitize_filename(&original_name);
        let extracted =
            file_extract::extract_text_from_bytes(&safe_name, &mime_type, data.as_ref());
        return Ok(Json(json!({
            "ok": !extracted.text.trim().is_empty() && extracted.parser != "unsupported",
            "name": safe_name,
            "mime_type": mime_type,
            "size": data.len(),
            "parser": extracted.parser,
            "truncated": extracted.truncated,
            "text": extracted.text,
        })));
    }

    Err(AppError::BadRequest("没有找到 file 字段".into()))
}

async fn get_file(
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = crate::state::db_pool();
    let file = file_repo::get_file(&pool, &user.0.id, &id)?
        .ok_or_else(|| AppError::NotFound("文件不存在".into()))?;
    Ok(Json(json!(file)))
}

async fn download_file(user: AuthUser, Path(id): Path<String>) -> Result<Response, AppError> {
    let pool = crate::state::db_pool();
    let file = file_repo::get_file(&pool, &user.0.id, &id)?
        .ok_or_else(|| AppError::NotFound("文件不存在".into()))?;
    let path = PathBuf::from(&file.file_path);
    if !path.exists() {
        return Err(AppError::NotFound("文件实体不存在".into()));
    }
    export_response(&path, &file.name)
}

/// 流式播放接口 — 支持 token 查询参数认证，返回 Content-Disposition: inline
/// 专门用于 <video>/<audio> 标签的 src URL，浏览器可边下载边播放
async fn stream_file(
    Path(id): Path<String>,
    Query(q): Query<StreamQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    // 优先从查询参数取 token，其次从 Authorization 头取
    let user = if let Some(token) = q.token.as_deref() {
        let claims = crate::auth::verify_token(token).map_err(|_| AppError::Unauthorized)?;
        let pool = crate::state::db_pool();
        crate::db::user_repo::find_by_id(&pool, &claims.sub)?
            .ok_or(AppError::Unauthorized)?
    } else {
        // 回退到 Header 认证
        let auth_header = headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Unauthorized)?;
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?;
        let claims = crate::auth::verify_token(token).map_err(|_| AppError::Unauthorized)?;
        let pool = crate::state::db_pool();
        crate::db::user_repo::find_by_id(&pool, &claims.sub)?
            .ok_or(AppError::Unauthorized)?
    };

    let pool = crate::state::db_pool();
    let file = file_repo::get_file(&pool, &user.id, &id)?
        .ok_or_else(|| AppError::NotFound("文件不存在".into()))?;
    let path = PathBuf::from(&file.file_path);
    if !path.exists() {
        return Err(AppError::NotFound("文件实体不存在".into()));
    }

    let data = tokio::fs::read(&path).await?;
    let mime = mime_guess::from_path(&file.name).first_or_octet_stream();

    // inline 而非 attachment，让浏览器直接播放/展示
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime.as_ref())
        .header(header::CONTENT_DISPOSITION, format!("inline; filename=\"{}\"", file.name))
        .header(header::CACHE_CONTROL, "private, max-age=3600")
        .header(header::ACCEPT_RANGES, "bytes")
        .body(Body::from(data))
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
}

async fn get_file_content(
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = crate::state::db_pool();
    let file = file_repo::get_file(&pool, &user.0.id, &id)?
        .ok_or_else(|| AppError::NotFound("文件不存在".into()))?;
    if let Some(metadata) = &file.metadata {
        if let Some(text) = metadata
            .get("extracted_text")
            .and_then(|value| value.as_str())
        {
            return Ok(Json(json!({
                "id": file.id,
                "name": file.name,
                "file_type": file.file_type,
                "text": text,
                "parser": metadata.get("text_parser"),
                "truncated": metadata.get("text_truncated").and_then(|value| value.as_bool()).unwrap_or(false),
            })));
        }
    }

    let path = PathBuf::from(&file.file_path);
    if !path.exists() {
        return Err(AppError::NotFound("文件实体不存在".into()));
    }
    let data = tokio::fs::read(&path).await?;
    let mime_type = file
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.get("mime_type"))
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| {
            mime_guess::from_path(&file.name)
                .first_or_octet_stream()
                .to_string()
        });
    let extracted = file_extract::extract_text_from_bytes(&file.name, &mime_type, &data);
    Ok(Json(json!({
        "id": file.id,
        "name": file.name,
        "file_type": file.file_type,
        "text": extracted.text,
        "parser": extracted.parser,
        "truncated": extracted.truncated,
    })))
}

async fn delete_file(
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = crate::state::db_pool();
    let deleted = file_repo::delete_file(&pool, &user.0.id, &id)?;
    if let Some(file) = deleted {
        let _ = tokio::fs::remove_file(&file.file_path).await;
        return Ok(Json(json!({ "deleted": true, "id": id })));
    }
    Ok(Json(json!({ "deleted": false, "id": id })))
}

/// 缩略图 — 图片类型返回缩略图字节，其他类型返回 JSON 元信息（前端用图标 + 文件色块代替）
async fn get_file_thumbnail(
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let pool = crate::state::db_pool();
    let file = file_repo::get_file(&pool, &user.0.id, &id)?
        .ok_or_else(|| AppError::NotFound("文件不存在".into()))?;

    // 图片类型：直接返回图片字节（前端用 CSS 缩放做缩略图）
    if file.file_type == "image" {
        let path = PathBuf::from(&file.file_path);
        if !path.exists() {
            return Err(AppError::NotFound("文件实体不存在".into()));
        }
        let data = tokio::fs::read(&path).await?;
        let mime = mime_guess::from_path(&file.name).first_or_octet_stream();
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime.as_ref())
            .header(header::CACHE_CONTROL, "public, max-age=3600")
            .body(Body::from(data))
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)));
    }

    // 非图片类型：返回 JSON 元信息，前端用图标渲染缩略图卡片
    let body = json!({
        "file_type": file.file_type,
        "name": file.name,
        "file_size": file.file_size,
    }).to_string();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
}

/// 预览 — 返回文件内容供前端渲染
/// 图片：返回图片字节（inline）
/// 文本/markdown/drawio：返回文本
/// xlsx/docx/pptx：返回提取的文本结构
async fn get_file_preview(
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = crate::state::db_pool();
    let file = file_repo::get_file(&pool, &user.0.id, &id)?
        .ok_or_else(|| AppError::NotFound("文件不存在".into()))?;

    let path = PathBuf::from(&file.file_path);
    if !path.exists() {
        return Err(AppError::NotFound("文件实体不存在".into()));
    }

    let extension = FsPath::new(&file.name)
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mime_type = file
        .metadata
        .as_ref()
        .and_then(|m| m.get("mime_type"))
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| {
            mime_guess::from_path(&file.name)
                .first_or_octet_stream()
                .to_string()
        });

    // 视频：返回视频流 URL（前端用 <video> 标签播放）
    if file.file_type == "video" || matches!(extension.as_str(), "mp4" | "webm" | "avi" | "mov" | "mkv" | "flv" | "wmv" | "m4v" | "3gp" | "ogv") {
        return Ok(Json(json!({
            "id": file.id,
            "name": file.name,
            "file_type": file.file_type,
            "preview_type": "video",
            "mime_type": mime_type,
            "video_url": format!("/api/files/{}/stream", file.id),
            "file_size": file.file_size,
        })));
    }

    // 图片：返回 base64 data URL
    if file.file_type == "image" || matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg") {
        let data = tokio::fs::read(&path).await?;
        let b64 = base64_encode(&data);
        return Ok(Json(json!({
            "id": file.id,
            "name": file.name,
            "file_type": file.file_type,
            "preview_type": "image",
            "mime_type": mime_type,
            "data_url": format!("data:{};base64,{}", mime_type, b64),
            "file_size": file.file_size,
        })));
    }

    // drawio / xml：返回 XML 文本
    if extension == "drawio" || extension == "xml" {
        let text = tokio::fs::read_to_string(&path).await?;
        return Ok(Json(json!({
            "id": file.id,
            "name": file.name,
            "file_type": file.file_type,
            "preview_type": "drawio",
            "text": text,
            "file_size": file.file_size,
        })));
    }

    // markdown / txt：返回文本
    if matches!(extension.as_str(), "md" | "markdown" | "txt") {
        let text = tokio::fs::read_to_string(&path).await?;
        return Ok(Json(json!({
            "id": file.id,
            "name": file.name,
            "file_type": file.file_type,
            "preview_type": "markdown",
            "text": text,
            "file_size": file.file_size,
        })));
    }

    // xlsx / docx / pptx / csv：返回结构化预览数据
    let data = tokio::fs::read(&path).await?;
    let structured = file_extract::extract_structured(&file.name, &mime_type, &data);
    let preview_type = match extension.as_str() {
        "xlsx" | "xls" | "csv" | "tsv" => "spreadsheet",
        "docx" | "doc" => "document",
        "pptx" | "ppt" => "presentation",
        "pdf" => "pdf",
        _ => "text",
    };

    // 对于结构化类型（presentation/spreadsheet/document），返回结构化 JSON
    if structured.preview_type == "presentation"
        || structured.preview_type == "spreadsheet"
        || structured.preview_type == "document"
    {
        return Ok(Json(json!({
            "id": file.id,
            "name": file.name,
            "file_type": file.file_type,
            "preview_type": structured.preview_type,
            "structured": structured.data,
            "parser": structured.parser,
            "truncated": structured.truncated,
            "file_size": file.file_size,
        })));
    }

    // 其他类型回退到文本
    Ok(Json(json!({
        "id": file.id,
        "name": file.name,
        "file_type": file.file_type,
        "preview_type": preview_type,
        "text": structured.data.get("text").and_then(|v| v.as_str()).unwrap_or(""),
        "parser": structured.parser,
        "truncated": structured.truncated,
        "file_size": file.file_size,
    })))
}

async fn list_folders(
    user: AuthUser,
    Query(q): Query<FolderQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = crate::state::db_pool();
    if let Some(parent_id) = q.parent_id.as_deref() {
        ensure_folder_owner(&pool, &user.0.id, parent_id)?;
    }
    let folders = file_repo::list_folders(&pool, &user.0.id, q.parent_id.as_deref())?;
    Ok(Json(json!({ "folders": folders })))
}

async fn create_folder(
    user: AuthUser,
    Json(payload): Json<CreateFolderReq>,
) -> Result<Json<serde_json::Value>, AppError> {
    let name = sanitize_folder_name(&payload.name)?;
    let pool = crate::state::db_pool();
    if let Some(parent_id) = payload.parent_id.as_deref() {
        ensure_folder_owner(&pool, &user.0.id, parent_id)?;
    }
    let folder = file_repo::create_folder(&pool, &user.0.id, &name, payload.parent_id.as_deref())?;
    Ok(Json(json!({ "ok": true, "folder": folder })))
}

async fn delete_folder(
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = crate::state::db_pool();
    let existed = file_repo::get_folder(&pool, &user.0.id, &id)?.is_some();
    let removed_files = file_repo::delete_folder_tree(&pool, &user.0.id, &id)?;
    for file in removed_files {
        let _ = tokio::fs::remove_file(&file.file_path).await;
    }
    Ok(Json(json!({ "deleted": existed, "id": id })))
}

fn ensure_folder_owner(
    pool: &crate::db::DbPool,
    owner_id: &str,
    folder_id: &str,
) -> Result<(), AppError> {
    file_repo::get_folder(pool, owner_id, folder_id)?
        .ok_or_else(|| AppError::NotFound("文件夹不存在".into()))?;
    Ok(())
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn user_file_dir(owner_id: &str) -> PathBuf {
    PathBuf::from(&crate::config::config().data_dir)
        .join("files")
        .join(owner_id)
}

fn export_response(path: &FsPath, filename: &str) -> Result<Response, AppError> {
    let data = std::fs::read(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let encoded = urlencoding::encode(filename);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime.as_ref())
        .header(
            header::CONTENT_DISPOSITION,
            format!(
                "attachment; filename=\"{}\"; filename*=UTF-8''{}",
                filename, encoded
            ),
        )
        .body(Body::from(data))
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
}

fn sanitize_filename(name: &str) -> String {
    let cleaned = name
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .collect::<String>()
        .trim()
        .to_string();
    if cleaned.is_empty() {
        "upload.bin".to_string()
    } else {
        cleaned
    }
}

fn sanitize_folder_name(name: &str) -> Result<String, AppError> {
    let cleaned = name
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .collect::<String>()
        .trim()
        .to_string();
    if cleaned.is_empty() {
        return Err(AppError::BadRequest("文件夹名称不能为空".into()));
    }
    Ok(cleaned)
}

fn infer_file_type(name: &str, mime_type: &str) -> String {
    let extension = FsPath::new(name)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase();
    match extension.as_str() {
        "ppt" | "pptx" => "ppt".into(),
        "doc" | "docx" | "md" | "markdown" | "txt" | "pdf" => "doc".into(),
        "xls" | "xlsx" | "csv" | "tsv" => "excel".into(),
        "drawio" | "xml" => "drawio".into(),
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" => "image".into(),
        "mp4" | "webm" | "avi" | "mov" | "mkv" | "flv" | "wmv" | "m4v" | "3gp" | "ogv" => "video".into(),
        _ if mime_type.starts_with("image/") => "image".into(),
        _ if mime_type.starts_with("video/") => "video".into(),
        _ if mime_type.contains("spreadsheet") || mime_type.contains("excel") => "excel".into(),
        _ if mime_type.contains("presentation") => "ppt".into(),
        _ if mime_type.contains("pdf")
            || mime_type.contains("word")
            || mime_type.starts_with("text/") =>
        {
            "doc".into()
        }
        _ => "other".into(),
    }
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        result.push(CHARS[(b[0] >> 2) as usize] as char);
        result.push(CHARS[((b[0] & 0x03) << 4 | b[1] >> 4) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((b[1] & 0x0f) << 2 | b[2] >> 6) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(b[2] & 0x3f) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}
