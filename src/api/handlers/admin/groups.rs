use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::api::{response::generate_id, ApiError, ApiResponse};

/// 负载均衡模式
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GroupMode {
    RoundRobin,
    Random,
    Failover,
    Weighted,
}

impl std::fmt::Display for GroupMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GroupMode::RoundRobin => write!(f, "round_robin"),
            GroupMode::Random => write!(f, "random"),
            GroupMode::Failover => write!(f, "failover"),
            GroupMode::Weighted => write!(f, "weighted"),
        }
    }
}

/// 分组
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub match_regex: Option<String>,
    pub retry_enabled: bool,
    pub max_retries: i32,
    pub first_token_timeout_secs: i32,
    pub enabled: bool,
    pub items: Vec<GroupItem>,
    pub created_at: String,
    pub updated_at: String,
}

/// 分组项
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupItem {
    pub id: String,
    pub channel_id: String,
    pub model_name: String,
    pub priority: i32,
    pub weight: i32,
}

/// 创建分组请求
#[derive(Debug, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    pub match_regex: Option<String>,
    pub retry_enabled: Option<bool>,
    pub max_retries: Option<i32>,
    pub first_token_timeout_secs: Option<i32>,
    pub enabled: Option<bool>,
    pub items: Vec<CreateGroupItemRequest>,
}

/// 创建分组项请求
#[derive(Debug, Deserialize)]
pub struct CreateGroupItemRequest {
    pub channel_id: String,
    pub model_name: String,
    pub priority: Option<i32>,
    pub weight: Option<i32>,
}

/// 更新分组请求
#[derive(Debug, Deserialize)]
pub struct UpdateGroupRequest {
    pub name: Option<String>,
    pub match_regex: Option<String>,
    pub retry_enabled: Option<bool>,
    pub max_retries: Option<i32>,
    pub first_token_timeout_secs: Option<i32>,
    pub enabled: Option<bool>,
}

/// 添加分组项请求
#[derive(Debug, Deserialize)]
pub struct AddGroupItemRequest {
    pub channel_id: String,
    pub model_name: String,
    pub priority: Option<i32>,
    pub weight: Option<i32>,
}

/// 分组状态
#[derive(Clone)]
pub struct GroupState {
    pub pool: SqlitePool,
}

/// 获取分组列表
pub async fn list(
    State(state): State<GroupState>,
) -> Result<Json<ApiResponse<Vec<Group>>>, (StatusCode, Json<ApiError>)> {
    let groups = sqlx::query_as::<_, (String, String, Option<String>, bool, i32, i32, bool, String, String)>(
        "SELECT id, name, match_regex, retry_enabled, max_retries, first_token_timeout_secs, enabled, created_at, updated_at FROM groups ORDER BY created_at DESC"
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let mut result = Vec::new();
    for (
        id,
        name,
        match_regex,
        retry_enabled,
        max_retries,
        first_token_timeout_secs,
        enabled,
        created_at,
        updated_at,
    ) in groups
    {
        let items = get_group_items(&state.pool, &id).await?;

        result.push(Group {
            id,
            name,
            match_regex,
            retry_enabled,
            max_retries,
            first_token_timeout_secs,
            enabled,
            items,
            created_at,
            updated_at,
        });
    }

    Ok(Json(ApiResponse::success(result)))
}

/// 创建分组
pub async fn create(
    State(state): State<GroupState>,
    Json(req): Json<CreateGroupRequest>,
) -> Result<(StatusCode, Json<ApiResponse<Group>>), (StatusCode, Json<ApiError>)> {
    // 验证输入
    if req.name.is_empty() {
        return Err(ApiError::bad_request("分组名称不能为空"));
    }
    if req.items.is_empty() {
        return Err(ApiError::bad_request("至少需要一个分组项"));
    }

    let group_id = generate_id();

    // 开始事务
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // 插入分组
    sqlx::query(
        r#"
        INSERT INTO groups (id, name, match_regex, retry_enabled, max_retries, first_token_timeout_secs, enabled)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(&group_id)
    .bind(&req.name)
    .bind(&req.match_regex)
    .bind(req.retry_enabled.unwrap_or(true))
    .bind(req.max_retries.unwrap_or(3))
    .bind(req.first_token_timeout_secs.unwrap_or(30))
    .bind(req.enabled.unwrap_or(true))
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint failed") {
            ApiError::conflict("分组名称已存在")
        } else {
            ApiError::internal_error(e.to_string())
        }
    })?;

    // 插入分组项
    for item in &req.items {
        let item_id = generate_id();
        sqlx::query(
            r#"
            INSERT INTO group_items (id, group_id, channel_id, model_name, priority, weight)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&item_id)
        .bind(&group_id)
        .bind(&item.channel_id)
        .bind(&item.model_name)
        .bind(item.priority.unwrap_or(1))
        .bind(item.weight.unwrap_or(100))
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if e.to_string().contains("FOREIGN KEY constraint failed") {
                ApiError::bad_request("渠道不存在")
            } else {
                ApiError::internal_error(e.to_string())
            }
        })?;
    }

    // 提交事务
    tx.commit()
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    // 返回创建的分组
    let group = get_group_by_id(&state.pool, &group_id).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::success(group))))
}

