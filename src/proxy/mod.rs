pub mod scheduler;
pub mod state;

use self::state::LoadBalancerState;
use crate::api::handlers::admin::channels::{CustomHeader, EndpointConfig, EndpointType};
use crate::protocol::inbound::Inbound;
use crate::protocol::outbound::Outbound;
use crate::stats::cost::CostCalculator;
use crate::stats::recorder::StatsRecorder;
use axum::body::Bytes;
use axum::http::{HeaderMap, StatusCode};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 缓存大小限制
const CACHE_MAX_SIZE: usize = 1000;

/// 渠道/分组缓存
#[derive(Clone)]
pub struct ProxyCache {
    channels: Arc<RwLock<HashMap<String, ChannelInfo>>>,
    groups: Arc<RwLock<HashMap<String, GroupInfo>>>,
}

impl ProxyCache {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            groups: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取缓存的渠道
    pub async fn get_channel(&self, id: &str) -> Option<ChannelInfo> {
        let cache = self.channels.read().await;
        cache.get(id).cloned()
    }

    /// 设置渠道缓存（超过限制时清除最旧条目）
    pub async fn set_channel(&self, channel: ChannelInfo) {
        let mut cache = self.channels.write().await;
        if cache.len() >= CACHE_MAX_SIZE {
            if let Some(oldest_key) = cache.keys().next().cloned() {
                cache.remove(&oldest_key);
            }
        }
        cache.insert(channel.id.clone(), channel);
    }

    /// 清除渠道缓存
    pub async fn invalidate_channel(&self, id: &str) {
        let mut cache = self.channels.write().await;
        cache.remove(id);
    }

    /// 清除所有渠道缓存
    pub async fn invalidate_all_channels(&self) {
        let mut cache = self.channels.write().await;
        cache.clear();
    }

    /// 获取缓存的分组
    pub async fn get_group(&self, name: &str) -> Option<GroupInfo> {
        let cache = self.groups.read().await;
        cache.get(name).cloned()
    }

    /// 设置分组缓存（超过限制时清除最旧条目）
    pub async fn set_group(&self, group: GroupInfo) {
        let mut cache = self.groups.write().await;
        if cache.len() >= CACHE_MAX_SIZE {
            if let Some(oldest_key) = cache.keys().next().cloned() {
                cache.remove(&oldest_key);
            }
        }
        cache.insert(group.name.clone(), group);
    }

    /// 清除分组缓存
    pub async fn invalidate_group(&self, name: &str) {
        let mut cache = self.groups.write().await;
        cache.remove(name);
    }

    /// 清除所有分组缓存
    pub async fn invalidate_all_groups(&self) {
        let mut cache = self.groups.write().await;
        cache.clear();
    }
}

/// 请求队列（流量控制）
#[derive(Clone)]
pub struct RequestQueue {
    semaphore: Arc<tokio::sync::Semaphore>,
    max_queue_size: usize,
    timeout_secs: u64,
}

impl RequestQueue {
    pub fn new(max_queue_size: usize, timeout_secs: u64) -> Self {
        Self {
            semaphore: Arc::new(tokio::sync::Semaphore::new(max_queue_size)),
            max_queue_size,
            timeout_secs,
        }
    }

    /// 获取队列许可（超时返回 429）
    pub async fn acquire(&self) -> Result<tokio::sync::SemaphorePermit<'_>, QueueError> {
        match tokio::time::timeout(
            std::time::Duration::from_secs(self.timeout_secs),
            self.semaphore.acquire(),
        )
        .await
        {
            Ok(Ok(permit)) => Ok(permit),
            Ok(Err(_)) => Err(QueueError::QueueClosed),
            Err(_) => Err(QueueError::QueueFull {
                max: self.max_queue_size,
                timeout: self.timeout_secs,
            }),
        }
    }
}

/// 队列错误
#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("队列已满，最大排队数: {max}，超时: {timeout}s")]
    QueueFull { max: usize, timeout: u64 },

    #[error("队列已关闭")]
    QueueClosed,
}

/// 代理状态
#[derive(Clone)]
pub struct ProxyState {
    pub pool: SqlitePool,
    pub http_client: reqwest::Client,
    pub lb_state: LoadBalancerState,
    pub stats_recorder: StatsRecorder,
    pub cost_calculator: CostCalculator,
    pub cache: ProxyCache,
    pub queue: Option<RequestQueue>,
}

/// 渠道信息
#[derive(Debug, Clone)]
pub struct ChannelInfo {
    pub id: String,
    pub name: String,
    pub api_keys: Vec<String>,
    pub endpoints: Vec<EndpointConfig>,
    pub models: Vec<String>,
    pub custom_headers: Vec<CustomHeader>,
}

/// 分组信息
#[derive(Debug, Clone)]
pub struct GroupInfo {
    pub id: String,
    pub name: String,
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
    pub endpoint: EndpointConfig,
}

/// 代理成功结果（非流式）
pub struct ProxySuccess {
    pub status: StatusCode,
    pub body: Vec<u8>,
}

