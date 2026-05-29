use axum::{
    body::Body,
    http::{Request, StatusCode, header},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;
use std::sync::LazyLock;

#[derive(Embed)]
#[folder = "frontend/dist/"]
pub struct StaticAssets;

static FALLBACK: &str = "index.html";
static CACHE_IMMUTABLE: LazyLock<String> =
    LazyLock::new(|| "public, max-age=31536000, immutable".to_string());

pub async fn serve(req: Request<Body>) -> Response {
    let path = req.uri().path();

    // 跳过 API 路由（不应该到这里）
    if path.starts_with("/api/") || path.starts_with("/v1/") {
        return (StatusCode::NOT_FOUND, Body::empty()).into_response();
    }

    // SPA fallback: 返回 index.html
    let file_path = if path == "/" || path.is_empty() || !path.contains('.') {
        FALLBACK
    } else {
        path.trim_start_matches('/')
    };

    let asset = StaticAssets::get(file_path);
    let Some(asset) = asset else {
        // 尝试 fallback 到 index.html
        if file_path != FALLBACK
            && let Some(index) = StaticAssets::get(FALLBACK)
        {
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                Body::from(index.data),
            )
                .into_response();
        }
        return (StatusCode::NOT_FOUND, Body::empty()).into_response();
    };

    let content_type = mime_guess::from_path(file_path)
        .first_or_octet_stream()
        .to_string();

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, content_type),
            (
                header::CACHE_CONTROL,
                if file_path == FALLBACK {
                    "no-cache".to_string()
                } else {
                    CACHE_IMMUTABLE.clone()
                },
            ),
        ],
        Body::from(asset.data),
    )
        .into_response()
}
