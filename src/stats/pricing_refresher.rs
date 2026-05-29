use std::path::PathBuf;
use std::sync::Arc;
use tokio::time::{interval, Duration};

use super::model::ModelRegistry;

/// 模型信息定时刷新器
pub struct PricingRefresher {
    registry: ModelRegistry,
    cache_path: PathBuf,
    providers: Vec<String>,
    refresh_interval_hours: u64,
}

impl PricingRefresher {
    pub fn new(registry: ModelRegistry, cache_path: PathBuf, providers: Vec<String>, refresh_interval_hours: u64) -> Self {
        Self {
            registry,
            cache_path,
            providers,
            refresh_interval_hours,
        }
    }

    pub fn start(self: Arc<Self>) {
        let refresher = self.clone();
        tokio::spawn(async move {
            refresher.run().await;
        });
    }

    async fn run(&self) {
        let mut tick = interval(Duration::from_secs(self.refresh_interval_hours * 3600));

        loop {
            tick.tick().await;

            tracing::info!("开始定时刷新模型信息");
            if let Err(e) = self.registry.fetch_remote_pricing(&self.cache_path, &self.providers).await {
                tracing::warn!("定时刷新模型信息失败: {}", e);
            } else {
                let count = self.registry.get_all_models().await.len();
                tracing::info!("模型信息刷新完成，当前 {} 条", count);
            }
        }
    }
}
