use axum::{extract::State, http::StatusCode, Json};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::api::{ApiError, ApiResponse};

/// 测试模型请求
#[derive(Debug, Deserialize)]
pub struct TestModelRequest {
    pub model: String,
    pub test_protocol: String,
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
    pub server_addr: String,
}

const TEST_PROMPT: &str = "Hello! Please respond with a brief greeting in one sentence.";

/// 测试协议对应的 (代理路由, 请求体构建器, 响应解析器)
fn get_test_config(protocol: &str, model: &str) -> Option<(&'static str, serde_json::Value, &'static str)> {
    match protocol {
        "openai_chat" => Some((
            "/v1/chat/completions",
            serde_json::json!({
                "model": model,
                "messages": [{"role": "user", "content": TEST_PROMPT}],
                "max_tokens": 100,
                "stream": false
            }),
            "openai_chat",
        )),
        "openai_response" => Some((
            "/v1/responses",
            serde_json::json!({
                "model": model,
                "input": TEST_PROMPT,
                "max_output_tokens": 100
            }),
            "openai_response",
        )),
        "anthropic" => Some((
            "/v1/messages",
            serde_json::json!({
                "model": model,
                "messages": [{"role": "user", "content": TEST_PROMPT}],
                "max_tokens": 100
            }),
            "anthropic",
        )),
        "openai_embedding" => Some((
            "/v1/embeddings",
            serde_json::json!({
                "model": model,
                "input": TEST_PROMPT
            }),
            "embedding",
        )),
        "openai_images" => Some((
            "/v1/images/generations",
            serde_json::json!({
                "model": model,
                "prompt": TEST_PROMPT,
                "n": 1,
                "size": "256x256"
            }),
            "images",
        )),
        _ => None,
    }
}

fn extract_content(resp_body: &serde_json::Value, protocol: &str) -> String {
    match protocol {
        "openai_chat" => resp_body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("(无内容)")
            .to_string(),
        "openai_response" => resp_body["output"][0]["content"][0]["text"]
            .as_str()
            .unwrap_or("(无内容)")
            .to_string(),
        "anthropic" => resp_body["content"][0]["text"]
            .as_str()
            .unwrap_or("(无内容)")
            .to_string(),
        "embedding" => {
            let len = resp_body["data"].as_array().map(|a| a.len()).unwrap_or(0);
            format!("Embedding 返回 {} 条向量数据", len)
        }
        "images" => {
            let count = resp_body["data"].as_array().map(|a| a.len()).unwrap_or(0);
            format!("图片生成成功，共 {} 张", count)
        }
        _ => "(未知协议)".to_string(),
    }
}

/// 测试模型 — 通过代理自身路由
pub async fn test_model(
    State(state): State<TestModelState>,
    Json(req): Json<TestModelRequest>,
) -> Result<Json<ApiResponse<TestModelResponse>>, (StatusCode, Json<ApiError>)> {
    let (path, body, protocol) = get_test_config(&req.test_protocol, &req.model)
        .ok_or_else(|| ApiError::bad_request(format!("不支持的测试协议: {}", req.test_protocol)))?;

    // 查找可用的 API Key
    let api_key: Option<String> = sqlx::query_scalar("SELECT api_key FROM api_keys WHERE enabled = 1 LIMIT 1")
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let api_key = api_key.ok_or_else(|| ApiError::bad_request("请先创建并启用一个 API Key 用于测试"))?;

    let url = format!("http://{}{}", state.server_addr, path);
    let start = std::time::Instant::now();

    let resp = state.http_client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
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
                    message: format!("代理返回 HTTP {}: {}", status, resp_text),
                    latency_ms,
                    input_prompt: TEST_PROMPT.to_string(),
                    output_content: None,
                })));
            }

            let resp_body: serde_json::Value = serde_json::from_str(&resp_text).unwrap_or_default();
            let has_error = resp_body.get("error").is_some();
            if has_error {
                let error_msg = resp_body["error"]["message"]
                    .as_str()
                    .unwrap_or("未知错误");
                return Ok(Json(ApiResponse::success(TestModelResponse {
                    success: false,
                    message: format!("模型返回错误: {}", error_msg),
                    latency_ms,
                    input_prompt: TEST_PROMPT.to_string(),
                    output_content: None,
                })));
            }

            let content = extract_content(&resp_body, protocol);
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
            message: format!("请求代理失败: {}", e),
            latency_ms,
            input_prompt: TEST_PROMPT.to_string(),
            output_content: None,
        }))),
    }
}
