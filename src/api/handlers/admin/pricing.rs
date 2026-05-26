use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::api::{ApiError, ApiResponse};
use crate::stats::cost::CostCalculator;

/// 定价 API 状态
#[derive(Clone)]
pub struct PricingState {
    pub cost_calculator: CostCalculator,
}

/// 获取所有定价
pub async fn list(
    State(state): State<PricingState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiError>)> {
    let pricing = state.cost_calculator.get_all_pricing().await;
    Ok(Json(ApiResponse::success(serde_json::json!(pricing))))
}

/// 获取指定模型定价
pub async fn get(
    State(state): State<PricingState>,
    Path(model): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiError>)> {
    match state.cost_calculator.get_pricing(&model).await {
        Some(pricing) => Ok(Json(ApiResponse::success(serde_json::json!(pricing)))),
        None => Err(ApiError::not_found(format!("模型 {} 的定价不存在", model))),
    }
}

/// 更新定价请求
#[derive(Debug, Deserialize)]
pub struct UpdatePricingRequest {
    pub model: String,
    pub input_per_million: f64,
    pub output_per_million: f64,
    pub cache_read_per_million: Option<f64>,
    pub cache_creation_per_million: Option<f64>,
}

/// 更新定价
pub async fn update(
    State(state): State<PricingState>,
    Json(req): Json<UpdatePricingRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiError>)> {
    let pricing = crate::stats::cost::ModelPricing {
        model: req.model,
        input_per_million: req.input_per_million,
        output_per_million: req.output_per_million,
        cache_read_per_million: req.cache_read_per_million,
        cache_creation_per_million: req.cache_creation_per_million,
    };

    // 保存到本地定价缓存
    state
        .cost_calculator
        .set_local_pricing(pricing.clone())
        .await;

    Ok(Json(ApiResponse::success(serde_json::json!(pricing))))
}
