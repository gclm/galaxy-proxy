pub mod aggregator;
pub mod cost;
pub mod recorder;

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// 统计状态
#[derive(Clone)]
pub struct StatsState {
    pub pool: SqlitePool,
}

/// 渠道统计行类型
type ChannelStatsRow = (String, String, i32, i32, i32, i32, i32, f64);

/// 用量日志
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UsageLog {
    pub id: String,
    pub api_key_id: Option<String>,
    pub channel_id: Option<String>,
    pub group_id: Option<String>,
    pub requested_model: String,
    pub actual_model: Option<String>,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub cache_read_tokens: i32,
    pub cache_creation_tokens: i32,
    pub cost: Option<f64>,
    pub latency_ms: Option<i32>,
    pub status_code: Option<i32>,
    pub error_message: Option<String>,
    pub created_at: String,
}

/// 每日统计
#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct UsageDaily {
    pub id: String,
    pub date: String,
    pub api_key_id: Option<String>,
    pub channel_id: Option<String>,
    pub group_id: Option<String>,
    pub model: String,
    pub request_count: i32,
    pub success_count: i32,
    pub failure_count: i32,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub cache_read_tokens: i32,
    pub cache_creation_tokens: i32,
    pub total_cost: f64,
}

/// 统计概览
#[derive(Debug, Serialize, Deserialize)]
pub struct StatsOverview {
    pub total_requests: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost: f64,
    pub today_requests: i64,
    pub today_input_tokens: i64,
    pub today_output_tokens: i64,
    pub today_cost: f64,
}

/// 按模型统计
#[derive(Debug, Serialize, Deserialize)]
pub struct ModelStats {
    pub model: String,
    pub request_count: i32,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub total_cost: f64,
}

/// 按渠道统计
#[derive(Debug, Serialize, Deserialize)]
pub struct ChannelStats {
    pub channel_id: String,
    pub channel_name: String,
    pub request_count: i32,
    pub success_count: i32,
    pub failure_count: i32,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub total_cost: f64,
}

impl StatsState {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 获取统计概览
    pub async fn get_overview(&self) -> Result<StatsOverview, sqlx::Error> {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        // 总计
        let total = sqlx::query_as::<_, (i64, i64, i64, f64)>(
            "SELECT
                COALESCE(SUM(request_count), 0),
                COALESCE(SUM(input_tokens), 0),
                COALESCE(SUM(output_tokens), 0),
                COALESCE(SUM(total_cost), 0)
            FROM usage_daily",
        )
        .fetch_one(&self.pool)
        .await?;

        // 今日
        let today_stats = sqlx::query_as::<_, (i64, i64, i64, f64)>(
            "SELECT
                COALESCE(SUM(request_count), 0),
                COALESCE(SUM(input_tokens), 0),
                COALESCE(SUM(output_tokens), 0),
                COALESCE(SUM(total_cost), 0)
            FROM usage_daily
            WHERE date = ?",
        )
        .bind(&today)
        .fetch_one(&self.pool)
        .await?;

        Ok(StatsOverview {
            total_requests: total.0,
            total_input_tokens: total.1,
            total_output_tokens: total.2,
            total_cost: total.3,
            today_requests: today_stats.0,
            today_input_tokens: today_stats.1,
            today_output_tokens: today_stats.2,
            today_cost: today_stats.3,
        })
    }

    /// 获取按模型统计
    pub async fn get_model_stats(&self, days: i32) -> Result<Vec<ModelStats>, sqlx::Error> {
        let start_date = (chrono::Utc::now() - chrono::Duration::days(days as i64))
            .format("%Y-%m-%d")
            .to_string();

        let stats = sqlx::query_as::<_, (String, i32, i32, i32, f64)>(
            "SELECT
                model,
                SUM(request_count),
                SUM(input_tokens),
                SUM(output_tokens),
                SUM(total_cost)
            FROM usage_daily
            WHERE date >= ?
            GROUP BY model
            ORDER BY SUM(request_count) DESC",
        )
        .bind(&start_date)
        .fetch_all(&self.pool)
        .await?;

        Ok(stats
            .into_iter()
            .map(|(model, requests, input, output, cost)| ModelStats {
                model,
                request_count: requests,
                input_tokens: input,
                output_tokens: output,
                total_cost: cost,
            })
            .collect())
    }

    /// 获取按渠道统计
    pub async fn get_channel_stats(&self, days: i32) -> Result<Vec<ChannelStats>, sqlx::Error> {
        let start_date = (chrono::Utc::now() - chrono::Duration::days(days as i64))
            .format("%Y-%m-%d")
            .to_string();

        let rows: Vec<ChannelStatsRow> = sqlx::query_as(
            "SELECT
                u.channel_id,
                COALESCE(c.name, 'unknown'),
                SUM(u.request_count),
                SUM(u.success_count),
                SUM(u.failure_count),
                SUM(u.input_tokens),
                SUM(u.output_tokens),
                SUM(u.total_cost)
            FROM usage_daily u
            LEFT JOIN channels c ON u.channel_id = c.id
            WHERE u.date >= ?
            GROUP BY u.channel_id
            ORDER BY SUM(u.request_count) DESC",
        )
        .bind(&start_date)
        .fetch_all(&self.pool)
        .await?;

        let stats = rows
            .into_iter()
            .map(
                |(id, name, requests, success, failure, input, output, cost)| ChannelStats {
                    channel_id: id,
                    channel_name: name,
                    request_count: requests,
                    success_count: success,
                    failure_count: failure,
                    input_tokens: input,
                    output_tokens: output,
                    total_cost: cost,
                },
            )
            .collect();

        Ok(stats)
    }

    /// 获取按天统计
    pub async fn get_daily_stats(&self, days: i32) -> Result<Vec<UsageDaily>, sqlx::Error> {
        let start_date = (chrono::Utc::now() - chrono::Duration::days(days as i64))
            .format("%Y-%m-%d")
            .to_string();

        let rows: Vec<UsageDaily> = sqlx::query_as(
            "SELECT id, date, api_key_id, channel_id, group_id, model, request_count, success_count, failure_count, input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens, total_cost FROM usage_daily WHERE date >= ? ORDER BY date DESC"
        )
        .bind(&start_date)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}