impl ProxyState {
    pub async fn new(pool: SqlitePool) -> Self {
        let proxy_enabled: bool = sqlx::query_scalar::<_, String>(
            "SELECT value FROM settings WHERE key = 'proxy.enabled'",
        )
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(false);

        let proxy_url = if proxy_enabled {
            sqlx::query_scalar::<_, String>(
                "SELECT value FROM settings WHERE key = 'proxy.url'",
            )
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten()
            .filter(|v| !v.is_empty())
        } else {
            None
        };

        let mut client_builder = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300));

        if let Some(url) = proxy_url {
            match reqwest::Proxy::all(&url) {
                Ok(proxy) => {
                    tracing::info!("上游代理已启用: {}", url);
                    client_builder = client_builder.proxy(proxy);
                }
                Err(e) => {
                    tracing::warn!("代理配置无效，忽略代理: {}", e);
                    client_builder = client_builder.no_proxy();
                }
            }
        } else {
            client_builder = client_builder.no_proxy();
        }

        let http_client = client_builder
            .build()
            .expect("Failed to create HTTP client");

        Self {
            stats_recorder: StatsRecorder::new(pool.clone()),
            cost_calculator: CostCalculator::new(),
            cache: ProxyCache::new(),
            queue: None,
            pool,
            http_client,
            lb_state: LoadBalancerState::new(),
        }
    }

    /// 设置请求队列
    pub fn with_queue(mut self, max_queue_size: usize, timeout_secs: u64) -> Self {
        self.queue = Some(RequestQueue::new(max_queue_size, timeout_secs));
        self
    }

    /// 选择渠道和端点（精确匹配端点类型）
    pub async fn select_channel(
        &self,
        model: &str,
        endpoint_type: EndpointType,
        session_hash: Option<&str>,
    ) -> Result<SelectionResult, ProxyError> {
        self.select_channel_inner(model, session_hash, &[], |ch| ch.find_endpoint(&endpoint_type))
            .await
    }

    /// 选择渠道和端点（支持排除已失败渠道，精确匹配端点类型）
    pub async fn select_channel_with_exclude(
        &self,
        model: &str,
        endpoint_type: EndpointType,
        session_hash: Option<&str>,
        exclude_ids: &[String],
    ) -> Result<SelectionResult, ProxyError> {
        self.select_channel_inner(model, session_hash, exclude_ids, |ch| ch.find_endpoint(&endpoint_type))
            .await
    }

    /// 按模型选择渠道（不限端点类型，用于跨协议转换）
    pub async fn select_channel_for_model(
        &self,
        model: &str,
        session_hash: Option<&str>,
    ) -> Result<SelectionResult, ProxyError> {
        self.select_channel_inner(model, session_hash, &[], |ch| ch.endpoints.first().cloned())
            .await
    }

    /// 按模型选择渠道（支持排除已失败渠道，不限端点类型）
    pub async fn select_channel_for_model_with_exclude(
        &self,
        model: &str,
        session_hash: Option<&str>,
        exclude_ids: &[String],
    ) -> Result<SelectionResult, ProxyError> {
        self.select_channel_inner(model, session_hash, exclude_ids, |ch| ch.endpoints.first().cloned())
            .await
    }

    /// 选择渠道内部实现（统一逻辑）
    async fn select_channel_inner(
        &self,
        model: &str,
        session_hash: Option<&str>,
        exclude_ids: &[String],
        find_endpoint: impl Fn(&ChannelInfo) -> Option<EndpointConfig>,
    ) -> Result<SelectionResult, ProxyError> {
        // 1. 检查粘性会话
        if let Some(hash) = session_hash
            && let Some(channel_id) = self.lb_state.get_sticky_session(hash).await
                && !exclude_ids.contains(&channel_id)
                    && let Ok(channel) = self.get_channel(&channel_id).await
                        && let Some(endpoint) = find_endpoint(&channel) {
                            let target_model = self.apply_model_mapping(&channel, model);
                            return Ok(SelectionResult { channel, target_model, endpoint });
                        }

        // 2. 查找分组（精确匹配 → 正则匹配）
        let group = match self.find_group_by_name(model).await? {
            Some(g) => Some(g),
            None => self.find_group_by_regex(model).await?,
        };

        // 3. 从分组中选择渠道
        if let Some(group) = group {
            if let Ok(item) = self.select_group_item(&group, exclude_ids).await {
                let channel = self.get_channel(&item.channel_id).await?;
                if let Some(endpoint) = find_endpoint(&channel) {
                    if let Some(hash) = session_hash {
                        self.lb_state.set_sticky_session(hash, &channel.id).await;
                    }
                    let target_model = self.apply_model_mapping(&channel, model);
                    return Ok(SelectionResult { channel, target_model, endpoint });
                }
            }
        }

        // 4. 直接查找渠道
        let channel = self.find_channel_by_model(model, exclude_ids, |ch| find_endpoint(ch).is_some()).await?;
        if let Some(endpoint) = find_endpoint(&channel) {
            if let Some(hash) = session_hash {
                self.lb_state.set_sticky_session(hash, &channel.id).await;
            }
            let target_model = self.apply_model_mapping(&channel, model);
            return Ok(SelectionResult { channel, target_model, endpoint });
        }

        Err(ProxyError::NoAvailableChannel("没有可用渠道".to_string()))
    }

    /// 根据名称查找分组
    /// 根据名称查找分组（带缓存）
    async fn find_group_by_name(&self, name: &str) -> Result<Option<GroupInfo>, ProxyError> {
        // 1. 检查缓存
        if let Some(group) = self.cache.get_group(name).await {
            return Ok(Some(group));
        }

        // 2. 缓存未命中，查询数据库
        let result = sqlx::query_as::<_, (String, String)>(
            "SELECT id, name FROM groups WHERE name = ? AND enabled = 1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ProxyError::DatabaseError(e.to_string()))?;

        match result {
            Some((id, name)) => {
                let items = self.get_group_items(&id).await?;
                let group = GroupInfo {
                    id,
                    name: name.clone(),
                    items,
                };
                // 3. 写入缓存
                self.cache.set_group(group.clone()).await;
                Ok(Some(group))
            }
            None => Ok(None),
        }
    }

    /// 根据正则查找分组
    async fn find_group_by_regex(&self, model: &str) -> Result<Option<GroupInfo>, ProxyError> {
        let groups = sqlx::query_as::<_, (String, String, Option<String>)>(
            "SELECT id, name, match_regex FROM groups WHERE enabled = 1 AND match_regex IS NOT NULL"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ProxyError::DatabaseError(e.to_string()))?;

        for (id, name, match_regex) in groups {
            if let Some(regex) = match_regex
                && let Ok(re) = regex::Regex::new(&regex)
                    && re.is_match(model) {
                        let items = self.get_group_items(&id).await?;
                        return Ok(Some(GroupInfo { id, name, items }));
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

        Ok(items
            .into_iter()
            .map(|(channel_id, model_name, priority, weight)| GroupItemInfo {
                channel_id,
                model_name,
                priority,
                weight,
            })
            .collect())
    }

    /// 从分组中选择一个渠道项（自适应负载均衡，支持排除）
    async fn select_group_item(
        &self,
        group: &GroupInfo,
        exclude_ids: &[String],
    ) -> Result<GroupItemInfo, ProxyError> {
        if group.items.is_empty() {
            return Err(ProxyError::NoAvailableChannel(
                "分组没有可用渠道".to_string(),
            ));
        }

        // 计算每个渠道的评分（排除已失败渠道）
        let mut scored_items: Vec<(f64, &GroupItemInfo)> = Vec::new();

        for item in &group.items {
            if exclude_ids.contains(&item.channel_id) {
                continue;
            }
            let score = self
                .lb_state
                .calculate_score(&item.channel_id, item.weight)
                .await;
            if score > 0.0 {
                scored_items.push((score, item));
            }
        }

        if scored_items.is_empty() {
            return Err(ProxyError::NoAvailableChannel(
                "所有渠道都不可用".to_string(),
            ));
        }

        // 按评分排序
        scored_items.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Top-K 加权随机选择（K=3）
        let top_k = 3.min(scored_items.len());
        let top_items = &scored_items[..top_k];

        // 加权随机选择
        let total_score: f64 = top_items.iter().map(|(score, _)| score).sum();
        if total_score <= 0.0 {
            return Ok(top_items[0].1.clone());
        }

        use rand::Rng;
        let mut rng = rand::rng();
        let mut random_value = rng.random_range(0.0..total_score);

        for (score, item) in top_items {
            random_value -= score;
            if random_value <= 0.0 {
                return Ok((*item).clone());
            }
        }

        Ok(top_items[0].1.clone())
    }

    /// 获取渠道信息（带缓存）
    async fn get_channel(&self, channel_id: &str) -> Result<ChannelInfo, ProxyError> {
        // 1. 检查缓存
        if let Some(channel) = self.cache.get_channel(channel_id).await {
            return Ok(channel);
        }

        // 2. 缓存未命中，查询数据库
        let result = sqlx::query_as::<_, (String, String, String, String, String, String)>(
            "SELECT id, name, api_keys, endpoints, models, custom_headers FROM channels WHERE id = ? AND enabled = 1"
        )
        .bind(channel_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ProxyError::DatabaseError(e.to_string()))?;

        let (id, name, api_keys_str, endpoints_str, models_str, custom_headers_str) =
            result.ok_or_else(|| ProxyError::ChannelNotFound("渠道不存在或已禁用".to_string()))?;

        let api_keys: Vec<String> = serde_json::from_str(&api_keys_str).unwrap_or_default();
        let endpoints: Vec<EndpointConfig> =
            serde_json::from_str(&endpoints_str).unwrap_or_default();
        let models = parse_models(&models_str);
        let custom_headers: Vec<CustomHeader> = serde_json::from_str(&custom_headers_str).unwrap_or_default();

        let channel = ChannelInfo {
            id,
            name,
            api_keys,
            endpoints,
            models,
            custom_headers,
        };

        // 3. 写入缓存
        self.cache.set_channel(channel.clone()).await;

        Ok(channel)
    }

    /// 按模型查找渠道（支持排除和端点过滤）
    async fn find_channel_by_model(
        &self,
        model: &str,
        exclude_ids: &[String],
        endpoint_filter: impl Fn(&ChannelInfo) -> bool,
    ) -> Result<ChannelInfo, ProxyError> {
        let channels = sqlx::query_as::<_, (String, String, String, String, String, String)>(
            "SELECT id, name, api_keys, endpoints, models, custom_headers FROM channels WHERE enabled = 1",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ProxyError::DatabaseError(e.to_string()))?;

        for (id, name, api_keys_str, endpoints_str, models_str, custom_headers_str) in channels {
            if exclude_ids.contains(&id) {
                continue;
            }

            let models = parse_models(&models_str);
            if !models.iter().any(|m| m == model) {
                continue;
            }

            let api_keys: Vec<String> = serde_json::from_str(&api_keys_str).unwrap_or_default();
            let endpoints: Vec<EndpointConfig> =
                serde_json::from_str(&endpoints_str).unwrap_or_default();

            if endpoints.is_empty() {
                continue;
            }

            let custom_headers: Vec<CustomHeader> = serde_json::from_str(&custom_headers_str).unwrap_or_default();

            if !endpoint_filter(&ChannelInfo {
                id: id.clone(),
                name: name.clone(),
                api_keys: api_keys.clone(),
                endpoints: endpoints.clone(),
                models: models.clone(),
                custom_headers: custom_headers.clone(),
            }) {
                continue;
            }

            return Ok(ChannelInfo {
                id,
                name,
                api_keys,
                endpoints,
                models,
                custom_headers,
            });
        }

        Err(ProxyError::NoAvailableChannel("没有可用渠道".to_string()))
    }

    /// 应用模型映射（模型映射已移至分组层，此处直接返回原始模型名）
    fn apply_model_mapping(&self, _channel: &ChannelInfo, model: &str) -> String {
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

impl ChannelInfo {
    /// 查找指定类型的端点
    pub fn find_endpoint(&self, endpoint_type: &EndpointType) -> Option<EndpointConfig> {
        self.endpoints
            .iter()
            .find(|e| e.endpoint_type == *endpoint_type)
            .cloned()
    }
}

/// 获取入站转换器（静态引用，避免堆分配）
pub fn get_inbound(endpoint_type: &EndpointType) -> &'static dyn Inbound {
    match endpoint_type {
        EndpointType::OpenAiChat => &crate::protocol::openai_chat::OpenAiChatInbound,
        EndpointType::OpenAiResponse => &crate::protocol::openai_responses::OpenAiResponsesInbound,
        EndpointType::Anthropic => &crate::protocol::anthropic::AnthropicInbound,
        _ => &crate::protocol::openai_chat::OpenAiChatInbound,
    }
}

/// 获取出站转换器（静态引用，避免堆分配）
pub fn get_outbound(endpoint_type: &EndpointType) -> &'static dyn Outbound {
    match endpoint_type {
        EndpointType::OpenAiChat => &crate::protocol::openai_chat::OpenAiChatOutbound,
        EndpointType::OpenAiResponse => &crate::protocol::openai_responses::OpenAiResponsesOutbound,
        EndpointType::Anthropic => &crate::protocol::anthropic::AnthropicOutbound,
        _ => &crate::protocol::openai_chat::OpenAiChatOutbound,
    }
}

/// 从响应体提取 usage 数据
pub fn extract_usage(body: &serde_json::Value, endpoint_type: &EndpointType) -> (i32, i32, i32, i32) {
    let usage = &body["usage"];
    match endpoint_type {
        EndpointType::OpenAiChat | EndpointType::OpenAiResponse => {
            let input = usage["prompt_tokens"].as_i64().unwrap_or(0) as i32;
            let output = usage["completion_tokens"].as_i64().unwrap_or(0) as i32;
            let cache_read = usage["prompt_tokens_details"]["cached_tokens"]
                .as_i64()
                .unwrap_or(0) as i32;
            (input, output, cache_read, 0)
        }
        EndpointType::Anthropic => {
            let input = usage["input_tokens"].as_i64().unwrap_or(0) as i32;
            let output = usage["output_tokens"].as_i64().unwrap_or(0) as i32;
            let cache_read = usage["cache_read_input_tokens"].as_i64().unwrap_or(0) as i32;
            let cache_creation = usage["cache_creation_input_tokens"]
                .as_i64()
                .unwrap_or(0) as i32;
            (input, output, cache_read, cache_creation)
        }
        _ => (0, 0, 0, 0),
    }
}

/// 选择渠道（支持重试排除）
async fn select_channel_for_proxy(
    state: &ProxyState,
    headers: &HeaderMap,
    body: &serde_json::Value,
    client_endpoint: &EndpointType,
    exclude_ids: &[String],
) -> Result<SelectionResult, ProxyError> {
    let model = body["model"].as_str().unwrap_or("unknown");
    let session_hash = headers
        .get("x-session-hash")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .or_else(|| body["session_hash"].as_str().map(|s| s.to_string()));

    tracing::debug!(
        "选择渠道: model={}, endpoint={}, excluded={:?}",
        model,
        client_endpoint.as_str(),
        exclude_ids
    );

    match state
        .select_channel_with_exclude(
            model,
            client_endpoint.clone(),
            session_hash.as_deref(),
            exclude_ids,
        )
        .await
    {
        Ok(sel) => {
            tracing::debug!(
                "选中渠道: channel={} ({}), target_model={}, url={}{}",
                sel.channel.name,
                sel.channel.id,
                sel.target_model,
                sel.endpoint.base_url,
                sel.endpoint.endpoint_type.path()
            );
            Ok(sel)
        }
        Err(e) => {
            tracing::warn!("精确端点匹配失败: {}, 尝试跨协议匹配", e);
            let result = state
                .select_channel_for_model_with_exclude(model, session_hash.as_deref(), exclude_ids)
                .await;
            if let Ok(ref sel) = result {
                tracing::debug!(
                    "跨协议选中: channel={} ({}), endpoint={}",
                    sel.channel.name,
                    sel.channel.id,
                    sel.endpoint.endpoint_type.as_str()
                );
            }
            result
        }
    }
}

/// 准备好的代理请求
struct PreparedProxyRequest {
    body: Vec<u8>,
    headers: reqwest::header::HeaderMap,
    url: String,
    upstream_endpoint: EndpointType,
    needs_conversion: bool,
    channel_id: String,
    model: String,
    target_model: String,
}

/// 准备代理请求（共享逻辑）
async fn prepare_proxy_request(
    state: &ProxyState,
    headers: &HeaderMap,
    body: &serde_json::Value,
    client_endpoint: &EndpointType,
    selection: &SelectionResult,
) -> Result<PreparedProxyRequest, ProxyError> {
    let model = body["model"].as_str().unwrap_or("unknown").to_string();
    let upstream_endpoint = selection.endpoint.endpoint_type.clone();
    let needs_conversion = client_endpoint != &upstream_endpoint;

    let request_body = if needs_conversion {
        let inbound = get_inbound(client_endpoint);
        let outbound = get_outbound(&upstream_endpoint);
        let body_bytes = serde_json::to_vec(body)
            .map_err(|e| ProxyError::TransformError(e.to_string()))?;
        let llm_request = inbound.transform_request(&body_bytes, headers).await
            .map_err(|e| ProxyError::TransformError(e.to_string()))?;
        outbound.transform_request(&llm_request)
            .map_err(|e| ProxyError::TransformError(e.to_string()))?
    } else {
        serde_json::to_vec(body).map_err(|e| ProxyError::TransformError(e.to_string()))?
    };

    let api_key = state.select_api_key(&selection.channel);
    let mut reqwest_headers = reqwest::header::HeaderMap::new();
    reqwest_headers.insert("Content-Type", "application/json".parse().unwrap());

    if needs_conversion {
        get_outbound(&upstream_endpoint).set_auth_header(&mut reqwest_headers, &api_key);
    } else {
        match client_endpoint {
            EndpointType::Anthropic => {
                reqwest_headers.insert("x-api-key", api_key.parse().unwrap());
                reqwest_headers.insert("anthropic-version", "2023-06-01".parse().unwrap());
            }
            _ => {
                reqwest_headers.insert("Authorization", format!("Bearer {}", api_key).parse().unwrap());
            }
        }
    }

    let url = format!("{}{}", selection.endpoint.base_url, upstream_endpoint.path());

    for header in &selection.channel.custom_headers {
        if let Ok(name) = reqwest::header::HeaderName::from_bytes(header.key.as_bytes()) {
            if let Ok(value) = header.value.parse() {
                reqwest_headers.insert(name, value);
            }
        }
    }

    Ok(PreparedProxyRequest {
        body: request_body,
        headers: reqwest_headers,
        url,
        upstream_endpoint,
        needs_conversion,
        channel_id: selection.channel.id.clone(),
        model,
        target_model: selection.target_model.clone(),
    })
}

/// 执行单次代理请求
async fn execute_proxy_request(
    state: &ProxyState,
    api_key_id: Option<&str>,
    headers: &HeaderMap,
    body: &serde_json::Value,
    client_endpoint: &EndpointType,
    selection: &SelectionResult,
) -> Result<ProxySuccess, ProxyError> {
    let prepared = prepare_proxy_request(state, headers, body, client_endpoint, selection).await?;
    let start_time = std::time::Instant::now();

    let response = state.http_client.post(&prepared.url)
        .headers(prepared.headers)
        .body(prepared.body)
        .send()
        .await
        .map_err(|e| ProxyError::RequestError(e.to_string()))?;

    let latency_ms = start_time.elapsed().as_millis() as i64;
    let status = response.status();
    let response_body = response.text().await.unwrap_or_default();

    let body_value: serde_json::Value = serde_json::from_str(&response_body).unwrap_or_default();
    let request_content = serde_json::to_string(&body).ok();
    let status_u16 = status.as_u16();

    // 记录统计
    {
        let (input_tokens, output_tokens, cache_read, cache_creation) =
            if (200..400).contains(&status_u16) {
                extract_usage(&body_value, &prepared.upstream_endpoint)
            } else {
                (0, 0, 0, 0)
            };
        let record = crate::stats::recorder::RequestRecord {
            api_key_id: api_key_id.map(|s| s.to_string()),
            channel_id: Some(prepared.channel_id.clone()),
            group_id: None,
            requested_model: prepared.model.clone(),
            actual_model: Some(prepared.target_model.clone()),
            input_tokens, output_tokens, cache_read_tokens: cache_read, cache_creation_tokens: cache_creation,
            cost: None,
            latency_ms: Some(latency_ms as i32),
            status_code: Some(status_u16 as i32),
            error_message: if !status.is_success() { Some(response_body.clone()) } else { None },
            endpoint_type: Some(prepared.upstream_endpoint.as_str().to_string()),
            request_type: if prepared.needs_conversion { "conversion".to_string() } else { "passthrough".to_string() },
            request_content,
            response_content: if status.is_success() { Some(response_body.clone()) } else { None },
            is_stream: false,
        };
        let _ = state.stats_recorder.record_request(record).await;
    }

    if !status.is_success() {
        tracing::warn!(
            "Upstream error: channel={}, status={}, body={}",
            prepared.channel_id,
            status,
            &response_body[..response_body.len().min(300)]
        );
        return Err(ProxyError::UpstreamError { status, body: response_body });
    }

    state.lb_state.record_success(&prepared.channel_id, latency_ms as f64).await;

    let final_body = if prepared.needs_conversion {
        let inbound = get_inbound(client_endpoint);
        let outbound = get_outbound(&prepared.upstream_endpoint);
        let llm_response = outbound.transform_response(response_body.as_bytes(), status.as_u16()).await
            .map_err(|e| ProxyError::TransformError(e.to_string()))?;
        inbound.transform_response(&llm_response)
            .map_err(|e| ProxyError::TransformError(e.to_string()))?
    } else {
        response_body.into_bytes()
    };

    Ok(ProxySuccess { status, body: final_body })
}

/// 非流式代理请求（支持重试和排队）
pub async fn proxy_request(
    state: &ProxyState,
    api_key_id: Option<&str>,
    headers: &HeaderMap,
    body: &serde_json::Value,
    client_endpoint: &EndpointType,
) -> Result<ProxySuccess, ProxyError> {
    // 排队控制
    let _permit = if let Some(queue) = &state.queue {
        Some(queue.acquire().await.map_err(|e| {
            ProxyError::RequestError(format!("排队失败: {}", e))
        })?)
    } else {
        None
    };

    let max_retries = 3;
    let mut exclude_ids = Vec::new();
    let mut last_error = None;

    for attempt in 0..max_retries {
        let selection = select_channel_for_proxy(state, headers, body, client_endpoint, &exclude_ids).await?;
        let channel_id = selection.channel.id.clone();

        match execute_proxy_request(state, api_key_id, headers, body, client_endpoint, &selection).await {
            Ok(result) => return Ok(result),
            Err(ProxyError::UpstreamError { status, body }) => {
                tracing::warn!(
                    "请求失败(第{}次), channel={}, status={}, 排除后重试",
                    attempt + 1,
                    channel_id,
                    status
                );
                state
                    .lb_state
                    .record_failure(&channel_id, status.is_server_error())
                    .await;
                exclude_ids.push(channel_id);
                last_error = Some(ProxyError::UpstreamError { status, body });
            }
            Err(e) => return Err(e),
        }
    }

    tracing::error!("所有重试耗尽, model={}", body["model"].as_str().unwrap_or("unknown"));
    Err(last_error.unwrap_or_else(|| ProxyError::NoAvailableChannel("所有渠道都不可用".to_string())))
}

/// 执行单次流式代理请求
async fn execute_proxy_stream(
    state: &ProxyState,
    api_key_id: Option<&str>,
    headers: &HeaderMap,
    body: &serde_json::Value,
    client_endpoint: &EndpointType,
    selection: &SelectionResult,
) -> Result<
    (
        StatusCode,
        std::pin::Pin<
            Box<
                dyn futures::Stream<Item = Result<Bytes, std::convert::Infallible>>
                    + Send
                    + 'static,
            >,
        >,
        String,
    ),
    ProxyError,
> {
    let prepared = prepare_proxy_request(state, headers, body, client_endpoint, selection).await?;
    let start_time = std::time::Instant::now();

    let response = state.http_client.post(&prepared.url)
        .headers(prepared.headers)
        .body(prepared.body)
        .send()
        .await
        .map_err(|e| ProxyError::RequestError(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(ProxyError::UpstreamError { status, body });
    }

    state.lb_state.record_success(&prepared.channel_id, start_time.elapsed().as_millis() as f64).await;

    let upstream_stream = response.bytes_stream();
    use futures::StreamExt;

    let state_clone = state.clone();
    let channel_id_clone = prepared.channel_id.clone();
    let model_clone = prepared.model.clone();
    let target_model_clone = prepared.target_model.clone();
    let upstream_endpoint_clone = prepared.upstream_endpoint.clone();
    let client_endpoint_clone = client_endpoint.clone();
    let needs_conversion = prepared.needs_conversion;
    let api_key_id_clone = api_key_id.map(|s| s.to_string());
    let request_content_clone = serde_json::to_string(&body).ok();

    let response_stream = async_stream::stream! {
        let mut stream = std::pin::pin!(upstream_stream);
        let mut last_usage: Option<serde_json::Value> = None;
        let mut buffer = Vec::new();
        let mut collected_text = String::new();

        if needs_conversion {
            let inbound = get_inbound(&client_endpoint_clone);
            let outbound = get_outbound(&upstream_endpoint_clone);

            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        buffer.extend_from_slice(&bytes);

                        // 处理完整的 SSE 事件（以 \n\n 分隔）
                        while let Some(event_end) = find_sse_boundary(&buffer) {
                            let event_bytes = buffer[..event_end].to_vec();
                            buffer = buffer[event_end..].to_vec();

                            // 跳过空事件
                            if event_bytes.iter().all(|b| *b == b'\n' || *b == b'\r') {
                                continue;
                            }

                            // 尝试提取 usage
                            if let Ok(text) = std::str::from_utf8(&event_bytes) {
                                if let Some(usage) = extract_usage_from_sse(text, &upstream_endpoint_clone) {
                                    last_usage = Some(usage);
                                }
                            }

                            // 转换事件
                            match outbound.transform_stream_event(&event_bytes) {
                                Ok(Some(llm_stream)) => {
                                    if let Some(choice) = llm_stream.first_choice() {
                                        if let Some(crate::protocol::model::Content::Text(t)) = &choice.delta.content {
                                            if !t.is_empty() {
                                                collected_text.push_str(t);
                                            }
                                        }
                                    }
                                    match inbound.transform_stream_event(&llm_stream) {
                                        Ok(converted) => {
                                            yield Ok::<_, std::convert::Infallible>(Bytes::from(converted));
                                        }
                                        Err(e) => {
                                            tracing::error!("Stream inbound conversion error: {}", e);
                                        }
                                    }
                                }
                                Ok(None) => {
                                    // 跳过不需要转换的事件（如 [DONE]）
                                }
                                Err(e) => {
                                    tracing::error!("Stream outbound conversion error: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Upstream stream error: {}", e);
                        break;
                    }
                }
            }

            // 处理缓冲区中剩余的数据
            if !buffer.is_empty() && !buffer.iter().all(|b| *b == b'\n' || *b == b'\r') {
                if let Ok(text) = std::str::from_utf8(&buffer) {
                    if let Some(usage) = extract_usage_from_sse(text, &upstream_endpoint_clone) {
                        last_usage = Some(usage);
                    }
                }
                match outbound.transform_stream_event(&buffer) {
                    Ok(Some(llm_stream)) => {
                        if let Ok(converted) = inbound.transform_stream_event(&llm_stream) {
                            yield Ok(Bytes::from(converted));
                        }
                    }
                    _ => {}
                }
            }
        } else {
            // 直通模式
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        if let Ok(text) = std::str::from_utf8(&bytes) {
                            if let Some(usage) = extract_usage_from_sse(text, &upstream_endpoint_clone) {
                                last_usage = Some(usage);
                            }
                            collect_sse_text(text, &upstream_endpoint_clone, &mut collected_text);
                        }
                        yield Ok::<_, std::convert::Infallible>(bytes);
                    }
                    Err(e) => {
                        tracing::error!("Stream error: {}", e);
                        break;
                    }
                }
            }
        }

        // 流结束后记录统计
        let latency_ms = start_time.elapsed().as_millis() as i64;
        let (input_tokens, output_tokens, cache_read, cache_creation) =
            last_usage
                .map(|u| extract_usage(&u, &upstream_endpoint_clone))
                .unwrap_or((0, 0, 0, 0));

        let record = crate::stats::recorder::RequestRecord {
            api_key_id: api_key_id_clone,
            channel_id: Some(channel_id_clone),
            group_id: None,
            requested_model: model_clone,
            actual_model: Some(target_model_clone),
            input_tokens,
            output_tokens,
            cache_read_tokens: cache_read,
            cache_creation_tokens: cache_creation,
            cost: None,
            latency_ms: Some(latency_ms as i32),
            status_code: Some(200),
            error_message: None,
            endpoint_type: Some(client_endpoint_clone.as_str().to_string()),
            request_type: if needs_conversion { "conversion".to_string() } else { "passthrough".to_string() },
            request_content: request_content_clone,
            response_content: if collected_text.is_empty() { None } else { Some(collected_text) },
            is_stream: true,
        };

        let _ = state_clone.stats_recorder.record_request(record).await;
    };

    Ok((
        StatusCode::OK,
        Box::pin(response_stream),
        "text/event-stream".to_string(),
    ))
}

