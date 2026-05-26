use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde_json::Value;

use crate::api::handlers::admin::channels::EndpointType;
use crate::proxy::ProxyState;

/// OpenAI Chat Completions 代理
pub async fn proxy(
    State(state): State<ProxyState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let model = body["model"].as_str().unwrap_or("unknown");
    let is_stream = body["stream"].as_bool().unwrap_or(false);

    // 获取 session_hash
    let session_hash = headers
        .get("x-session-hash")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .or_else(|| body["session_hash"].as_str().map(|s| s.to_string()));

    // 选择渠道
    let selection = match state
        .select_channel(model, EndpointType::OpenAiChat, session_hash.as_deref())
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "error": { "message": e.to_string(), "type": "server_error" }
                })),
            )
                .into_response();
        }
    };

    // 应用模型映射
    let mut request_body = body.clone();
    request_body["model"] = serde_json::Value::String(selection.target_model.clone());

    // 获取 API Key
    let api_key = state.select_api_key(&selection.channel);

    // 构建请求头
    let mut reqwest_headers = reqwest::header::HeaderMap::new();
    reqwest_headers.insert("Content-Type", "application/json".parse().unwrap());
    reqwest_headers.insert(
        "Authorization",
        format!("Bearer {}", api_key).parse().unwrap(),
    );

    // 构建 URL
    let url = format!(
        "{}{}",
        selection.endpoint.base_url,
        EndpointType::OpenAiChat.path()
    );

    let start_time = std::time::Instant::now();
    let channel_id = selection.channel.id.clone();

    // 发送请求
    if is_stream {
        // 流式响应
        match state
            .http_client
            .post(&url)
            .headers(reqwest_headers)
            .body(request_body.to_string())
            .send()
            .await
        {
            Ok(response) => {
                if !response.status().is_success() {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();

                    // 记录失败
                    state
                        .lb_state
                        .record_failure(&channel_id, status.is_server_error())
                        .await;

                    return (
                        status,
                        Json(serde_json::json!({
                            "error": { "message": body, "type": "server_error" }
                        })),
                    )
                        .into_response();
                }

                // 记录成功
                state
                    .lb_state
                    .record_success(&channel_id, start_time.elapsed().as_millis() as f64)
                    .await;

                // 流式转发
                let stream = response.bytes_stream();
                use futures::StreamExt;

                let response_stream = async_stream::stream! {
                    let mut stream = std::pin::pin!(stream);
                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(bytes) => yield Ok::<_, std::convert::Infallible>(bytes),
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
                // 记录失败
                state.lb_state.record_failure(&channel_id, true).await;

                (StatusCode::BAD_GATEWAY, Json(serde_json::json!({
                    "error": { "message": format!("请求上游失败: {}", e), "type": "server_error" }
                }))).into_response()
            }
        }
    } else {
        // 非流式响应
        match state
            .http_client
            .post(&url)
            .headers(reqwest_headers)
            .body(request_body.to_string())
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();

                if !status.is_success() {
                    // 记录失败
                    state
                        .lb_state
                        .record_failure(&channel_id, status.is_server_error())
                        .await;

                    return (
                        status,
                        Json(serde_json::json!({
                            "error": { "message": body, "type": "server_error" }
                        })),
                    )
                        .into_response();
                }

                // 记录成功
                state
                    .lb_state
                    .record_success(&channel_id, start_time.elapsed().as_millis() as f64)
                    .await;

                // 直接返回上游响应
                (StatusCode::OK, axum::response::Html(body)).into_response()
            }
            Err(e) => {
                // 记录失败
                state.lb_state.record_failure(&channel_id, true).await;

                (StatusCode::BAD_GATEWAY, Json(serde_json::json!({
                    "error": { "message": format!("请求上游失败: {}", e), "type": "server_error" }
                }))).into_response()
            }
        }
    }
}
