use anyhow::Result;
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

mod api;
mod auth;
mod config;
mod db;
mod protocol;
mod proxy;

use auth::JwtService;
use config::AppConfig;
use db::Database;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    init_logging()?;

    // 加载配置
    let config_path = get_config_path();
    let config = AppConfig::load(&config_path)?;
    info!("Configuration loaded from {:?}", config_path);

    // 初始化数据库
    let database = Database::new(&config.database_url()).await?;
    info!("Database initialized");

    // 检查是否需要初始化 JWT 密钥
    let config = ensure_jwt_secret(config, &config_path)?;

    // 创建路由
    let app = api::create_router(database.pool().clone(), config.auth.jwt_secret.clone());

    // 启动服务器
    let addr = config.server_addr();
    info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// 初始化日志
fn init_logging() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    Ok(())
}

/// 获取配置文件路径
fn get_config_path() -> PathBuf {
    // 优先使用环境变量
    if let Ok(path) = std::env::var("GALAXY_PROXY_CONFIG") {
        return PathBuf::from(path);
    }

    // 默认路径
    PathBuf::from("config.toml")
}

/// 确保 JWT 密钥存在
fn ensure_jwt_secret(mut config: AppConfig, config_path: &PathBuf) -> Result<AppConfig> {
    if config.auth.jwt_secret.is_empty() {
        use rand::Rng;
        let secret: String = rand::rng()
            .sample_iter(&rand::distr::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        info!("Generated new JWT secret");

        // 读取并更新配置文件
        let content = std::fs::read_to_string(config_path)?;
        let mut toml_value: toml::Value = toml::from_str(&content)?;

        if let Some(auth) = toml_value.get_mut("auth") {
            if let Some(table) = auth.as_table_mut() {
                table.insert(
                    "jwt_secret".to_string(),
                    toml::Value::String(secret.clone()),
                );
            }
        }

        std::fs::write(config_path, toml::to_string_pretty(&toml_value)?)?;
        config.auth.jwt_secret = secret;
    }

    Ok(config)
}
