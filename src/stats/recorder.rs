use crate::api::response::generate_id;
use sqlx::SqlitePool;

/// 统计记录器
#[derive(Clone)]
pub struct StatsRecorder {
    pool: SqlitePool,
}

/// 记录请求
#[derive(Debug)]
pub struct RequestRecord {
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
}

impl StatsRecorder {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 记录请求日志
    pub async fn record_request(&self, record: RequestRecord) -> Result<(), sqlx::Error> {
        let id = generate_id();

        sqlx::query(
            r#"
            INSERT INTO usage_logs (
                id, api_key_id, channel_id, group_id,
                requested_model, actual_model,
                input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                cost, latency_ms, status_code, error_message
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&record.api_key_id)
        .bind(&record.channel_id)
        .bind(&record.group_id)
        .bind(&record.requested_model)
        .bind(&record.actual_model)
        .bind(record.input_tokens)
        .bind(record.output_tokens)
        .bind(record.cache_read_tokens)
        .bind(record.cache_creation_tokens)
        .bind(record.cost)
        .bind(record.latency_ms)
        .bind(record.status_code)
        .bind(&record.error_message)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 更新每日统计
    pub async fn update_daily_stats(&self, record: &RequestRecord) -> Result<(), sqlx::Error> {
        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let id = generate_id();

        let is_success = record.status_code.is_none_or(|s| (200..400).contains(&s));

        sqlx::query(
            r#"
            INSERT INTO usage_daily (
                id, date, api_key_id, channel_id, group_id, model,
                request_count, success_count, failure_count,
                input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                total_cost
            ) VALUES (?, ?, ?, ?, ?, ?, 1, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(date, api_key_id, channel_id, group_id, model) DO UPDATE SET
                request_count = request_count + 1,
                success_count = success_count + ?,
                failure_count = failure_count + ?,
                input_tokens = input_tokens + ?,
                output_tokens = output_tokens + ?,
                cache_read_tokens = cache_read_tokens + ?,
                cache_creation_tokens = cache_creation_tokens + ?,
                total_cost = total_cost + ?
            "#,
        )
        .bind(&id)
        .bind(&date)
        .bind(&record.api_key_id)
        .bind(&record.channel_id)
        .bind(&record.group_id)
        .bind(&record.requested_model)
        .bind(if is_success { 1 } else { 0 })
        .bind(if is_success { 0 } else { 1 })
        .bind(record.input_tokens)
        .bind(record.output_tokens)
        .bind(record.cache_read_tokens)
        .bind(record.cache_creation_tokens)
        .bind(record.cost.unwrap_or(0.0))
        // ON CONFLICT 部分
        .bind(if is_success { 1 } else { 0 })
        .bind(if is_success { 0 } else { 1 })
        .bind(record.input_tokens)
        .bind(record.output_tokens)
        .bind(record.cache_read_tokens)
        .bind(record.cache_creation_tokens)
        .bind(record.cost.unwrap_or(0.0))
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
