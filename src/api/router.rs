use axum::{routing::{get, post, put}, Json, Router, middleware};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tower_http::trace::TraceLayer;

use crate::api::handlers::admin::auth::{self, AuthState};

/// 创建应用路由
pub fn create_router(pool: SqlitePool, jwt_secret: String) -> Router {
    let auth_state = AuthState {
        pool: pool.clone(),
        jwt_service: crate::auth::JwtService::new(&jwt_secret, 24),
    };

    Router::new()
        // 健康检查
        .route("/health", get(health_check))
        // 代理 API 路由占位
        .nest("/v1", proxy_routes())
        // 管理 API 路由
        .nest("/api/v1/admin", admin_routes(auth_state))
        // 注入 JWT secret 到 extensions
        .layer(middleware::from_fn(move |mut req: axum::http::Request<axum::body::Body>, next: middleware::Next| {
            let secret = jwt_secret.clone();
            async move {
                req.extensions_mut().insert(secret);
                next.run(req).await
            }
        }))
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
}

/// 管理 API 路由
fn admin_routes(auth_state: AuthState) -> Router {
    Router::new()
        // 认证路由（无需认证）
        .route("/auth/setup", post(auth::setup))
        .route("/auth/login", post(auth::login))
        // 需要认证的路由
        .route("/auth/me", get(auth::me))
        .route("/auth/password", put(auth::change_password))
        .with_state(auth_state)
}