/// 流式代理请求（支持重试和排队）
pub async fn proxy_stream(
    state: &ProxyState,
    api_key_id: Option<&str>,
    headers: &HeaderMap,
    body: &serde_json::Value,
    client_endpoint: &EndpointType,
) -> Result<
    (
        StatusCode,
        std::pin::Pin<
            Box<
                dyn futures::Stream<Item = Result<Bytes, std::convert::Infallible>>
                    + Send
                    + 'static,
            >,
        >,
        String,
    ),
    ProxyError,
> {
    // 排队控制
    let permit = if let Some(queue) = &state.queue {
        Some(queue.acquire().await.map_err(|e| {
            ProxyError::RequestError(format!("排队失败: {}", e))
        })?)
    } else {
        None
    };

    let max_retries = 3;
    let mut exclude_ids = Vec::new();
    let mut last_error = None;

    for attempt in 0..max_retries {
        let selection =
            select_channel_for_proxy(state, headers, body, client_endpoint, &exclude_ids).await?;
        let channel_id = selection.channel.id.clone();

        match execute_proxy_stream(state, api_key_id, headers, body, client_endpoint, &selection).await {
            Ok(result) => {
                // 流式连接成功，释放 permit（流式会持续很长时间，不应占用队列位置）
                drop(permit);
                return Ok(result);
            }
            Err(ProxyError::UpstreamError { status, body }) => {
                tracing::warn!(
                    "流式请求失败(第{}次), channel={}, status={}, 排除后重试",
                    attempt + 1,
                    channel_id,
                    status
                );
                state
                    .lb_state
                    .record_failure(&channel_id, status.is_server_error())
                    .await;
                exclude_ids.push(channel_id);
                last_error = Some(ProxyError::UpstreamError { status, body });
            }
            Err(e) => return Err(e),
        }
    }

    tracing::error!("流式重试耗尽, model={}", body["model"].as_str().unwrap_or("unknown"));
    Err(last_error.unwrap_or_else(|| {
        ProxyError::NoAvailableChannel("所有渠道都不可用".to_string())
    }))
}

