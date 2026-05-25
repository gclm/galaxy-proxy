use axum::{extract::{Path, State}, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::api::{ApiError, ApiResponse, response::generate_id};

/// API Key
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub api_key: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建 API Key 请求
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
}

/// 更新 API Key 请求
#[derive(Debug, Deserialize)]
pub struct UpdateApiKeyRequest {
    pub name: Option<String>,
    pub enabled: Option<bool>,
}

/// API Key 状态
#[derive(Clone)]
pub struct ApiKeyState {
    pub pool: SqlitePool,
}

/// 获取 API Key 列表
pub async fn list(
    State(state): State<ApiKeyState>,
) -> Result<Json<ApiResponse<Vec<ApiKey>>>, (StatusCode, Json<ApiError>)> {
    let keys = sqlx::query_as::<_, (String, String, String, bool, String, String)>(
        "SELECT id, name, api_key, enabled, created_at, updated_at FROM api_keys ORDER BY created_at DESC"
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let result: Vec<ApiKey> = keys
        .into_iter()
        .map(|(id, name, api_key, enabled, created_at, updated_at)| {
            ApiKey {
                id,
                name,
                api_key,
                enabled,
                created_at,
                updated_at,
            }
        })
        .collect();

    Ok(Json(ApiResponse::success(result)))
}

/// 创建 API Key
pub async fn create(
    State(state): State<ApiKeyState>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<ApiResponse<ApiKey>>), (StatusCode, Json<ApiError>)> {
    // 验证输入
    if req.name.is_empty() {
        return Err(ApiError::bad_request("名称不能为空"));
    }

    let id = generate_id();
    let api_key = format!("gp-{}", generate_id());

    // 插入 API Key
    sqlx::query(
        r#"
        INSERT INTO api_keys (id, name, api_key, enabled)
        VALUES (?, ?, ?, ?)
        "#
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&api_key)
    .bind(true)
    .execute(&state.pool)
    .await
    .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let key = ApiKey {
        id,
        name: req.name,
        api_key,
        enabled: true,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    Ok((StatusCode::CREATED, Json(ApiResponse::success(key))))
}

/// 获取单个 API Key
pub async fn get(
    State(state): State<ApiKeyState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<ApiKey>>, (StatusCode, Json<ApiError>)> {
    let result = sqlx::query_as::<_, (String, String, String, bool, String, String)>(
        "SELECT id, name, api_key, enabled, created_at, updated_at FROM api_keys WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let (id, name, api_key, enabled, created_at, updated_at) =
        result.ok_or_else(|| ApiError::not_found("API Key 不存在"))?;

    Ok(Json(ApiResponse::success(ApiKey {
        id,
        name,
        api_key,
        enabled,
        created_at,
        updated_at,
    })))
}

/// 更新 API Key
pub async fn update(
    State(state): State<ApiKeyState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateApiKeyRequest>,
) -> Result<Json<ApiResponse<ApiKey>>, (StatusCode, Json<ApiError>)> {
    // 检查 API Key 是否存在
    let existing = sqlx::query_scalar::<_, String>("SELECT id FROM api_keys WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    if existing.is_none() {
        return Err(ApiError::not_found("API Key 不存在"));
    }

    // 构建更新语句
    let mut updates = Vec::new();
    let mut values: Vec<String> = Vec::new();

    if let Some(name) = &req.name {
        updates.push("name = ?");
        values.push(name.clone());
    }
    if let Some(enabled) = &req.enabled {
        updates.push("enabled = ?");
        values.push(enabled.to_string());
    }

    if updates.is_empty() {
        return Err(ApiError::bad_request("没有需要更新的字段"));
    }

    updates.push("updated_at = CURRENT_TIMESTAMP");

    // 构建动态 SQL，手动审计安全性
    let sql = format!("UPDATE api_keys SET {} WHERE id = ?", updates.join(", "));
    let sql: &'static str = Box::leak(sql.into_boxed_str());

    let mut query = sqlx::query(sql);
    for value in &values {
        query = query.bind(value);
    }
    query = query.bind(&id);

    query.execute(&state.pool)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // 返回更新后的 API Key
    get(State(state), Path(id)).await
}

/// 删除 API Key
pub async fn delete(
    State(state): State<ApiKeyState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiError>)> {
    let result = sqlx::query("DELETE FROM api_keys WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("API Key 不存在"));
    }

    Ok(Json(crate::api::response::success_empty()))
}

/// 验证 API Key（供代理 API 使用）
pub async fn validate_api_key(pool: &SqlitePool, api_key: &str) -> bool {
    let result = sqlx::query_scalar::<_, bool>(
        "SELECT enabled FROM api_keys WHERE api_key = ?"
    )
    .bind(api_key)
    .fetch_optional(pool)
    .await;

    match result {
        Ok(Some(enabled)) => enabled,
        _ => false,
    }
}
