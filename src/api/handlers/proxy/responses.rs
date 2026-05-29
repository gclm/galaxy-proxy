use axum::{Json, extract::State, http::HeaderMap, response::IntoResponse};
use serde_json::Value;

use crate::api::handlers::admin::channels::EndpointType;
use crate::api::middleware::ApiKeyAuth;
use crate::proxy::{self, ProxyState};

/// OpenAI Responses 代理
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
        &EndpointType::OpenAiResponse,
        &proxy::ErrorFormat::OpenAi,
    )
    .await
}
