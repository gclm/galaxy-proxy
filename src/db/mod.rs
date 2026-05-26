use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::Path;
use tracing::info;

use crate::config::{
    CostConfig, RuntimeConfig, SchedulerConfig, ScoreWeights, StatsConfig, StickySessionConfig,
};

/// 设置项（数据库行）
#[derive(Debug, sqlx::FromRow)]
struct SettingRow {
    key: String,
    category: String,
    value: String,
    description: Option<String>,
}

/// 设置项（API 返回）
#[derive(Debug, Serialize, Deserialize)]
pub struct SettingItem {
    pub key: String,
    pub category: String,
    pub value: String,
    pub description: Option<String>,
}

/// 数据库连接池
#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// 创建数据库连接
    pub async fn new(database_url: &str) -> Result<Self> {
        // 确保数据目录存在
        if let Some(path) = database_url.strip_prefix("sqlite:") {
            if let Some(parent) = Path::new(path).parent() {
                std::fs::create_dir_all(parent)?;
            }
        }

        // 对于文件数据库，使用 sqlite:{path} 格式
        let connect_url = if database_url.starts_with("sqlite:") && !database_url.contains("?") {
            database_url
        } else {
            database_url
        };

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(connect_url)
            .await?;

        let db = Self { pool };
        db.run_migrations().await?;
        Ok(db)
    }

    /// 运行数据库迁移
    async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS _migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // 获取已应用的迁移
        let applied: Vec<i32> =
            sqlx::query_scalar("SELECT version FROM _migrations ORDER BY version")
                .fetch_all(&self.pool)
                .await?;

        // 应用迁移
        let migrations = get_migrations();
        for migration in migrations {
            if !applied.contains(&migration.version) {
                info!(
                    "Applying migration {}: {}",
                    migration.version, migration.name
                );
                sqlx::query(migration.sql).execute(&self.pool).await?;
                sqlx::query("INSERT INTO _migrations (version, name) VALUES (?, ?)")
                    .bind(migration.version)
                    .bind(migration.name)
                    .execute(&self.pool)
                    .await?;
            }
        }

        info!("Database migrations completed");
        Ok(())
    }

    /// 获取连接池引用
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// 从数据库加载运行时配置
    pub async fn load_runtime_config(&self) -> Result<RuntimeConfig> {
        let settings: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM settings")
            .fetch_all(&self.pool)
            .await?;

        let settings_map: std::collections::HashMap<String, String> =
            settings.into_iter().collect();

        Ok(self.build_runtime_config(&settings_map))
    }

    /// 按分类查询设置
    pub async fn get_settings_by_category(&self, category: &str) -> Result<Vec<SettingItem>> {
        let settings = sqlx::query_as::<_, SettingRow>(
            "SELECT key, category, value, description FROM settings WHERE category = ? ORDER BY key"
        )
        .bind(category)
        .fetch_all(&self.pool)
        .await?;

        Ok(settings
            .into_iter()
            .map(|r| SettingItem {
                key: r.key,
                category: r.category,
                value: r.value,
                description: r.description,
            })
            .collect())
    }

    /// 获取所有设置
    pub async fn get_all_settings(&self) -> Result<Vec<SettingItem>> {
        let settings = sqlx::query_as::<_, SettingRow>(
            "SELECT key, category, value, description FROM settings ORDER BY category, key",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(settings
            .into_iter()
            .map(|r| SettingItem {
                key: r.key,
                category: r.category,
                value: r.value,
                description: r.description,
            })
            .collect())
    }

    /// 更新设置
    pub async fn update_setting(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query("UPDATE settings SET value = ?, updated_at = CURRENT_TIMESTAMP WHERE key = ?")
            .bind(value)
            .bind(key)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    fn build_runtime_config(
        &self,
        settings_map: &std::collections::HashMap<String, String>,
    ) -> RuntimeConfig {
        let top_k = settings_map
            .get("scheduler.top_k")
            .and_then(|v| v.parse().ok())
            .unwrap_or(7);

        let score_weights: ScoreWeights = settings_map
            .get("scheduler.score_weights")
            .and_then(|v| serde_json::from_str(v).ok())
            .unwrap_or_default();

        let sticky_enabled = settings_map
            .get("sticky_session.enabled")
            .map(|v| v == "true")
            .unwrap_or(true);

        let sticky_ttl = settings_map
            .get("sticky_session.ttl_seconds")
            .and_then(|v| v.parse().ok())
            .unwrap_or(3600);

        let log_detail_mode = settings_map
            .get("stats.log_detail_mode")
            .cloned()
            .unwrap_or_else(|| "failures_only".to_string());

        let cost_source = settings_map
            .get("stats.cost.source")
            .cloned()
            .unwrap_or_else(|| "models.dev".to_string());

        let cost_refresh_hours = settings_map
            .get("stats.cost.refresh_interval_hours")
            .and_then(|v| v.parse().ok())
            .unwrap_or(24);

        RuntimeConfig {
            scheduler: SchedulerConfig {
                top_k,
                score_weights,
            },
            sticky_session: StickySessionConfig {
                enabled: sticky_enabled,
                ttl_seconds: sticky_ttl,
            },
            stats: StatsConfig {
                log_detail_mode,
                cost: CostConfig {
                    source: cost_source,
                    refresh_interval_hours: cost_refresh_hours,
                },
            },
        }
    }
}

/// 迁移定义
struct Migration {
    version: i32,
    name: &'static str,
    sql: &'static str,
}

/// 获取所有迁移
fn get_migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            name: "create_users",
            sql: r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        },
        Migration {
            version: 2,
            name: "create_channels",
            sql: r#"
            CREATE TABLE IF NOT EXISTS channels (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                api_keys TEXT NOT NULL DEFAULT '[]',
                endpoints TEXT NOT NULL DEFAULT '[]',
                model_maps TEXT NOT NULL DEFAULT '{}',
                rate_limit_rpm INTEGER,
                rate_limit_tpm INTEGER,
                failure_threshold INTEGER NOT NULL DEFAULT 3,
                blacklist_minutes INTEGER NOT NULL DEFAULT 10,
                concurrency INTEGER NOT NULL DEFAULT 10,
                enabled BOOLEAN NOT NULL DEFAULT TRUE,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        },
        Migration {
            version: 3,
            name: "create_api_keys",
            sql: r#"
            CREATE TABLE IF NOT EXISTS api_keys (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                api_key TEXT NOT NULL UNIQUE,
                enabled BOOLEAN NOT NULL DEFAULT TRUE,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        },
        Migration {
            version: 4,
            name: "create_groups",
            sql: r#"
            CREATE TABLE IF NOT EXISTS groups (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                match_regex TEXT,
                retry_enabled BOOLEAN NOT NULL DEFAULT TRUE,
                max_retries INTEGER NOT NULL DEFAULT 3,
                first_token_timeout_secs INTEGER NOT NULL DEFAULT 30,
                enabled BOOLEAN NOT NULL DEFAULT TRUE,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        },
        Migration {
            version: 5,
            name: "create_group_items",
            sql: r#"
            CREATE TABLE IF NOT EXISTS group_items (
                id TEXT PRIMARY KEY,
                group_id TEXT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
                channel_id TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
                model_name TEXT NOT NULL,
                priority INTEGER NOT NULL DEFAULT 1,
                weight INTEGER NOT NULL DEFAULT 100,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(group_id, channel_id, model_name)
            )
            "#,
        },
        Migration {
            version: 6,
            name: "create_model_pricing",
            sql: r#"
            CREATE TABLE IF NOT EXISTS model_pricing (
                id TEXT PRIMARY KEY,
                model TEXT NOT NULL UNIQUE,
                input_per_million REAL NOT NULL,
                output_per_million REAL NOT NULL,
                cache_read_per_million REAL,
                cache_creation_per_million REAL,
                source TEXT NOT NULL DEFAULT 'manual',
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        },
        Migration {
            version: 7,
            name: "create_usage_logs",
            sql: r#"
            CREATE TABLE IF NOT EXISTS usage_logs (
                id TEXT PRIMARY KEY,
                api_key_id TEXT REFERENCES api_keys(id),
                channel_id TEXT REFERENCES channels(id),
                group_id TEXT REFERENCES groups(id),
                requested_model TEXT NOT NULL,
                actual_model TEXT,
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                cache_read_tokens INTEGER NOT NULL DEFAULT 0,
                cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
                cost REAL,
                latency_ms INTEGER,
                status_code INTEGER,
                error_message TEXT,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        },
        Migration {
            version: 8,
            name: "create_usage_daily",
            sql: r#"
            CREATE TABLE IF NOT EXISTS usage_daily (
                id TEXT PRIMARY KEY,
                date TEXT NOT NULL,
                api_key_id TEXT REFERENCES api_keys(id),
                channel_id TEXT REFERENCES channels(id),
                group_id TEXT REFERENCES groups(id),
                model TEXT NOT NULL,
                request_count INTEGER NOT NULL DEFAULT 0,
                success_count INTEGER NOT NULL DEFAULT 0,
                failure_count INTEGER NOT NULL DEFAULT 0,
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                cache_read_tokens INTEGER NOT NULL DEFAULT 0,
                cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
                total_cost REAL NOT NULL DEFAULT 0,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(date, api_key_id, channel_id, group_id, model)
            )
            "#,
        },
        Migration {
            version: 9,
            name: "create_settings",
            sql: r#"
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                category TEXT NOT NULL DEFAULT 'general',
                value TEXT NOT NULL,
                description TEXT,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            INSERT OR IGNORE INTO settings (key, category, value, description) VALUES
                ('scheduler.top_k', 'scheduler', '7', 'Top-K 候选数量'),
                ('scheduler.score_weights', 'scheduler', '{"priority":1.0,"load":1.0,"queue":0.7,"error_rate":0.8,"ttft":0.5}', '评分权重'),
                ('sticky_session.enabled', 'sticky_session', 'true', '是否启用粘性会话'),
                ('sticky_session.ttl_seconds', 'sticky_session', '3600', '会话保持时间（秒）'),
                ('stats.log_detail_mode', 'stats', 'failures_only', '日志模式：all/failures_only/none'),
                ('stats.cost.source', 'stats', 'models.dev', '成本数据源'),
                ('stats.cost.refresh_interval_hours', 'stats', '24', '成本数据刷新间隔（小时）');
            "#,
        },
    ]
}
