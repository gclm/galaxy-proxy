use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 模型定价
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub model: String,
    pub input_per_million: f64,
    pub output_per_million: f64,
    pub cache_read_per_million: Option<f64>,
    pub cache_creation_per_million: Option<f64>,
}

/// models.dev API 响应
#[derive(Debug, Deserialize)]
struct ModelsDevResponse {
    models: Vec<ModelsDevModel>,
}

#[derive(Debug, Deserialize)]
struct ModelsDevModel {
    id: String,
    pricing: Option<ModelsDevPricing>,
}

#[derive(Debug, Deserialize)]
struct ModelsDevPricing {
    prompt: Option<String>,
    completion: Option<String>,
}

/// 成本计算器
#[derive(Clone)]
pub struct CostCalculator {
    /// 本地定价覆盖
    local_pricing: Arc<RwLock<HashMap<String, ModelPricing>>>,
    /// models.dev 定价
    remote_pricing: Arc<RwLock<HashMap<String, ModelPricing>>>,
    /// HTTP 客户端
    http_client: reqwest::Client,
}

impl Default for CostCalculator {
    fn default() -> Self {
        Self::new()
    }
}

impl CostCalculator {
    pub fn new() -> Self {
        Self {
            local_pricing: Arc::new(RwLock::new(HashMap::new())),
            remote_pricing: Arc::new(RwLock::new(HashMap::new())),
            http_client: reqwest::Client::new(),
        }
    }

    /// 从数据库加载本地定价
    pub async fn load_local_pricing(&self, pool: &sqlx::SqlitePool) -> Result<(), sqlx::Error> {
        let pricing = sqlx::query_as::<_, (String, f64, f64, Option<f64>, Option<f64>)>(
            "SELECT model, input_per_million, output_per_million, cache_read_per_million, cache_creation_per_million FROM model_pricing WHERE source = 'manual'"
        )
        .fetch_all(pool)
        .await?;

        let mut local = self.local_pricing.write().await;
        for (model, input, output, cache_read, cache_creation) in pricing {
            local.insert(
                model.clone(),
                ModelPricing {
                    model,
                    input_per_million: input,
                    output_per_million: output,
                    cache_read_per_million: cache_read,
                    cache_creation_per_million: cache_creation,
                },
            );
        }

        Ok(())
    }

    /// 从 models.dev 拉取定价
    pub async fn fetch_remote_pricing(&self) -> Result<(), reqwest::Error> {
        let response = self
            .http_client
            .get("https://models.dev/api.json")
            .send()
            .await?
            .json::<ModelsDevResponse>()
            .await?;

        let mut remote = self.remote_pricing.write().await;
        for model in response.models {
            if let Some(pricing) = model.pricing {
                let input = pricing
                    .prompt
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let output = pricing
                    .completion
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);

                remote.insert(
                    model.id.clone(),
                    ModelPricing {
                        model: model.id,
                        input_per_million: input,
                        output_per_million: output,
                        cache_read_per_million: None,
                        cache_creation_per_million: None,
                    },
                );
            }
        }

        Ok(())
    }

    /// 保存远程定价到数据库
    pub async fn save_remote_pricing(&self, pool: &sqlx::SqlitePool) -> Result<(), sqlx::Error> {
        let remote = self.remote_pricing.read().await;

        for (_, pricing) in remote.iter() {
            sqlx::query(
                r#"
                INSERT INTO model_pricing (model, input_per_million, output_per_million, source)
                VALUES (?, ?, ?, 'models.dev')
                ON CONFLICT(model) DO UPDATE SET
                    input_per_million = excluded.input_per_million,
                    output_per_million = excluded.output_per_million,
                    updated_at = CURRENT_TIMESTAMP
                WHERE source = 'models.dev'
                "#,
            )
            .bind(&pricing.model)
            .bind(pricing.input_per_million)
            .bind(pricing.output_per_million)
            .execute(pool)
            .await?;
        }

        Ok(())
    }

    /// 计算成本
    pub async fn calculate_cost(
        &self,
        model: &str,
        input_tokens: i32,
        output_tokens: i32,
        cache_read_tokens: i32,
        cache_creation_tokens: i32,
    ) -> f64 {
        // 优先使用本地定价
        let local = self.local_pricing.read().await;
        if let Some(pricing) = local.get(model) {
            return self.compute_cost(
                pricing,
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cache_creation_tokens,
            );
        }

        // 然后使用远程定价
        let remote = self.remote_pricing.read().await;
        if let Some(pricing) = remote.get(model) {
            return self.compute_cost(
                pricing,
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cache_creation_tokens,
            );
        }

        // 模糊匹配（移除版本后缀）
        let base_model = model.rsplitn(2, '-').last().unwrap_or(model);
        for (key, pricing) in remote.iter() {
            if key.starts_with(base_model) || base_model.starts_with(key.as_str()) {
                return self.compute_cost(
                    pricing,
                    input_tokens,
                    output_tokens,
                    cache_read_tokens,
                    cache_creation_tokens,
                );
            }
        }

        0.0
    }

    fn compute_cost(
        &self,
        pricing: &ModelPricing,
        input_tokens: i32,
        output_tokens: i32,
        cache_read_tokens: i32,
        cache_creation_tokens: i32,
    ) -> f64 {
        let input_cost = (input_tokens as f64 / 1_000_000.0) * pricing.input_per_million;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * pricing.output_per_million;

        let cache_read_cost = if let Some(rate) = pricing.cache_read_per_million {
            (cache_read_tokens as f64 / 1_000_000.0) * rate
        } else {
            0.0
        };

        let cache_creation_cost = if let Some(rate) = pricing.cache_creation_per_million {
            (cache_creation_tokens as f64 / 1_000_000.0) * rate
        } else {
            0.0
        };

        input_cost + output_cost + cache_read_cost + cache_creation_cost
    }

    /// 获取定价信息
    pub async fn get_pricing(&self, model: &str) -> Option<ModelPricing> {
        let local = self.local_pricing.read().await;
        if let Some(pricing) = local.get(model) {
            return Some(pricing.clone());
        }

        let remote = self.remote_pricing.read().await;
        remote.get(model).cloned()
    }

    /// 设置本地定价
    pub async fn set_local_pricing(&self, pricing: ModelPricing) {
        let mut local = self.local_pricing.write().await;
        local.insert(pricing.model.clone(), pricing);
    }

    /// 获取所有定价
    pub async fn get_all_pricing(&self) -> Vec<ModelPricing> {
        let mut all = Vec::new();

        let local = self.local_pricing.read().await;
        for (_, pricing) in local.iter() {
            all.push(pricing.clone());
        }

        let remote = self.remote_pricing.read().await;
        for (_, pricing) in remote.iter() {
            if !local.contains_key(&pricing.model) {
                all.push(pricing.clone());
            }
        }

        all
    }
}
