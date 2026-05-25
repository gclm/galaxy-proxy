use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 应用配置（从 TOML 文件加载）
#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub logging: LoggingConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub file: bool,
    pub file_path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub token_expiry_hours: u64,
}

/// 运行时配置（从数据库加载）
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub scheduler: SchedulerConfig,
    pub sticky_session: StickySessionConfig,
    pub stats: StatsConfig,
}

#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub top_k: usize,
    pub score_weights: ScoreWeights,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreWeights {
    pub priority: f64,
    pub load: f64,
    pub queue: f64,
    pub error_rate: f64,
    pub ttft: f64,
}

impl Default for ScoreWeights {
    fn default() -> Self {
        Self {
            priority: 1.0,
            load: 1.0,
            queue: 0.7,
            error_rate: 0.8,
            ttft: 0.5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StickySessionConfig {
    pub enabled: bool,
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct StatsConfig {
    pub log_detail_mode: String,
    pub cost: CostConfig,
}

#[derive(Debug, Clone)]
pub struct CostConfig {
    pub source: String,
    pub refresh_interval_hours: u64,
}

impl AppConfig {
    /// 从配置文件加载配置
    pub fn load(path: &Path) -> Result<Self> {
        let config = config::Config::builder()
            .add_source(config::File::from(path))
            .add_source(config::Environment::with_prefix("GALAXY_PROXY"))
            .build()?;

        let app_config: AppConfig = config.try_deserialize()?;
        Ok(app_config)
    }

    /// 获取服务器地址
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    /// 获取数据库 URL
    pub fn database_url(&self) -> String {
        format!("sqlite:{}?mode=rwc", self.database.path)
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            scheduler: SchedulerConfig {
                top_k: 7,
                score_weights: ScoreWeights {
                    priority: 1.0,
                    load: 1.0,
                    queue: 0.7,
                    error_rate: 0.8,
                    ttft: 0.5,
                },
            },
            sticky_session: StickySessionConfig {
                enabled: true,
                ttl_seconds: 3600,
            },
            stats: StatsConfig {
                log_detail_mode: "failures_only".to_string(),
                cost: CostConfig {
                    source: "models.dev".to_string(),
                    refresh_interval_hours: 24,
                },
            },
        }
    }
}
