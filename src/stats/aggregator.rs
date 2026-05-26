use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use chrono::Timelike;

/// 统计聚合器
pub struct StatsAggregator {
    pool: SqlitePool,
}

impl StatsAggregator {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 启动定时聚合任务
    pub fn start(self: Arc<Self>) {
        let aggregator = self.clone();
        tokio::spawn(async move {
            aggregator.run_aggregation().await;
        });
    }

    /// 运行聚合任务
    async fn run_aggregation(&self) {
        // 每天凌晨 2 点执行聚合
        let mut interval = interval(Duration::from_secs(3600)); // 每小时检查一次

        loop {
            interval.tick().await;

            let now = chrono::Utc::now();
            if now.hour() == 2 && now.minute() < 60 {
                if let Err(e) = self.aggregate_daily_stats().await {
                    tracing::error!("聚合每日统计失败: {}", e);
                }
            }
        }
    }

    /// 聚合每日统计
    async fn aggregate_daily_stats(&self) -> Result<(), sqlx::Error> {
        let yesterday = (chrono::Utc::now() - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        // 从 usage_logs 聚合到 usage_daily
        sqlx::query(
            r#"
            INSERT INTO usage_daily (
                id, date, api_key_id, channel_id, group_id, model,
                request_count, success_count, failure_count,
                input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                total_cost
            )
            SELECT
                lower(hex(randomblob(16))),
                date(created_at) as date,
                api_key_id,
                channel_id,
                group_id,
                requested_model,
                COUNT(*),
                SUM(CASE WHEN status_code >= 200 AND status_code < 400 THEN 1 ELSE 0 END),
                SUM(CASE WHEN status_code < 200 OR status_code >= 400 THEN 1 ELSE 0 END),
                SUM(input_tokens),
                SUM(output_tokens),
                SUM(cache_read_tokens),
                SUM(cache_creation_tokens),
                SUM(COALESCE(cost, 0))
            FROM usage_logs
            WHERE date(created_at) = ?
            GROUP BY date(created_at), api_key_id, channel_id, group_id, requested_model
            ON CONFLICT(date, api_key_id, channel_id, group_id, model) DO UPDATE SET
                request_count = excluded.request_count,
                success_count = excluded.success_count,
                failure_count = excluded.failure_count,
                input_tokens = excluded.input_tokens,
                output_tokens = excluded.output_tokens,
                cache_read_tokens = excluded.cache_read_tokens,
                cache_creation_tokens = excluded.cache_creation_tokens,
                total_cost = excluded.total_cost
            "#
        )
        .bind(&yesterday)
        .execute(&self.pool)
        .await?;

        tracing::info!("每日统计聚合完成: {}", yesterday);
        Ok(())
    }
}
