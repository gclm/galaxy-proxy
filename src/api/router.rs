use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware,
    response::Response,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tower_http::trace::TraceLayer;

use crate::api::handlers::admin::api_keys::{self, ApiKeyState};
use crate::api::handlers::admin::auth::{self, AuthState};
use crate::api::handlers::admin::backup::{self, BackupState};
use crate::api::handlers::admin::channels::{self, ChannelState};
use crate::api::handlers::admin::fetch_models::{self, FetchModelsState};
use crate::api::handlers::admin::groups::{self, GroupState};
use crate::api::handlers::admin::model_info::{self, ModelInfoState};
use crate::api::handlers::admin::settings::{self, SettingsState};
use crate::api::handlers::admin::stats::{self, StatsApiState};
use crate::api::handlers::admin::system_info::{self, SystemInfoState};
use crate::api::handlers::admin::test_model::{self, TestModelState};
use crate::api::handlers::proxy::{chat, embeddings, images, messages, models, responses};
use crate::config::{AppConfig, QueuingConfig};
use crate::proxy::ProxyState;
use crate::static_assets;
use crate::stats::StatsState;

/// 创建应用路由
pub async fn create_router(pool: SqlitePool, jwt_secret: String, queuing: &QueuingConfig, _server_addr: &str, config: AppConfig, model_registry: crate::stats::model::ModelRegistry) -> Router {
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

    let model_info_state = ModelInfoState {
        model_registry: model_registry.clone(),
    };

    let fetch_models_state = FetchModelsState {
        http_client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client"),
    };

    let test_model_state = TestModelState {
        http_client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client"),
        pool: pool.clone(),
    };

    let system_info_state = SystemInfoState {
        pool: pool.clone(),
        start_time: std::sync::Arc::new(std::time::Instant::now()),
    };

    let settings_state = SettingsState {
        pool: pool.clone(),
        config: std::sync::Arc::new(config),
    };

    let backup_state = BackupState { pool: pool.clone() };

    let proxy_state = if queuing.enabled {
        ProxyState::new(pool.clone(), model_registry.clone()).await.with_queue(queuing.max_queue_size, queuing.queue_timeout_secs)
    } else {
        ProxyState::new(pool.clone(), model_registry.clone()).await
    };

    Router::new()
        // 健康检查（返回初始化状态）
        .route("/api/v1/health", get(health_check))
        // 代理 API 路由
        .nest("/v1", proxy_routes(proxy_state, pool.clone()))
        // 初始化接口（无需认证）
        .nest("/api/v1/init", init_routes(auth_state.clone()))
        // 管理 API 路由 - 认证
        .nest("/api/v1/admin/auth", auth_routes(auth_state))
        // 管理 API 路由 - 渠道
        .nest("/api/v1/admin/channels", channel_routes(channel_state))
        // 管理 API 路由 - 获取模型列表
        .nest("/api/v1/admin", fetch_models_routes(fetch_models_state))
        // 管理 API 路由 - 测试模型
        .nest("/api/v1/admin", test_model_routes(test_model_state))
        // 管理 API 路由 - 分组
        .nest("/api/v1/admin/groups", group_routes(group_state))
        // 管理 API 路由 - API Key
        .nest("/api/v1/admin/api-keys", api_key_routes(api_key_state))
        // 管理 API 路由 - 统计
        .nest("/api/v1/admin/stats", stats_routes(stats_state))
        // 管理 API 路由 - 定价
        .nest("/api/v1/admin/models/info", model_info_routes(model_info_state))
        // 管理 API 路由 - 系统信息
        .nest("/api/v1/admin", system_info_routes(system_info_state))
        // 管理 API 路由 - 设置
        .nest("/api/v1/admin/settings", settings_routes(settings_state))
        // 管理 API 路由 - 备份
        .nest("/api/v1/admin/backup", backup_routes(backup_state))
        // 静态文件服务（SPA fallback）
        .fallback(static_assets::serve)
        // 注入 pool 和 JWT secret 到 extensions
        .layer(middleware::from_fn(
            move |mut req: Request<Body>, next: middleware::Next| {
                let secret = jwt_secret.clone();
                let pool = pool.clone();
                async move {
                    req.extensions_mut().insert(secret);
                    req.extensions_mut().insert(pool);
                    next.run(req).await
                }
            },
        ))
        // 中间件
        .layer(TraceLayer::new_for_http())
}

/// 健康检查端点（返回初始化状态）
async fn health_check(
    axum::Extension(pool): axum::Extension<SqlitePool>,
) -> Json<Value> {
    let needs_setup = sqlx::query_scalar::<_, i32>("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
        .map(|count| count == 0)
        .unwrap_or(true);

    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "needs_setup": needs_setup
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
        .layer(middleware::from_fn(move |mut req: Request<Body>, next: middleware::Next| {
            let pool = pool_clone.clone();
            let cache = api_key_cache.clone();
            async move {
                req.extensions_mut().insert(pool);
                req.extensions_mut().insert(cache);
                next.run(req).await
            }
        }))
}

/// 初始化路由（无需认证）
fn init_routes(auth_state: AuthState) -> Router {
    Router::new()
        .route("/", post(auth::init))
        .with_state(auth_state)
}

/// 认证路由
fn auth_routes(auth_state: AuthState) -> Router {
    Router::new()
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

/// 获取模型列表路由
fn fetch_models_routes(state: FetchModelsState) -> Router {
    Router::new()
        .route("/fetch-models", post(fetch_models::fetch_models))
        .with_state(state)
}

/// 测试模型路由
fn test_model_routes(state: TestModelState) -> Router {
    Router::new()
        .route("/test-model", post(test_model::test_model))
        .with_state(state)
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
        .route("/logs", get(stats::logs))
        .route("/logs/{id}", get(stats::log_detail))
        .with_state(stats_state)
}

/// 定价路由
fn model_info_routes(model_info_state: ModelInfoState) -> Router {
    Router::new()
        .route("/", get(model_info::list).put(model_info::update))
        .route("/{model}", get(model_info::get))
        .with_state(model_info_state)
}

/// 系统信息路由
fn system_info_routes(system_info_state: SystemInfoState) -> Router {
    Router::new()
        .route("/system-info", get(system_info::get))
        .with_state(system_info_state)
}

/// 设置路由
fn settings_routes(settings_state: SettingsState) -> Router {
    Router::new()
        .route("/", get(settings::list))
        .route("/infra", get(settings::infra))
        .route("/{key}", put(settings::update))
        .with_state(settings_state)
}

/// 备份路由
fn backup_routes(backup_state: BackupState) -> Router {
    Router::new()
        .route("/export", get(backup::export))
        .route("/import", post(backup::import))
        .route("/reset", post(backup::reset))
        .with_state(backup_state)
}