/// 错误格式类型
pub enum ErrorFormat {
    /// OpenAI 格式: {"error": {"message": ..., "type": ...}}
    OpenAi,
    /// Anthropic 格式: {"type": "error", "error": {"type": ..., "message": ...}}
    Anthropic,
}

/// 统一代理请求入口（供各 handler 调用）
pub async fn handle_proxy_request(
    state: &ProxyState,
    auth: crate::api::middleware::ApiKeyAuth,
    headers: HeaderMap,
    body: serde_json::Value,
    client_endpoint: &crate::api::handlers::admin::channels::EndpointType,
    error_format: &ErrorFormat,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let is_stream = body["stream"].as_bool().unwrap_or(false);
    let api_key_id = Some(auth.key_id.as_str());

    if is_stream {
        match proxy_stream(state, api_key_id, &headers, &body, client_endpoint).await {
            Ok((status, stream, content_type)) => axum::response::Response::builder()
                .status(status)
                .header("Content-Type", content_type)
                .header("Cache-Control", "no-cache")
                .header("Connection", "keep-alive")
                .body(axum::body::Body::from_stream(stream))
                .unwrap()
                .into_response(),
            Err(e) => format_proxy_error(e, error_format),
        }
    } else {
        match proxy_request(state, api_key_id, &headers, &body, client_endpoint).await {
            Ok(result) => axum::response::Response::builder()
                .status(result.status)
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(result.body))
                .unwrap()
                .into_response(),
            Err(e) => format_proxy_error(e, error_format),
        }
    }
}

