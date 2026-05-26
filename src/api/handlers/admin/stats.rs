use axum::{extract::{Query, State}, http::StatusCode, Json};
use serde::Deserialize;

use crate::api::{ApiError, ApiResponse};
use crate::stats::StatsState;

/// 查询参数
#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub days: Option<i32>,
}

/// 统计 API 状态
#[derive(Clone)]
pub struct StatsApiState {
    pub stats: StatsState,
}

/// 获取统计概览
pub async fn overview(
    State(state): State<StatsApiState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiError>)> {
    let overview = state.stats.get_overview().await
        .map_err(|e: sqlx::Error| ApiError::internal_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(serde_json::json!(overview))))
}

/// 获取按模型统计
pub async fn models(
    State(state): State<StatsApiState>,
    Query(query): Query<StatsQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiError>)> {
    let days = query.days.unwrap_or(30);
    let stats = state.stats.get_model_stats(days).await
        .map_err(|e: sqlx::Error| ApiError::internal_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(serde_json::json!(stats))))
}

/// 获取按渠道统计
pub async fn channels(
    State(state): State<StatsApiState>,
    Query(query): Query<StatsQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiError>)> {
    let days = query.days.unwrap_or(30);
    let stats = state.stats.get_channel_stats(days).await
        .map_err(|e: sqlx::Error| ApiError::internal_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(serde_json::json!(stats))))
}

/// 获取按天统计
pub async fn daily(
    State(state): State<StatsApiState>,
    Query(query): Query<StatsQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiError>)> {
    let days = query.days.unwrap_or(30);
    let stats = state.stats.get_daily_stats(days).await
        .map_err(|e: sqlx::Error| ApiError::internal_error(e.to_string()))?;

    Ok(Json(ApiResponse::success(serde_json::json!(stats))))
}
