use axum::{
    middleware,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tower_http::trace::TraceLayer;

use crate::api::handlers::admin::api_keys::{self, ApiKeyState};
use crate::api::handlers::admin::auth::{self, AuthState};
use crate::api::handlers::admin::channels::{self, ChannelState};
use crate::api::handlers::admin::groups::{self, GroupState};
use crate::api::handlers::admin::pricing::{self, PricingState};
use crate::api::handlers::admin::stats::{self, StatsApiState};
use crate::api::handlers::proxy::{chat, embeddings, images, messages, models, responses};
use crate::config::QueuingConfig;
use crate::proxy::ProxyState;
use crate::stats::StatsState;

/// 创建应用路由
pub fn create_router(pool: SqlitePool, jwt_secret: String, queuing: &QueuingConfig) -> Router {
    let auth_state = AuthState {
        pool: pool.clone(),
        jwt_service: crate::auth::JwtService::new(&jwt_secret, 24),
    };

    let channel_state = ChannelState { pool: pool.clone() };

    let group_state = GroupState { pool: pool.clone() };

    let api_key_state = ApiKeyState { pool: pool.clone() };

    let stats_state = StatsApiState {
        stats: StatsState::new(pool.clone()),
    };

    let pricing_state = PricingState {
        cost_calculator: crate::stats::cost::CostCalculator::new(),
    };

    let proxy_state = if queuing.enabled {
        ProxyState::new(pool.clone()).with_queue(queuing.max_queue_size, queuing.queue_timeout_secs)
    } else {
        ProxyState::new(pool.clone())
    };

    Router::new()
        // 健康检查
        .route("/health", get(health_check))
        // 代理 API 路由
        .nest("/v1", proxy_routes(proxy_state, pool.clone()))
        // 管理 API 路由 - 认证
        .nest("/api/v1/admin/auth", auth_routes(auth_state))
        // 管理 API 路由 - 渠道
        .nest("/api/v1/admin/channels", channel_routes(channel_state))
        // 管理 API 路由 - 分组
        .nest("/api/v1/admin/groups", group_routes(group_state))
        // 管理 API 路由 - API Key
        .nest("/api/v1/admin/api-keys", api_key_routes(api_key_state))
        // 管理 API 路由 - 统计
        .nest("/api/v1/admin/stats", stats_routes(stats_state))
        // 管理 API 路由 - 定价
        .nest("/api/v1/admin/pricing", pricing_routes(pricing_state))
        // 注入 JWT secret 到 extensions
        .layer(middleware::from_fn(
            move |mut req: axum::http::Request<axum::body::Body>, next: middleware::Next| {
                let secret = jwt_secret.clone();
                async move {
                    req.extensions_mut().insert(secret);
                    next.run(req).await
                }
            },
        ))
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
fn proxy_routes(proxy_state: ProxyState, pool: SqlitePool) -> Router {
    use crate::api::middleware::ApiKeyCache;

    let pool_clone = pool.clone();
    let api_key_cache = ApiKeyCache::new();

    Router::new()
        .route("/chat/completions", post(chat::proxy))
        .route("/responses", post(responses::proxy))
        .route("/messages", post(messages::proxy))
        .route("/embeddings", post(embeddings::proxy))
        .route("/images/generations", post(images::proxy))
        .with_state(proxy_state)
        .route("/models", get(models::list))
        .with_state(pool)
        .layer(middleware::from_fn(move |mut req: axum::http::Request<axum::body::Body>, next: middleware::Next| {
            let pool = pool_clone.clone();
            let cache = api_key_cache.clone();
            async move {
                req.extensions_mut().insert(pool);
                req.extensions_mut().insert(cache);
                next.run(req).await
            }
        }))
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
        .route(
            "/{id}",
            get(channels::get)
                .put(channels::update)
                .delete(channels::delete),
        )
        .with_state(channel_state)
}

/// 分组路由
fn group_routes(group_state: GroupState) -> Router {
    Router::new()
        .route("/", get(groups::list).post(groups::create))
        .route(
            "/{id}",
            get(groups::get).put(groups::update).delete(groups::delete),
        )
        .route("/{id}/items", post(groups::add_item))
        .route("/{id}/items/{item_id}", delete(groups::delete_item))
        .with_state(group_state)
}

/// API Key 路由
fn api_key_routes(api_key_state: ApiKeyState) -> Router {
    Router::new()
        .route("/", get(api_keys::list).post(api_keys::create))
        .route(
            "/{id}",
            get(api_keys::get)
                .put(api_keys::update)
                .delete(api_keys::delete),
        )
        .with_state(api_key_state)
}

/// 统计路由
fn stats_routes(stats_state: StatsApiState) -> Router {
    Router::new()
        .route("/overview", get(stats::overview))
        .route("/models", get(stats::models))
        .route("/channels", get(stats::channels))
        .route("/daily", get(stats::daily))
        .with_state(stats_state)
}

/// 定价路由
fn pricing_routes(pricing_state: PricingState) -> Router {
    Router::new()
        .route("/", get(pricing::list).put(pricing::update))
        .route("/{model}", get(pricing::get))
        .with_state(pricing_state)
}
