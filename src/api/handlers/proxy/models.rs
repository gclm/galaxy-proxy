use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use sqlx::SqlitePool;

/// 获取可用模型列表
pub async fn list(
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    // 获取所有启用的分组
    let groups = sqlx::query_scalar::<_, String>(
        "SELECT name FROM groups WHERE enabled = 1"
    )
    .fetch_all(&pool)
    .await;

    match groups {
        Ok(names) => {
            let models: Vec<serde_json::Value> = names
                .into_iter()
                .map(|name| {
                    serde_json::json!({
                        "id": name,
                        "object": "model",
                        "created": 0,
                        "owned_by": "galaxy-proxy"
                    })
                })
                .collect();

            Json(serde_json::json!({
                "object": "list",
                "data": models
            })).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": { "message": e.to_string(), "type": "server_error" }
            }))).into_response()
        }
    }
}
