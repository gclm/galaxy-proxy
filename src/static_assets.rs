use axum::{
    body::Body,
    http::{header, HeaderValue, Request, Response, StatusCode},
};
use rust_embed::Embed;
use std::sync::LazyLock;

#[derive(Embed)]
#[folder = "frontend/dist/"]
pub struct StaticAssets;

static FALLBACK: &str = "index.html";

pub async fn serve(req: Request<Body>) -> Response<Body> {
    let path = req.uri().path();

    // 跳过 API 路由（不应该到这里）
    if path.starts_with("/api/") || path.starts_with("/v1/") {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap();
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
            && let Some(index) = StaticAssets::get(FALLBACK) {
                return Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                    .body(Body::from(index.data))
                    .unwrap();
            }
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap();
    };

    let content_type = mime_guess::from_path(file_path)
        .first_or_octet_stream()
        .to_string();

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CACHE_CONTROL,
            if file_path == FALLBACK {
                HeaderValue::from_static("no-cache")
            } else {
                static CACHE_ONE_YEAR: LazyLock<HeaderValue> = LazyLock::new(|| {
                    HeaderValue::from_static("public, max-age=31536000, immutable")
                });
                CACHE_ONE_YEAR.clone()
            },
        )
        .body(Body::from(asset.data))
        .unwrap()
}