/// 格式化代理错误为 HTTP 响应
pub fn format_proxy_error(e: ProxyError, format: &ErrorFormat) -> axum::response::Response {
    use axum::response::IntoResponse;

    match (e, format) {
        (ProxyError::NoAvailableChannel(msg), ErrorFormat::OpenAi) => (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(serde_json::json!({
                "error": { "message": msg, "type": "server_error" }
            })),
        )
            .into_response(),
        (ProxyError::NoAvailableChannel(msg), ErrorFormat::Anthropic) => (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(serde_json::json!({
                "type": "error",
                "error": { "type": "api_error", "message": msg }
            })),
        )
            .into_response(),
        (ProxyError::UpstreamError { status, body }, ErrorFormat::OpenAi) => (
            status,
            axum::Json(serde_json::json!({
                "error": { "message": body, "type": "server_error" }
            })),
        )
            .into_response(),
        (ProxyError::UpstreamError { status, body }, ErrorFormat::Anthropic) => (
            status,
            axum::Json(serde_json::json!({
                "type": "error",
                "error": { "type": "api_error", "message": body }
            })),
        )
            .into_response(),
        (e, ErrorFormat::OpenAi) => (
            StatusCode::BAD_GATEWAY,
            axum::Json(serde_json::json!({
                "error": { "message": e.to_string(), "type": "server_error" }
            })),
        )
            .into_response(),
        (e, ErrorFormat::Anthropic) => (
            StatusCode::BAD_GATEWAY,
            axum::Json(serde_json::json!({
                "type": "error",
                "error": { "type": "api_error", "message": e.to_string() }
            })),
        )
            .into_response(),
    }
}

