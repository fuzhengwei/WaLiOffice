use axum::body::Body;
use axum::extract::Path;
use axum::http::header;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;

use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use crate::render;

pub fn router() -> Router {
    Router::new()
        .route("/api/doc/export", post(export_docx))
        .route("/api/excel/export", post(export_xlsx))
        .route("/api/files/download/:filename", get(download_file))
}

#[derive(Deserialize)]
struct DocExportReq {
    title: String,
    sections: Vec<render::docx_render::DocSection>,
}

#[derive(Deserialize)]
struct SheetExportReq {
    title: String,
    tables: Vec<render::xlsx_render::SheetTable>,
}

async fn export_docx(_user: AuthUser, Json(req): Json<DocExportReq>) -> Result<Response, AppError> {
    let data = render::docx_render::DocData {
        title: req.title.clone(),
        sections: req.sections,
    };
    let filename = format!("{}.docx", sanitize_filename(&req.title));
    let path = render::output_path(&filename);
    render::docx_render::render_docx(&data, &path)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    Ok(export_response(&path, &filename))
}

async fn export_xlsx(_user: AuthUser, Json(req): Json<SheetExportReq>) -> Result<Response, AppError> {
    let data = render::xlsx_render::SheetData {
        title: req.title.clone(),
        tables: req.tables,
    };
    let filename = format!("{}.xlsx", sanitize_filename(&req.title));
    let path = render::output_path(&filename);
    render::xlsx_render::render_xlsx(&data, &path)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    Ok(export_response(&path, &filename))
}

async fn download_file(
    _user: AuthUser,
    Path(filename): Path<String>,
) -> Result<Response, AppError> {
    // 防路径穿越：只取纯文件名（去除目录/分隔符），并在 render 目录下解析
    let base = std::path::Path::new(&filename)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    if base.is_empty() || base != filename {
        return Err(AppError::NotFound("文件不存在".into()));
    }
    let path = render::output_path(&base);
    if !path.exists() {
        return Err(AppError::NotFound("文件不存在".into()));
    }
    Ok(export_response(&path, &base))
}

fn export_response(path: &std::path::Path, filename: &str) -> Response {
    let data = std::fs::read(path).unwrap_or_default();
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
        .unwrap()
}

fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .collect();
    if cleaned.is_empty() {
        "output".to_string()
    } else {
        cleaned
    }
}
