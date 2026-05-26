use axum::{
    extract::State,
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use serde_json::Value;

use crate::api::handlers::admin::channels::EndpointType;
use crate::api::middleware::ApiKeyAuth;
use crate::proxy::{self, ProxyState};

/// OpenAI Chat Completions 代理
pub async fn proxy(
    State(state): State<ProxyState>,
    _auth: ApiKeyAuth,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let is_stream = body["stream"].as_bool().unwrap_or(false);
    let client_endpoint = EndpointType::OpenAiChat;

    if is_stream {
        match proxy::proxy_stream(&state, &headers, &body, &client_endpoint).await {
            Ok((status, stream, content_type)) => axum::response::Response::builder()
                .status(status)
                .header("Content-Type", content_type)
                .header("Cache-Control", "no-cache")
                .header("Connection", "keep-alive")
                .body(axum::body::Body::from_stream(stream))
                .unwrap()
                .into_response(),
            Err(e) => proxy::format_proxy_error(e, &proxy::ErrorFormat::OpenAi),
        }
    } else {
        match proxy::proxy_request(&state, &headers, &body, &client_endpoint).await {
            Ok(result) => axum::response::Response::builder()
                .status(result.status)
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(result.body))
                .unwrap()
                .into_response(),
            Err(e) => proxy::format_proxy_error(e, &proxy::ErrorFormat::OpenAi),
        }
    }
}
