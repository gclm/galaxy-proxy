use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::api::{response::generate_id, ApiError, ApiResponse};

/// 端点类型
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EndpointType {
    #[serde(rename = "openai_chat")]
    OpenAiChat,
    #[serde(rename = "openai_response")]
    OpenAiResponse,
    Anthropic,
    Gemini,
    #[serde(rename = "openai_embedding")]
    OpenAiEmbedding,
    #[serde(rename = "openai_images")]
    OpenAiImages,
}

impl EndpointType {
    /// 获取端点路径
    pub fn path(&self) -> &'static str {
        match self {
            Self::OpenAiChat => "/chat/completions",
            Self::OpenAiResponse => "/responses",
            Self::Anthropic => "/messages",
            Self::Gemini => "/models/{model}:generateContent",
            Self::OpenAiEmbedding => "/embeddings",
            Self::OpenAiImages => "/images/generations",
        }
    }
}

/// 端点配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EndpointConfig {
    #[serde(rename = "type")]
    pub endpoint_type: EndpointType,
    pub base_url: String,
}

/// 渠道
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub api_keys: Vec<String>,
    pub endpoints: Vec<EndpointConfig>,
    pub model_maps: serde_json::Value,
    pub rate_limit_rpm: Option<i32>,
    pub rate_limit_tpm: Option<i32>,
    pub failure_threshold: i32,
    pub blacklist_minutes: i32,
    pub concurrency: i32,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建渠道请求
#[derive(Debug, Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    pub api_keys: Vec<String>,
    pub endpoints: Vec<EndpointConfig>,
    pub model_maps: Option<serde_json::Value>,
    pub rate_limit_rpm: Option<i32>,
    pub rate_limit_tpm: Option<i32>,
    pub failure_threshold: Option<i32>,
    pub blacklist_minutes: Option<i32>,
    pub concurrency: Option<i32>,
    pub enabled: Option<bool>,
}

/// 更新渠道请求
#[derive(Debug, Deserialize)]
pub struct UpdateChannelRequest {
    pub name: Option<String>,
    pub api_keys: Option<Vec<String>>,
    pub endpoints: Option<Vec<EndpointConfig>>,
    pub model_maps: Option<serde_json::Value>,
    pub rate_limit_rpm: Option<i32>,
    pub rate_limit_tpm: Option<i32>,
    pub failure_threshold: Option<i32>,
    pub blacklist_minutes: Option<i32>,
    pub concurrency: Option<i32>,
    pub enabled: Option<bool>,
}

/// 渠道状态
#[derive(Clone)]
pub struct ChannelState {
    pub pool: SqlitePool,
}

/// 获取渠道列表
pub async fn list(
    State(state): State<ChannelState>,
) -> Result<Json<ApiResponse<Vec<Channel>>>, (StatusCode, Json<ApiError>)> {
    let channels = sqlx::query_as::<_, (String, String, String, String, String, Option<i32>, Option<i32>, i32, i32, i32, bool, String, String)>(
        "SELECT id, name, api_keys, endpoints, model_maps, rate_limit_rpm, rate_limit_tpm, failure_threshold, blacklist_minutes, concurrency, enabled, created_at, updated_at FROM channels ORDER BY created_at DESC"
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let result: Vec<Channel> = channels
        .into_iter()
        .map(
            |(
                id,
                name,
                api_keys_str,
                endpoints_str,
                model_maps_str,
                rate_limit_rpm,
                rate_limit_tpm,
                failure_threshold,
                blacklist_minutes,
                concurrency,
                enabled,
                created_at,
                updated_at,
            )| {
                let api_keys: Vec<String> = serde_json::from_str(&api_keys_str).unwrap_or_default();
                let endpoints: Vec<EndpointConfig> =
                    serde_json::from_str(&endpoints_str).unwrap_or_default();
                let model_maps: serde_json::Value =
                    serde_json::from_str(&model_maps_str).unwrap_or_default();

                Channel {
                    id,
                    name,
                    api_keys,
                    endpoints,
                    model_maps,
                    rate_limit_rpm,
                    rate_limit_tpm,
                    failure_threshold,
                    blacklist_minutes,
                    concurrency,
                    enabled,
                    created_at,
                    updated_at,
                }
            },
        )
        .collect();

    Ok(Json(ApiResponse::success(result)))
}

/// 创建渠道
pub async fn create(
    State(state): State<ChannelState>,
    Json(req): Json<CreateChannelRequest>,
) -> Result<(StatusCode, Json<ApiResponse<Channel>>), (StatusCode, Json<ApiError>)> {
    // 验证输入
    if req.name.is_empty() {
        return Err(ApiError::bad_request("渠道名称不能为空"));
    }
    if req.api_keys.is_empty() {
        return Err(ApiError::bad_request("至少需要一个 API Key"));
    }
    if req.endpoints.is_empty() {
        return Err(ApiError::bad_request("至少需要一个端点"));
    }

    let id = generate_id();
    let api_keys_json = serde_json::to_string(&req.api_keys)
        .map_err(|e| ApiError::internal_error(e.to_string()))?;
    let endpoints_json = serde_json::to_string(&req.endpoints)
        .map_err(|e| ApiError::internal_error(e.to_string()))?;
    let model_maps_json = serde_json::to_string(&req.model_maps.unwrap_or_default())
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // 插入渠道
    sqlx::query(
        r#"
        INSERT INTO channels (id, name, api_keys, endpoints, model_maps, rate_limit_rpm, rate_limit_tpm, failure_threshold, blacklist_minutes, concurrency, enabled)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&api_keys_json)
    .bind(&endpoints_json)
    .bind(&model_maps_json)
    .bind(req.rate_limit_rpm)
    .bind(req.rate_limit_tpm)
    .bind(req.failure_threshold.unwrap_or(3))
    .bind(req.blacklist_minutes.unwrap_or(10))
    .bind(req.concurrency.unwrap_or(10))
    .bind(req.enabled.unwrap_or(true))
    .execute(&state.pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint failed") {
            ApiError::conflict("渠道名称已存在")
        } else {
            ApiError::internal_error(e.to_string())
        }
    })?;

    // 返回创建的渠道
    let channel = get_channel_by_id(&state.pool, &id).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::success(channel))))
}

