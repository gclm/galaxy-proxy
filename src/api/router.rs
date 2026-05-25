use axum::{routing::get, Json, Router};
use serde_json::{json, Value};
use tower_http::trace::TraceLayer;

/// 创建应用路由
pub fn create_router() -> Router {
    Router::new()
        // 健康检查
        .route("/health", get(health_check))
        // 代理 API 路由占位
        .nest("/v1", proxy_routes())
        // 管理 API 路由占位
        .nest("/api/v1/admin", admin_routes())
        // 中间件
        .layer(TraceLayer::new_for_http())
}

/// 健康检查端点
async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// 代理 API 路由
fn proxy_routes() -> Router {
    Router::new()
        // TODO: 添加代理路由
        // .route("/chat/completions", post(chat_completions))
        // .route("/responses", post(responses))
        // .route("/messages", post(messages))
        // .route("/embeddings", post(embeddings))
        // .route("/images/generations", post(images))
        // .route("/models", get(models))
}

/// 管理 API 路由
fn admin_routes() -> Router {
    Router::new()
        // TODO: 添加管理路由
        // .nest("/auth", auth_routes())
        // .nest("/channels", channel_routes())
        // .nest("/groups", group_routes())
        // .nest("/api-keys", api_key_routes())
        // .nest("/stats", stats_routes())
}
