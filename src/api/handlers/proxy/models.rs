use axum::{Json, extract::State, response::IntoResponse};
use sqlx::SqlitePool;

use crate::api::middleware::ApiKeyAuth;

/// 获取可用模型列表（分组名 + 渠道直接支持的模型）
pub async fn list(_auth: ApiKeyAuth, State(pool): State<SqlitePool>) -> impl IntoResponse {
    // 获取所有启用的分组名
    let groups = sqlx::query_scalar::<_, String>("SELECT name FROM groups WHERE enabled = 1")
        .fetch_all(&pool)
        .await;

    // 获取渠道直接支持的模型
    let channel_models =
        sqlx::query_scalar::<_, String>("SELECT models FROM channels WHERE enabled = 1")
            .fetch_all(&pool)
            .await;

    let mut model_set = std::collections::HashSet::new();

    if let Ok(names) = &groups {
        for name in names {
            model_set.insert(name.clone());
        }
    }

    if let Ok(model_lists) = &channel_models {
        for models_str in model_lists {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(models_str) {
                if let Some(arr) = value.as_array() {
                    for m in arr {
                        if let Some(s) = m.as_str() {
                            model_set.insert(s.to_string());
                        }
                    }
                } else if let Some(available) = value["available_models"].as_array() {
                    for m in available {
                        if let Some(s) = m.as_str() {
                            model_set.insert(s.to_string());
                        }
                    }
                }
            }
        }
    }

    let models: Vec<serde_json::Value> = model_set
        .into_iter()
        .map(|name| {
            serde_json::json!({
                "id": name,
                "object": "model",
                "created": 0,
                "owned_by": "galaxy-router"
            })
        })
        .collect();

    Json(serde_json::json!({
        "object": "list",
        "data": models
    }))
    .into_response()
}
