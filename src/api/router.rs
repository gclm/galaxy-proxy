use axum::{routing::{get, post, put, delete}, Json, Router, middleware};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tower_http::trace::TraceLayer;

use crate::api::handlers::admin::auth::{self, AuthState};
use crate::api::handlers::admin::channels::{self, ChannelState};
use crate::api::handlers::admin::groups::{self, GroupState};

/// 创建应用路由
pub fn create_router(pool: SqlitePool, jwt_secret: String) -> Router {
    let auth_state = AuthState {
        pool: pool.clone(),
        jwt_service: crate::auth::JwtService::new(&jwt_secret, 24),
    };

    let channel_state = ChannelState {
        pool: pool.clone(),
    };

    let group_state = GroupState {
        pool: pool.clone(),
    };

    Router::new()
        // 健康检查
        .route("/health", get(health_check))
        // 代理 API 路由占位
        .nest("/v1", proxy_routes())
        // 管理 API 路由 - 认证
        .nest("/api/v1/admin/auth", auth_routes(auth_state))
        // 管理 API 路由 - 渠道
        .nest("/api/v1/admin/channels", channel_routes(channel_state))
        // 管理 API 路由 - 分组
        .nest("/api/v1/admin/groups", group_routes(group_state))
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

/// 认证路由
fn auth_routes(auth_state: AuthState) -> Router {
    Router::new()
        .route("/setup", post(auth::setup))
        .route("/login", post(auth::login))
        .route("/me", get(auth::me))
        .route("/password", put(auth::change_password))
        .with_state(auth_state)
}

/// 渠道路由
fn channel_routes(channel_state: ChannelState) -> Router {
    Router::new()
        .route("/", get(channels::list).post(channels::create))
        .route("/{id}", get(channels::get).put(channels::update).delete(channels::delete))
        .with_state(channel_state)
}

/// 分组路由
fn group_routes(group_state: GroupState) -> Router {
    Router::new()
        .route("/", get(groups::list).post(groups::create))
        .route("/{id}", get(groups::get).put(groups::update).delete(groups::delete))
        .route("/{id}/items", post(groups::add_item))
        .route("/{id}/items/{item_id}", delete(groups::delete_item))
        .with_state(group_state)
}
