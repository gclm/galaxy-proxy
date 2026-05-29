use std::sync::Arc;
use tokio::time::{Duration, interval};

use super::state::LoadBalancerState;

/// 定时任务调度器
pub struct Scheduler {
    lb_state: LoadBalancerState,
}

impl Scheduler {
    pub fn new(lb_state: LoadBalancerState) -> Self {
        Self { lb_state }
    }

    /// 启动定时任务
    pub fn start(self: Arc<Self>) {
        let scheduler = self.clone();
        tokio::spawn(async move {
            scheduler.run_cleanup().await;
        });
    }

    /// 清理任务
    async fn run_cleanup(&self) {
        let mut interval = interval(Duration::from_secs(60)); // 每分钟执行一次

        loop {
            interval.tick().await;

            // 清理过期的粘性会话
            self.lb_state.cleanup_expired_sessions().await;

            // 清理过期的拉黑
            self.lb_state.cleanup_expired_blacklists().await;
        }
    }
}
