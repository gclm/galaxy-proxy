use crate::api::response::generate_id;
use sqlx::SqlitePool;

/// 统计记录器
#[derive(Clone)]
pub struct StatsRecorder {
    pool: SqlitePool,
}

/// 单次渠道尝试记录
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChannelAttempt {
    pub channel_id: String,
    pub channel_name: Option<String>,
    pub status: String,
    pub duration_ms: i64,
    pub error: Option<String>,
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
    pub ttft_ms: Option<i32>,
    pub status_code: Option<i32>,
    pub error_message: Option<String>,
    pub endpoint_type: Option<String>,
    pub request_type: String,
    pub request_content: Option<String>,
    pub response_content: Option<String>,
    pub is_stream: bool,
    pub attempts: Vec<ChannelAttempt>,
}

impl StatsRecorder {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 记录请求日志
    pub async fn record_request(&self, record: RequestRecord) -> Result<(), sqlx::Error> {
        let id = generate_id();
        let attempts_json = if record.attempts.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&record.attempts).unwrap_or_default())
        };

        sqlx::query(
            r#"
            INSERT INTO usage_logs (
                id, api_key_id, channel_id, group_id,
                requested_model, actual_model,
                input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                cost, latency_ms, ttft_ms, status_code, error_message,
                endpoint_type, request_type, request_content, response_content, is_stream,
                attempts
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
        .bind(record.ttft_ms)
        .bind(record.status_code)
        .bind(&record.error_message)
        .bind(&record.endpoint_type)
        .bind(&record.request_type)
        .bind(&record.request_content)
        .bind(&record.response_content)
        .bind(record.is_stream)
        .bind(&attempts_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
