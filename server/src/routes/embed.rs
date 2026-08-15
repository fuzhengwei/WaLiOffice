use axum::body::Body;
use axum::http::header;
use axum::http::{StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../frontend/dist/"]
struct WebAsset;

/// SPA fallback handler：匹配所有非 /api 路由
pub async fn fallback_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    // 尝试精确匹配静态文件
    if !path.is_empty() {
        if let Some(asset) = WebAsset::get(path) {
            return serve_asset(path, asset);
        }
    }

    // SPA fallback → index.html
    if let Some(index) = WebAsset::get("index.html") {
        return serve_asset("index.html", index);
    }

    // 前端未构建时返回 API 信息
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "name": "WaLiOffice",
                "version": "0.2.0",
                "message": "前端未构建。请先运行 npm run build 构建 frontend/dist/，或访问 /api/health",
                "docs": "/api/health",
            })
            .to_string(),
        ))
        .unwrap()
}

fn serve_asset(path: &str, asset: rust_embed::EmbeddedFile) -> Response {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let body = Body::from(asset.data.into_owned());
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime.as_ref())
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(body)
        .unwrap()
}
