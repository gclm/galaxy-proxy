use axum::{extract::{Path, State}, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// 渠道类型
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    OpenAiChat,
    OpenAiResponse,
    Anthropic,
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelType::OpenAiChat => write!(f, "openai_chat"),
            ChannelType::OpenAiResponse => write!(f, "openai_response"),
            ChannelType::Anthropic => write!(f, "anthropic"),
        }
    }
}

/// 渠道
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Channel {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub channel_type: ChannelType,
    pub base_url: String,
    pub api_keys: Vec<String>,
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
    #[serde(rename = "type")]
    pub channel_type: ChannelType,
    pub base_url: String,
    pub api_keys: Vec<String>,
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
    #[serde(rename = "type")]
    pub channel_type: Option<ChannelType>,
    pub base_url: Option<String>,
    pub api_keys: Option<Vec<String>>,
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
) -> Result<Json<Vec<Channel>>, (StatusCode, String)> {
    let channels = sqlx::query_as::<_, (i64, String, String, String, String, String, Option<i32>, Option<i32>, i32, i32, i32, bool, String, String)>(
        "SELECT id, name, type, base_url, api_keys, model_maps, rate_limit_rpm, rate_limit_tpm, failure_threshold, blacklist_minutes, concurrency, enabled, created_at, updated_at FROM channels ORDER BY id"
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let result: Vec<Channel> = channels
        .into_iter()
        .map(|(id, name, type_str, base_url, api_keys_str, model_maps_str, rate_limit_rpm, rate_limit_tpm, failure_threshold, blacklist_minutes, concurrency, enabled, created_at, updated_at)| {
            let channel_type = match type_str.as_str() {
                "openai_chat" => ChannelType::OpenAiChat,
                "openai_response" => ChannelType::OpenAiResponse,
                "anthropic" => ChannelType::Anthropic,
                _ => ChannelType::OpenAiChat,
            };
            let api_keys: Vec<String> = serde_json::from_str(&api_keys_str).unwrap_or_default();
            let model_maps: serde_json::Value = serde_json::from_str(&model_maps_str).unwrap_or_default();

            Channel {
                id,
                name,
                channel_type,
                base_url,
                api_keys,
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
        })
        .collect();

    Ok(Json(result))
}

/// 创建渠道
pub async fn create(
    State(state): State<ChannelState>,
    Json(req): Json<CreateChannelRequest>,
) -> Result<(StatusCode, Json<Channel>), (StatusCode, String)> {
    // 验证输入
    if req.name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "渠道名称不能为空".to_string()));
    }
    if req.api_keys.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "至少需要一个 API Key".to_string()));
    }

    let api_keys_json = serde_json::to_string(&req.api_keys)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let model_maps_json = serde_json::to_string(&req.model_maps.unwrap_or_default())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 插入渠道
    let id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO channels (name, type, base_url, api_keys, model_maps, rate_limit_rpm, rate_limit_tpm, failure_threshold, blacklist_minutes, concurrency, enabled)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING id
        "#
    )
    .bind(&req.name)
    .bind(req.channel_type.to_string())
    .bind(&req.base_url)
    .bind(&api_keys_json)
    .bind(&model_maps_json)
    .bind(req.rate_limit_rpm)
    .bind(req.rate_limit_tpm)
    .bind(req.failure_threshold.unwrap_or(3))
    .bind(req.blacklist_minutes.unwrap_or(10))
    .bind(req.concurrency.unwrap_or(10))
    .bind(req.enabled.unwrap_or(true))
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint failed") {
            (StatusCode::CONFLICT, "渠道名称已存在".to_string())
        } else {
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
    })?;

    // 返回创建的渠道
    get(State(state), Path(id)).await
        .map(|Json(channel)| (StatusCode::CREATED, Json(channel)))
}

/// 获取单个渠道
pub async fn get(
    State(state): State<ChannelState>,
    Path(id): Path<i64>,
) -> Result<Json<Channel>, (StatusCode, String)> {
    let result = sqlx::query_as::<_, (i64, String, String, String, String, String, Option<i32>, Option<i32>, i32, i32, i32, bool, String, String)>(
        "SELECT id, name, type, base_url, api_keys, model_maps, rate_limit_rpm, rate_limit_tpm, failure_threshold, blacklist_minutes, concurrency, enabled, created_at, updated_at FROM channels WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let (id, name, type_str, base_url, api_keys_str, model_maps_str, rate_limit_rpm, rate_limit_tpm, failure_threshold, blacklist_minutes, concurrency, enabled, created_at, updated_at) =
        result.ok_or_else(|| (StatusCode::NOT_FOUND, "渠道不存在".to_string()))?;

    let channel_type = match type_str.as_str() {
        "openai_chat" => ChannelType::OpenAiChat,
        "openai_response" => ChannelType::OpenAiResponse,
        "anthropic" => ChannelType::Anthropic,
        _ => ChannelType::OpenAiChat,
    };

    let channel = Channel {
        id,
        name,
        channel_type,
        base_url,
        api_keys: serde_json::from_str(&api_keys_str).unwrap_or_default(),
        model_maps: serde_json::from_str(&model_maps_str).unwrap_or_default(),
        rate_limit_rpm,
        rate_limit_tpm,
        failure_threshold,
        blacklist_minutes,
        concurrency,
        enabled,
        created_at,
        updated_at,
    };

    Ok(Json(channel))
}

/// 更新渠道
pub async fn update(
    State(state): State<ChannelState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateChannelRequest>,
) -> Result<Json<Channel>, (StatusCode, String)> {
    // 检查渠道是否存在
    let existing = sqlx::query_scalar::<_, i64>("SELECT id FROM channels WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if existing.is_none() {
        return Err((StatusCode::NOT_FOUND, "渠道不存在".to_string()));
    }

    // 构建更新语句
    let mut updates = Vec::new();
    let mut values: Vec<String> = Vec::new();

    if let Some(name) = &req.name {
        updates.push("name = ?");
        values.push(name.clone());
    }
    if let Some(channel_type) = &req.channel_type {
        updates.push("type = ?");
        values.push(channel_type.to_string());
    }
    if let Some(base_url) = &req.base_url {
        updates.push("base_url = ?");
        values.push(base_url.clone());
    }
    if let Some(api_keys) = &req.api_keys {
        updates.push("api_keys = ?");
        values.push(serde_json::to_string(api_keys).unwrap_or_default());
    }
    if let Some(model_maps) = &req.model_maps {
        updates.push("model_maps = ?");
        values.push(serde_json::to_string(model_maps).unwrap_or_default());
    }

    if updates.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "没有需要更新的字段".to_string()));
    }

    updates.push("updated_at = CURRENT_TIMESTAMP");

    let sql = format!("UPDATE channels SET {} WHERE id = ?", updates.join(", "));

    let mut query = sqlx::query(&sql);
    for value in &values {
        query = query.bind(value);
    }
    query = query.bind(id);

    query.execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 返回更新后的渠道
    get(State(state), Path(id)).await
}

/// 删除渠道
pub async fn delete(
    State(state): State<ChannelState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, String)> {
    let result = sqlx::query("DELETE FROM channels WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "渠道不存在".to_string()));
    }

    Ok(StatusCode::NO_CONTENT)
}
