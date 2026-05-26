use sqlx::SqlitePool;

/// 代理状态
#[derive(Clone)]
pub struct ProxyState {
    pub pool: SqlitePool,
    pub http_client: reqwest::Client,
}

/// 渠道信息
#[derive(Debug, Clone)]
pub struct ChannelInfo {
    pub id: String,
    pub name: String,
    pub channel_type: String,
    pub base_url: String,
    pub api_keys: Vec<String>,
    pub model_maps: serde_json::Value,
}

/// 分组信息
#[derive(Debug, Clone)]
pub struct GroupInfo {
    pub id: String,
    pub name: String,
    pub mode: String,
    pub items: Vec<GroupItemInfo>,
}

/// 分组项信息
#[derive(Debug, Clone)]
pub struct GroupItemInfo {
    pub channel_id: String,
    pub model_name: String,
    pub priority: i32,
    pub weight: i32,
}

/// 选择结果
#[derive(Debug)]
pub struct SelectionResult {
    pub channel: ChannelInfo,
    pub target_model: String,
}

impl ProxyState {
    pub fn new(pool: SqlitePool) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .unwrap();

        Self {
            pool,
            http_client,
        }
    }

    /// 选择渠道
    pub async fn select_channel(&self, model: &str) -> Result<SelectionResult, ProxyError> {
        // 1. 精确匹配分组名
        let group = self.find_group_by_name(model).await?;

        // 2. 如果没有精确匹配，尝试正则匹配
        let group = match group {
            Some(g) => Some(g),
            None => self.find_group_by_regex(model).await?,
        };

        // 3. 如果找到分组，从分组中选择渠道
        if let Some(group) = group {
            let item = self.select_group_item(&group).await?;
            let channel = self.get_channel(&item.channel_id).await?;

            // 4. 应用模型映射
            let target_model = self.apply_model_mapping(&channel, model);

            return Ok(SelectionResult {
                channel,
                target_model,
            });
        }

        // 5. 如果没有分组匹配，尝试直接查找渠道
        let channel = self.find_channel_by_model(model).await?;
        let target_model = self.apply_model_mapping(&channel, model);

        Ok(SelectionResult {
            channel,
            target_model,
        })
    }

    /// 根据名称查找分组
    async fn find_group_by_name(&self, name: &str) -> Result<Option<GroupInfo>, ProxyError> {
        let result = sqlx::query_as::<_, (String, String, String)>(
            "SELECT id, name, mode FROM groups WHERE name = ? AND enabled = 1"
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ProxyError::DatabaseError(e.to_string()))?;

        match result {
            Some((id, name, mode)) => {
                let items = self.get_group_items(&id).await?;
                Ok(Some(GroupInfo { id, name, mode, items }))
            }
            None => Ok(None),
        }
    }

    /// 根据正则查找分组
    async fn find_group_by_regex(&self, model: &str) -> Result<Option<GroupInfo>, ProxyError> {
        let groups = sqlx::query_as::<_, (String, String, String, Option<String>)>(
            "SELECT id, name, mode, match_regex FROM groups WHERE enabled = 1 AND match_regex IS NOT NULL"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ProxyError::DatabaseError(e.to_string()))?;

        for (id, name, mode, match_regex) in groups {
            if let Some(regex) = match_regex {
                if let Ok(re) = regex::Regex::new(&regex) {
                    if re.is_match(model) {
                        let items = self.get_group_items(&id).await?;
                        return Ok(Some(GroupInfo { id, name, mode, items }));
                    }
                }
            }
        }

        Ok(None)
    }

    /// 获取分组项
    async fn get_group_items(&self, group_id: &str) -> Result<Vec<GroupItemInfo>, ProxyError> {
        let items = sqlx::query_as::<_, (String, String, i32, i32)>(
            "SELECT channel_id, model_name, priority, weight FROM group_items WHERE group_id = ? ORDER BY priority DESC, weight DESC"
        )
        .bind(group_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ProxyError::DatabaseError(e.to_string()))?;

        Ok(items.into_iter().map(|(channel_id, model_name, priority, weight)| {
            GroupItemInfo {
                channel_id,
                model_name,
                priority,
                weight,
            }
        }).collect())
    }

    /// 从分组中选择一个渠道项
    async fn select_group_item(&self, group: &GroupInfo) -> Result<GroupItemInfo, ProxyError> {
        if group.items.is_empty() {
            return Err(ProxyError::NoAvailableChannel("分组没有可用渠道".to_string()));
        }

        match group.mode.as_str() {
            "round_robin" => {
                // 简单轮询：返回第一个
                Ok(group.items[0].clone())
            }
            "random" => {
                use rand::Rng;
                let mut rng = rand::rng();
                let idx = rng.random_range(0..group.items.len());
                Ok(group.items[idx].clone())
            }
            "failover" => {
                // 按优先级返回第一个
                Ok(group.items[0].clone())
            }
            "weighted" => {
                // 按权重随机选择
                let total_weight: i32 = group.items.iter().map(|i| i.weight).sum();
                if total_weight == 0 {
                    return Ok(group.items[0].clone());
                }

                use rand::Rng;
                let mut rng = rand::rng();
                let mut random_weight = rng.random_range(0..total_weight);

                for item in &group.items {
                    random_weight -= item.weight;
                    if random_weight < 0 {
                        return Ok(item.clone());
                    }
                }

                Ok(group.items[0].clone())
            }
            _ => Ok(group.items[0].clone()),
        }
    }

    /// 获取渠道信息
    async fn get_channel(&self, channel_id: &str) -> Result<ChannelInfo, ProxyError> {
        let result = sqlx::query_as::<_, (String, String, String, String, String)>(
            "SELECT id, name, type, base_url, api_keys FROM channels WHERE id = ? AND enabled = 1"
        )
        .bind(channel_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ProxyError::DatabaseError(e.to_string()))?;

        let (id, name, channel_type, base_url, api_keys_str) =
            result.ok_or_else(|| ProxyError::ChannelNotFound("渠道不存在或已禁用".to_string()))?;

        let api_keys: Vec<String> = serde_json::from_str(&api_keys_str).unwrap_or_default();

        let model_maps: serde_json::Value = sqlx::query_scalar::<_, String>(
            "SELECT model_maps FROM channels WHERE id = ?"
        )
        .bind(channel_id)
        .fetch_one(&self.pool)
        .await
        .map(|s| serde_json::from_str(&s).unwrap_or_default())
        .unwrap_or_default();

        Ok(ChannelInfo {
            id,
            name,
            channel_type,
            base_url,
            api_keys,
            model_maps,
        })
    }

    /// 根据模型查找渠道
    async fn find_channel_by_model(&self, model: &str) -> Result<ChannelInfo, ProxyError> {
        // 查找支持该模型的渠道
        let channels = sqlx::query_as::<_, (String, String, String, String, String)>(
            "SELECT id, name, type, base_url, api_keys FROM channels WHERE enabled = 1"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ProxyError::DatabaseError(e.to_string()))?;

        for (id, name, channel_type, base_url, api_keys_str) in channels {
            let api_keys: Vec<String> = serde_json::from_str(&api_keys_str).unwrap_or_default();

            let model_maps_str: String = sqlx::query_scalar(
                "SELECT model_maps FROM channels WHERE id = ?"
            )
            .bind(&id)
            .fetch_one(&self.pool)
            .await
            .unwrap_or_default();

            let model_maps: serde_json::Value = serde_json::from_str(&model_maps_str).unwrap_or_default();

            let channel = ChannelInfo {
                id: id.clone(),
                name,
                channel_type,
                base_url,
                api_keys,
                model_maps: model_maps.clone(),
            };

            // 检查模型映射
            if let Some(maps) = model_maps.as_object() {
                for (source, target) in maps {
                    if source == model || source == "*" {
                        return Ok(channel);
                    }
                    if source.contains('*') || source.contains('?') {
                        if wildcard_match(source, model) {
                            return Ok(channel);
                        }
                    }
                }
            }
        }

        Err(ProxyError::NoAvailableChannel("没有可用渠道".to_string()))
    }

    /// 应用模型映射
    fn apply_model_mapping(&self, channel: &ChannelInfo, model: &str) -> String {
        if let Some(maps) = channel.model_maps.as_object() {
            // 精确匹配
            if let Some(target) = maps.get(model) {
                if let Some(target_str) = target.as_str() {
                    return target_str.to_string();
                }
            }

            // 通配符匹配
            for (source, target) in maps {
                if source.contains('*') || source.contains('?') {
                    if let Some(target_str) = target.as_str() {
                        if wildcard_match(source, model) {
                            return target_str.replace('*', model);
                        }
                    }
                }
            }
        }

        model.to_string()
    }

    /// 选择 API Key（轮询）
    pub fn select_api_key(&self, channel: &ChannelInfo) -> String {
        if channel.api_keys.is_empty() {
            return String::new();
        }

        // 简单轮询：使用时间戳取模
        let idx = (chrono::Utc::now().timestamp_millis() as usize) % channel.api_keys.len();
        channel.api_keys[idx].clone()
    }
}

/// 通配符匹配
fn wildcard_match(pattern: &str, text: &str) -> bool {
    let regex_pattern = pattern
        .replace('.', "\\.")
        .replace('*', ".*")
        .replace('?', ".");

    if let Ok(re) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
        re.is_match(text)
    } else {
        false
    }
}

/// 代理错误
#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("数据库错误: {0}")]
    DatabaseError(String),

    #[error("渠道不存在: {0}")]
    ChannelNotFound(String),

    #[error("没有可用渠道: {0}")]
    NoAvailableChannel(String),

    #[error("请求失败: {0}")]
    RequestError(String),

    #[error("转换失败: {0}")]
    TransformError(String),
}