/// 查找 SSE 事件边界（\n\n 或 \r\n\r\n）
fn find_sse_boundary(buffer: &[u8]) -> Option<usize> {
    for i in 0..buffer.len() {
        if i + 1 < buffer.len() && buffer[i] == b'\n' && buffer[i + 1] == b'\n' {
            return Some(i + 2);
        }
        if i + 3 < buffer.len()
            && buffer[i] == b'\r'
            && buffer[i + 1] == b'\n'
            && buffer[i + 2] == b'\r'
            && buffer[i + 3] == b'\n'
        {
            return Some(i + 4);
        }
    }
    None
}

/// 从 SSE 事件中提取 usage 数据
fn extract_usage_from_sse(text: &str, endpoint_type: &EndpointType) -> Option<serde_json::Value> {
    match endpoint_type {
        EndpointType::OpenAiChat | EndpointType::OpenAiResponse => {
            // OpenAI 格式: data: {"usage": {...}}
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                        if parsed.get("usage").is_some() {
                            return Some(parsed);
                        }
                    }
                }
            }
            None
        }
        EndpointType::Anthropic => {
            // Anthropic 格式: event: message_delta\ndata: {"usage": {...}}
            let mut event_type = "";
            let mut data = "";
            for line in text.lines() {
                if line.starts_with("event: ") {
                    event_type = &line[7..];
                } else if line.starts_with("data: ") {
                    data = &line[6..];
                }
            }
            if event_type == "message_delta" {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                    if parsed.get("usage").is_some() {
                        return Some(parsed);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// 从直通模式的 SSE 文本中提取内容文本
fn collect_sse_text(text: &str, endpoint_type: &EndpointType, output: &mut String) {
    match endpoint_type {
        EndpointType::OpenAiChat | EndpointType::OpenAiResponse => {
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" { continue; }
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                            if !content.is_empty() {
                                output.push_str(content);
                            }
                        }
                    }
                }
            }
        }
        EndpointType::Anthropic => {
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                        if parsed["type"] == "content_block_delta" {
                            if let Some(t) = parsed["delta"]["text"].as_str() {
                                output.push_str(t);
                            }
                        }
                    }
                }
            }
        }
        _ => {}
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

/// 解析 models 字段，兼容旧格式 {"available_models":[...],"model_maps":{}} 和新格式 ["m1","m2"]
fn parse_models(models_str: &str) -> Vec<String> {
    let value: serde_json::Value = serde_json::from_str(models_str).unwrap_or_default();

    // 新格式：直接是数组
    if let Some(arr) = value.as_array() {
        return arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
    }

    // 旧格式：从 available_models 字段提取
    if let Some(available) = value["available_models"].as_array() {
        return available.iter().filter_map(|v| v.as_str().map(String::from)).collect();
    }

    Vec::new()
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

    #[error("上游错误: {status}")]
    UpstreamError {
        status: StatusCode,
        body: String,
    },
}