/// 获取单个渠道
pub async fn get(
    State(state): State<ChannelState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Channel>>, (StatusCode, Json<ApiError>)> {
    let channel = get_channel_by_id(&state.pool, &id).await?;
    Ok(Json(ApiResponse::success(channel)))
}

/// 更新渠道
pub async fn update(
    State(state): State<ChannelState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateChannelRequest>,
) -> Result<Json<ApiResponse<Channel>>, (StatusCode, Json<ApiError>)> {
    // 检查渠道是否存在
    let existing = sqlx::query_scalar::<_, String>("SELECT id FROM channels WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    if existing.is_none() {
        return Err(ApiError::not_found("渠道不存在"));
    }

    // 构建更新语句
    let mut updates = Vec::new();
    let mut values: Vec<String> = Vec::new();

    if let Some(name) = &req.name {
        updates.push("name = ?");
        values.push(name.clone());
    }
    if let Some(api_keys) = &req.api_keys {
        updates.push("api_keys = ?");
        values.push(serde_json::to_string(api_keys).unwrap_or_default());
    }
    if let Some(endpoints) = &req.endpoints {
        updates.push("endpoints = ?");
        values.push(serde_json::to_string(endpoints).unwrap_or_default());
    }
    if let Some(model_maps) = &req.model_maps {
        updates.push("model_maps = ?");
        values.push(serde_json::to_string(model_maps).unwrap_or_default());
    }

    if updates.is_empty() {
        return Err(ApiError::bad_request("没有需要更新的字段"));
    }

    updates.push("updated_at = CURRENT_TIMESTAMP");

    // 构建动态 SQL
    let sql = format!("UPDATE channels SET {} WHERE id = ?", updates.join(", "));
    let sql: &'static str = Box::leak(sql.into_boxed_str());

    let mut query = sqlx::query(sql);
    for value in &values {
        query = query.bind(value);
    }
    query = query.bind(&id);

    query
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // 返回更新后的渠道
    let channel = get_channel_by_id(&state.pool, &id).await?;
    Ok(Json(ApiResponse::success(channel)))
}

/// 删除渠道
pub async fn delete(
    State(state): State<ChannelState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiError>)> {
    let result = sqlx::query("DELETE FROM channels WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("渠道不存在"));
    }

    Ok(Json(crate::api::response::success_empty()))
}

/// 根据 ID 获取渠道
async fn get_channel_by_id(
    pool: &SqlitePool,
    id: &str,
) -> Result<Channel, (StatusCode, Json<ApiError>)> {
    let result = sqlx::query_as::<_, (String, String, String, String, String, Option<i32>, Option<i32>, i32, i32, i32, bool, String, String)>(
        "SELECT id, name, api_keys, endpoints, model_maps, rate_limit_rpm, rate_limit_tpm, failure_threshold, blacklist_minutes, concurrency, enabled, created_at, updated_at FROM channels WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let (
        id,
        name,
        api_keys_str,
        endpoints_str,
        model_maps_str,
        rate_limit_rpm,
        rate_limit_tpm,
        failure_threshold,
        blacklist_minutes,
        concurrency,
        enabled,
        created_at,
        updated_at,
    ) = result.ok_or_else(|| ApiError::not_found("渠道不存在"))?;

    let api_keys: Vec<String> = serde_json::from_str(&api_keys_str).unwrap_or_default();
    let endpoints: Vec<EndpointConfig> = serde_json::from_str(&endpoints_str).unwrap_or_default();
    let model_maps: serde_json::Value = serde_json::from_str(&model_maps_str).unwrap_or_default();

    Ok(Channel {
        id,
        name,
        api_keys,
        endpoints,
        model_maps,
        rate_limit_rpm,
        rate_limit_tpm,
        failure_threshold,
        blacklist_minutes,
        concurrency,
        enabled,
        created_at,
        updated_at,
    })
}
