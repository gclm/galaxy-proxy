use axum::{extract::State, http::{HeaderMap, StatusCode}, response::IntoResponse, Json};
use serde_json::Value;

use crate::proxy::ProxyState;

/// OpenAI Responses 代理
pub async fn proxy(
    State(state): State<ProxyState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let model = body["model"].as_str().unwrap_or("unknown");
    let is_stream = body["stream"].as_bool().unwrap_or(false);

    // 获取 session_hash
    let session_hash = headers.get("x-session-hash")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .or_else(|| body["session_hash"].as_str().map(|s| s.to_string()));

    let selection = match state.select_channel(model, session_hash.as_deref()).await {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({
                "error": { "message": e.to_string(), "type": "server_error" }
            }))).into_response();
        }
    };

    let mut request_body = body.clone();
    request_body["model"] = serde_json::Value::String(selection.target_model.clone());

    let api_key = state.select_api_key(&selection.channel);

    let mut reqwest_headers = reqwest::header::HeaderMap::new();
    reqwest_headers.insert("Content-Type", "application/json".parse().unwrap());
    reqwest_headers.insert("Authorization", format!("Bearer {}", api_key).parse().unwrap());

    let url = format!("{}/v1/responses", selection.channel.base_url);

    if is_stream {
        match state.http_client.post(&url)
            .headers(reqwest_headers)
            .body(request_body.to_string())
            .send()
            .await
        {
            Ok(response) => {
                if !response.status().is_success() {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    return (status, Json(serde_json::json!({
                        "error": { "message": body, "type": "server_error" }
                    }))).into_response();
                }

                let stream = response.bytes_stream();
                use futures::StreamExt;

                let response_stream = async_stream::stream! {
                    let mut stream = std::pin::pin!(stream);
                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(bytes) => yield Ok::<_, std::convert::Infallible>(axum::body::Bytes::from(bytes)),
                            Err(e) => {
                                tracing::error!("Stream error: {}", e);
                                break;
                            }
                        }
                    }
                };

                axum::response::Response::builder()
                    .status(200)
                    .header("Content-Type", "text/event-stream")
                    .header("Cache-Control", "no-cache")
                    .header("Connection", "keep-alive")
                    .body(axum::body::Body::from_stream(response_stream))
                    .unwrap()
                    .into_response()
            }
            Err(e) => {
                (StatusCode::BAD_GATEWAY, Json(serde_json::json!({
                    "error": { "message": format!("请求上游失败: {}", e), "type": "server_error" }
                }))).into_response()
            }
        }
    } else {
        match state.http_client.post(&url)
            .headers(reqwest_headers)
            .body(request_body.to_string())
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();

                if !status.is_success() {
                    return (status, Json(serde_json::json!({
                        "error": { "message": body, "type": "server_error" }
                    }))).into_response();
                }

                (StatusCode::OK, axum::response::Html(body)).into_response()
            }
            Err(e) => {
                (StatusCode::BAD_GATEWAY, Json(serde_json::json!({
                    "error": { "message": format!("请求上游失败: {}", e), "type": "server_error" }
                }))).into_response()
            }
        }
    }
}
