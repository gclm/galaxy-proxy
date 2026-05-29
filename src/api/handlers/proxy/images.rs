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

/// OpenAI Images 代理
pub async fn proxy(
    State(state): State<ProxyState>,
    auth: ApiKeyAuth,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    proxy::handle_proxy_request(
        &state,
        auth,
        headers,
        body,
        &EndpointType::OpenAiImages,
        &proxy::ErrorFormat::OpenAi,
    ).await
}
