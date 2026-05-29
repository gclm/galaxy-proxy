use axum::{Json, extract::State, http::StatusCode};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::api::handlers::admin::channels::{CustomHeader, EndpointConfig, EndpointType};
use crate::api::{ApiError, ApiResponse};

/// 测试模型请求
#[derive(Debug, Deserialize)]
pub struct TestModelRequest {
    pub channel_id: String,
    pub model: String,
    pub test_protocol: String,
    pub user_agent: Option<String>,
}

/// 测试模型响应
#[derive(Debug, Serialize)]
pub struct TestModelResponse {
    pub success: bool,
    pub message: String,
    pub latency_ms: u64,
    pub input_prompt: String,
    pub output_content: Option<String>,
}

/// 模型测试服务状态
#[derive(Clone)]
pub struct TestModelState {
    pub http_client: Client,
    pub pool: SqlitePool,
}

const TEST_PROMPT: &str = "Hello! Please respond with a brief greeting in one sentence.";

/// 测试协议对应的请求体和上游路径
fn get_test_config(
    protocol: &EndpointType,
    model: &str,
) -> Option<(serde_json::Value, &'static str)> {
    match protocol {
        EndpointType::OpenAiChat => Some((
            serde_json::json!({
                "model": model,
                "messages": [{"role": "user", "content": TEST_PROMPT}],
                "max_tokens": 100,
                "stream": false
            }),
            "/chat/completions",
        )),
        EndpointType::OpenAiResponse => Some((
            serde_json::json!({
                "model": model,
                "input": TEST_PROMPT,
                "max_output_tokens": 100
            }),
            "/responses",
        )),
        EndpointType::Anthropic => Some((
            serde_json::json!({
                "model": model,
                "messages": [{"role": "user", "content": TEST_PROMPT}],
                "max_tokens": 100
            }),
            "/messages",
        )),
        EndpointType::OpenAiEmbedding => Some((
            serde_json::json!({
                "model": model,
                "input": TEST_PROMPT
            }),
            "/embeddings",
        )),
        EndpointType::OpenAiImages => Some((
            serde_json::json!({
                "model": model,
                "prompt": TEST_PROMPT,
                "n": 1,
                "size": "256x256"
            }),
            "/images/generations",
        )),
        _ => None,
    }
}

fn extract_content(resp_body: &serde_json::Value, endpoint_type: &EndpointType) -> String {
    match endpoint_type {
        EndpointType::OpenAiChat => resp_body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("(无内容)")
            .to_string(),
        EndpointType::OpenAiResponse => resp_body["output"][0]["content"][0]["text"]
            .as_str()
            .unwrap_or("(无内容)")
            .to_string(),
        EndpointType::Anthropic => resp_body["content"][0]["text"]
            .as_str()
            .unwrap_or("(无内容)")
            .to_string(),
        EndpointType::OpenAiEmbedding => {
            let len = resp_body["data"].as_array().map(|a| a.len()).unwrap_or(0);
            format!("Embedding 返回 {} 条向量数据", len)
        }
        EndpointType::OpenAiImages => {
            let count = resp_body["data"].as_array().map(|a| a.len()).unwrap_or(0);
            format!("图片生成成功，共 {} 张", count)
        }
        _ => "(未知协议)".to_string(),
    }
}

/// 注入自定义请求头到 reqwest builder
fn inject_custom_headers(
    req_builder: reqwest::RequestBuilder,
    headers: &[CustomHeader],
) -> reqwest::RequestBuilder {
    let mut builder = req_builder;
    for header in headers {
        if let Ok(name) = reqwest::header::HeaderName::from_bytes(header.key.as_bytes())
            && let Ok(value) = header.value.parse::<reqwest::header::HeaderValue>()
        {
            builder = builder.header(name, value);
        }
    }
    builder
}

/// 解析测试协议字符串为 EndpointType
fn parse_protocol(protocol: &str) -> Option<EndpointType> {
    serde_json::from_value::<EndpointType>(serde_json::Value::String(protocol.to_string())).ok()
}