/// 获取单个分组
pub async fn get(
    State(state): State<GroupState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Group>>, (StatusCode, Json<ApiError>)> {
    let group = get_group_by_id(&state.pool, &id).await?;
    Ok(Json(ApiResponse::success(group)))
}

/// 更新分组
pub async fn update(
    State(state): State<GroupState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateGroupRequest>,
) -> Result<Json<ApiResponse<Group>>, (StatusCode, Json<ApiError>)> {
    // 检查分组是否存在
    let existing = sqlx::query_scalar::<_, String>("SELECT id FROM groups WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    if existing.is_none() {
        return Err(ApiError::not_found("分组不存在"));
    }

    // 构建更新语句
    let mut updates = Vec::new();
    let mut values: Vec<String> = Vec::new();

    if let Some(name) = &req.name {
        updates.push("name = ?");
        values.push(name.clone());
    }
    if let Some(match_regex) = &req.match_regex {
        updates.push("match_regex = ?");
        values.push(match_regex.clone());
    }

    if updates.is_empty() {
        return Err(ApiError::bad_request("没有需要更新的字段"));
    }

    updates.push("updated_at = CURRENT_TIMESTAMP");

    // 构建动态 SQL，手动审计安全性
    let sql = format!("UPDATE groups SET {} WHERE id = ?", updates.join(", "));
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

    // 返回更新后的分组
    let group = get_group_by_id(&state.pool, &id).await?;
    Ok(Json(ApiResponse::success(group)))
}

/// 删除分组
pub async fn delete(
    State(state): State<GroupState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiError>)> {
    let result = sqlx::query("DELETE FROM groups WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("分组不存在"));
    }

    Ok(Json(crate::api::response::success_empty()))
}

/// 添加分组项
pub async fn add_item(
    State(state): State<GroupState>,
    Path(id): Path<String>,
    Json(req): Json<AddGroupItemRequest>,
) -> Result<(StatusCode, Json<ApiResponse<GroupItem>>), (StatusCode, Json<ApiError>)> {
    // 检查分组是否存在
    let existing = sqlx::query_scalar::<_, String>("SELECT id FROM groups WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    if existing.is_none() {
        return Err(ApiError::not_found("分组不存在"));
    }

    let item_id = generate_id();

    sqlx::query(
        r#"
        INSERT INTO group_items (id, group_id, channel_id, model_name, priority, weight)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&item_id)
    .bind(&id)
    .bind(&req.channel_id)
    .bind(&req.model_name)
    .bind(req.priority.unwrap_or(1))
    .bind(req.weight.unwrap_or(100))
    .execute(&state.pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("FOREIGN KEY constraint failed") {
            ApiError::bad_request("渠道不存在")
        } else {
            ApiError::internal_error(e.to_string())
        }
    })?;

    let item = GroupItem {
        id: item_id,
        channel_id: req.channel_id,
        model_name: req.model_name,
        priority: req.priority.unwrap_or(1),
        weight: req.weight.unwrap_or(100),
    };

    Ok((StatusCode::CREATED, Json(ApiResponse::success(item))))
}

/// 删除分组项
pub async fn delete_item(
    State(state): State<GroupState>,
    Path((group_id, item_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiError>)> {
    let result = sqlx::query("DELETE FROM group_items WHERE id = ? AND group_id = ?")
        .bind(&item_id)
        .bind(&group_id)
        .execute(&state.pool)
        .await
        .map_err(|e| ApiError::internal_error(e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("分组项不存在"));
    }

    Ok(Json(crate::api::response::success_empty()))
}

/// 根据 ID 获取分组
async fn get_group_by_id(
    pool: &SqlitePool,
    id: &str,
) -> Result<Group, (StatusCode, Json<ApiError>)> {
    let result = sqlx::query_as::<_, (String, String, Option<String>, bool, i32, i32, bool, String, String)>(
        "SELECT id, name, match_regex, retry_enabled, max_retries, first_token_timeout_secs, enabled, created_at, updated_at FROM groups WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::internal_error(e.to_string()))?;

    let (
        id,
        name,
        match_regex,
        retry_enabled,
        max_retries,
        first_token_timeout_secs,
        enabled,
        created_at,
        updated_at,
    ) = result.ok_or_else(|| ApiError::not_found("分组不存在"))?;

    let items = get_group_items(pool, &id).await?;

    Ok(Group {
        id,
        name,
        match_regex,
        retry_enabled,
        max_retries,
        first_token_timeout_secs,
        enabled,
        items,
        created_at,
        updated_at,
    })
}

/// 获取分组的所有分组项
async fn get_group_items(
    pool: &SqlitePool,
    group_id: &str,
) -> Result<Vec<GroupItem>, (StatusCode, Json<ApiError>)> {
    let items = sqlx::query_as::<_, (String, String, String, i32, i32)>(
        "SELECT id, channel_id, model_name, priority, weight FROM group_items WHERE group_id = ? ORDER BY priority DESC, weight DESC"
    )
    .bind(group_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::internal_error(e.to_string()))?;

    Ok(items
        .into_iter()
        .map(|(id, channel_id, model_name, priority, weight)| GroupItem {
            id,
            channel_id,
            model_name,
            priority,
            weight,
        })
        .collect())
}
