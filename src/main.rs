use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

mod api;
mod auth;
mod config;
mod db;
mod protocol;
mod proxy;
mod static_assets;
mod stats;

use config::AppConfig;
use db::Database;

/// Galaxy Proxy - AI 协议互转代理网关
#[derive(Parser, Debug)]
#[command(name = "galaxy-proxy", version, about)]
struct Cli {
    /// 配置文件路径
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    /// 监听端口（覆盖配置文件）
    #[arg(short, long)]
    port: Option<u16>,

    /// 监听地址（覆盖配置文件）
    #[arg(long)]
    host: Option<String>,

    /// 日志级别
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 初始化日志
    init_logging(&cli.log_level)?;

    // 加载配置
    let config = AppConfig::load(&cli.config)?;
    info!("Configuration loaded from {:?}", cli.config);

    // 应用 CLI 覆盖
    let config = apply_cli_overrides(config, &cli);

    // 初始化数据库
    let database = Database::new(&config.database_url()).await?;
    info!("Database initialized");

    // 检查是否需要初始化 JWT 密钥
    let config = ensure_jwt_secret(config, &cli.config)?;

    // 初始化成本计算器并加载定价数据
    let cost_calculator = stats::cost::CostCalculator::new();
    if let Err(e) = cost_calculator.load_local_pricing(database.pool()).await {
        tracing::warn!("加载本地定价数据失败: {}", e);
    }
    if let Err(e) = cost_calculator.fetch_remote_pricing().await {
        tracing::warn!("获取远程定价数据失败: {}", e);
    }
    info!("Cost calculator initialized");

    // 启动后台调度器
    let lb_state = proxy::state::LoadBalancerState::new();
    let scheduler = Arc::new(proxy::scheduler::Scheduler::new(lb_state));
    scheduler.start();
    info!("Scheduler started");

    let aggregator = Arc::new(stats::aggregator::StatsAggregator::new(
        database.pool().clone(),
    ));
    aggregator.start();
    info!("Stats aggregator started");

    // 创建路由（带排队配置）
    let addr = config.server_addr();
    let jwt_secret = config.auth.jwt_secret.clone();
    let queuing = config.queuing.clone();
    let app = api::create_router(
        database.pool().clone(),
        jwt_secret,
        &queuing,
        &addr,
        config,
    ).await;

    // 启动服务器
    info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// 初始化日志
fn init_logging(log_level: &str) -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    Ok(())
}

/// 应用 CLI 参数覆盖配置
fn apply_cli_overrides(mut config: AppConfig, cli: &Cli) -> AppConfig {
    if let Some(port) = cli.port {
        config.server.port = port;
    }
    if let Some(host) = &cli.host {
        config.server.host = host.clone();
    }
    config
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

        if let Some(auth) = toml_value.get_mut("auth")
            && let Some(table) = auth.as_table_mut() {
                table.insert(
                    "jwt_secret".to_string(),
                    toml::Value::String(secret.clone()),
                );
            }

        std::fs::write(config_path, toml::to_string_pretty(&toml_value)?)?;
        config.auth.jwt_secret = secret;
    }

    Ok(config)
}