/// 测试模型 — 直接发送到渠道上游，绕过代理路由
pub async fn test_model(
    State(state): State<TestModelState>,
    Json(req): Json<TestModelRequest>,
) -> Result<Json<ApiResponse<TestModelResponse>>, (StatusCode, Json<ApiError>)> {
    let endpoint_type = parse_protocol(&req.test_protocol)
        .ok_or_else(|| ApiError::bad_request(format!("不支持的测试协议: {}", req.test_protocol)))?;

    let (body, upstream_path) = get_test_config(&endpoint_type, &req.model)
        .ok_or_else(|| ApiError::bad_request(format!("不支持的测试协议: {}", req.test_protocol)))?;

    // 查询渠道（只需 api_keys, endpoints, custom_headers）
    let row = sqlx::query_as::<_, (String, String, String)>(
        "SELECT api_keys, endpoints, custom_headers FROM channels WHERE id = ? AND enabled = 1",
    )
    .bind(&req.channel_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let (api_keys_str, endpoints_str, headers_str) =
        row.ok_or_else(|| ApiError::bad_request("渠道不存在或已禁用"))?;

    let api_keys: Vec<String> = serde_json::from_str(&api_keys_str).unwrap_or_default();
    let endpoints: Vec<EndpointConfig> = serde_json::from_str(&endpoints_str).unwrap_or_default();
    let custom_headers: Vec<CustomHeader> = serde_json::from_str(&headers_str).unwrap_or_default();

    let endpoint = endpoints
        .iter()
        .find(|e| e.endpoint_type == endpoint_type)
        .or_else(|| endpoints.first())
        .ok_or_else(|| ApiError::bad_request("渠道没有可用端点"))?;

    let upstream_api_key = api_keys
        .first()
        .ok_or_else(|| ApiError::bad_request("渠道没有配置 API Key"))?;

    let url = format!(
        "{}{}",
        endpoint.base_url.trim_end_matches('/'),
        upstream_path
    );
    let start = std::time::Instant::now();

    let mut req_builder = state
        .http_client
        .post(&url)
        .header("Content-Type", "application/json");

    match endpoint_type {
        EndpointType::Anthropic => {
            req_builder = req_builder
                .header("x-api-key", upstream_api_key.as_str())
                .header("anthropic-version", "2023-06-01");
        }
        _ => {
            req_builder =
                req_builder.header("Authorization", format!("Bearer {}", upstream_api_key));
        }
    }

    req_builder = inject_custom_headers(req_builder, &custom_headers);

    if let Some(ua) = &req.user_agent
        && !ua.is_empty()
    {
        req_builder = req_builder.header("User-Agent", ua.as_str());
    }

    let resp = req_builder
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await;

    let latency_ms = start.elapsed().as_millis() as u64;

    match resp {
        Ok(resp) => {
            let status = resp.status();
            let resp_text = resp.text().await.unwrap_or_default();

            if !status.is_success() {
                return Ok(Json(ApiResponse::success(TestModelResponse {
                    success: false,
                    message: format!("上游返回 HTTP {}: {}", status, resp_text),
                    latency_ms,
                    input_prompt: TEST_PROMPT.to_string(),
                    output_content: None,
                })));
            }

            let resp_body: serde_json::Value = serde_json::from_str(&resp_text).unwrap_or_default();
            if resp_body.get("error").is_some() {
                let error_msg = resp_body["error"]["message"].as_str().unwrap_or("未知错误");
                return Ok(Json(ApiResponse::success(TestModelResponse {
                    success: false,
                    message: format!("模型返回错误: {}", error_msg),
                    latency_ms,
                    input_prompt: TEST_PROMPT.to_string(),
                    output_content: None,
                })));
            }

            let content = extract_content(&resp_body, &endpoint_type);
            Ok(Json(ApiResponse::success(TestModelResponse {
                success: true,
                message: "模型测试成功".to_string(),
                latency_ms,
                input_prompt: TEST_PROMPT.to_string(),
                output_content: Some(content),
            })))
        }
        Err(e) => Ok(Json(ApiResponse::success(TestModelResponse {
            success: false,
            message: format!("请求上游失败: {}", e),
            latency_ms,
            input_prompt: TEST_PROMPT.to_string(),
            output_content: None,
        }))),
    }
}
