use axum::{extract::State, http::StatusCode, Json};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::api::handlers::admin::channels::{EndpointConfig, EndpointType};
use crate::api::{ApiError, ApiResponse};

/// 测试模型请求
#[derive(Debug, Deserialize)]
pub struct TestModelRequest {
    pub endpoint: EndpointConfig,
    pub api_key: String,
    pub model: String,
}

/// 测试模型响应
#[derive(Debug, Serialize)]
pub struct TestModelResponse {
    pub success: bool,
    pub message: String,
    pub latency_ms: u64,
}

/// 模型测试服务状态
#[derive(Clone)]
pub struct TestModelState {
    pub http_client: Client,
}

/// 测试模型
pub async fn test_model(
    State(state): State<TestModelState>,
    Json(req): Json<TestModelRequest>,
) -> Result<Json<ApiResponse<TestModelResponse>>, (StatusCode, Json<ApiError>)> {
    let start = std::time::Instant::now();

    let result = match req.endpoint.endpoint_type {
        EndpointType::Anthropic => {
            test_anthropic_model(&state.http_client, &req.endpoint.base_url, &req.api_key, &req.model).await
        }
        EndpointType::Gemini => {
            test_gemini_model(&state.http_client, &req.endpoint.base_url, &req.api_key, &req.model).await
        }
        _ => {
            test_openai_model(&state.http_client, &req.endpoint.base_url, &req.api_key, &req.model).await
        }
    };

    let latency_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(_) => Ok(Json(ApiResponse::success(TestModelResponse {
            success: true,
            message: "模型测试成功".to_string(),
            latency_ms,
        }))),
        Err(e) => Ok(Json(ApiResponse::success(TestModelResponse {
            success: false,
            message: format!("模型测试失败: {}", e),
            latency_ms,
        }))),
    }
}

/// 测试 OpenAI 兼容模型
async fn test_openai_model(
    client: &Client,
    base_url: &str,
    api_key: &str,
    model: &str,
) -> Result<(), String> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": "Hi"}],
        "max_tokens": 5,
        "stream": false
    });

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status() == 401 || resp.status() == 403 {
        return Err("API Key 无效".to_string());
    }

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, text));
    }

    Ok(())
}

/// 测试 Anthropic 模型
async fn test_anthropic_model(
    client: &Client,
    base_url: &str,
    api_key: &str,
    model: &str,
) -> Result<(), String> {
    let url = format!("{}/messages", base_url.trim_end_matches('/'));

    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": "Hi"}],
        "max_tokens": 5
    });

    let resp = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status() == 401 || resp.status() == 403 {
        return Err("API Key 无效".to_string());
    }

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, text));
    }

    Ok(())
}

/// 测试 Gemini 模型
async fn test_gemini_model(
    client: &Client,
    base_url: &str,
    api_key: &str,
    model: &str,
) -> Result<(), String> {
    let url = format!(
        "{}/models/{}:generateContent",
        base_url.trim_end_matches('/'),
        model
    );

    let body = serde_json::json!({
        "contents": [{"parts": [{"text": "Hi"}]}],
        "generationConfig": {"maxOutputTokens": 5}
    });

    let resp = client
        .post(&url)
        .header("x-goog-api-key", api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status() == 401 || resp.status() == 403 {
        return Err("API Key 无效".to_string());
    }

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, text));
    }

    Ok(())
}
