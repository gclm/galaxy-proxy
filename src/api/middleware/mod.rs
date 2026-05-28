use std::future::Future;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    RequestPartsExt,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// API Key 缓存
#[derive(Clone)]
pub struct ApiKeyCache {
    keys: Arc<RwLock<HashMap<String, ApiKeyEntry>>>,
}

#[derive(Clone)]
struct ApiKeyEntry {
    id: String,
    name: String,
    enabled: bool,
}

impl ApiKeyCache {
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取缓存的 API Key
    async fn get(&self, key: &str) -> Option<(String, String, bool)> {
        let cache = self.keys.read().await;
        cache.get(key).map(|e| (e.id.clone(), e.name.clone(), e.enabled))
    }

    /// 设置 API Key 缓存
    async fn set(&self, key: String, id: String, name: String, enabled: bool) {
        let mut cache = self.keys.write().await;
        // 限制缓存大小
        if cache.len() >= 1000 {
            if let Some(oldest_key) = cache.keys().next().cloned() {
                cache.remove(&oldest_key);
            }
        }
        cache.insert(key, ApiKeyEntry { id, name, enabled });
    }

    /// 清除缓存
    pub async fn invalidate(&self, key: &str) {
        let mut cache = self.keys.write().await;
        cache.remove(key);
    }

    /// 清除所有缓存
    pub async fn invalidate_all(&self) {
        let mut cache = self.keys.write().await;
        cache.clear();
    }
}

/// JWT Claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub exp: usize,
    pub iat: usize,
}

/// 从请求中提取 Claims（管理 API 认证）
pub struct AuthClaims(pub Claims);

impl<S: Send + Sync> FromRequestParts<S> for AuthClaims {
    type Rejection = (StatusCode, String);

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            // 提取 Authorization header
            let TypedHeader(Authorization(bearer)) = parts
                .extract::<TypedHeader<Authorization<Bearer>>>()
                .await
                .map_err(|_| (StatusCode::UNAUTHORIZED, "缺少认证令牌".to_string()))?;

            // 从 extensions 获取 JWT secret
            let jwt_secret = parts.extensions.get::<String>().ok_or_else(|| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "JWT 配置缺失".to_string(),
                )
            })?;

            // 验证 Token
            let token_data = jsonwebtoken::decode::<Claims>(
                bearer.token(),
                &jsonwebtoken::DecodingKey::from_secret(jwt_secret.as_bytes()),
                &jsonwebtoken::Validation::default(),
            )
            .map_err(|_| (StatusCode::UNAUTHORIZED, "无效的认证令牌".to_string()))?;

            Ok(AuthClaims(token_data.claims))
        }
    }
}

/// API Key 认证结果（代理 API 认证）
pub struct ApiKeyAuth {
    pub key_id: String,
    pub key_name: String,
}

impl<S: Send + Sync> FromRequestParts<S> for ApiKeyAuth {
    type Rejection = (StatusCode, axum::Json<serde_json::Value>);

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            // 优先从 Authorization: Bearer 提取，回退到 x-api-key（Anthropic 兼容）
            let api_key = match parts
                .extract::<TypedHeader<Authorization<Bearer>>>()
                .await
            {
                Ok(TypedHeader(Authorization(bearer))) => bearer.token().to_string(),
                Err(_) => parts
                    .headers
                    .get("x-api-key")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
            };

            if api_key.is_empty() {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    axum::Json(serde_json::json!({
                        "error": { "message": "缺少 API Key", "type": "authentication_error" }
                    })),
                ));
            }

            // 1. 检查缓存
            if let Some(cache) = parts.extensions.get::<ApiKeyCache>() {
                if let Some((id, name, enabled)) = cache.get(&api_key).await {
                    if !enabled {
                        return Err((
                            StatusCode::FORBIDDEN,
                            axum::Json(serde_json::json!({
                                "error": { "message": "API Key 已禁用", "type": "authentication_error" }
                            })),
                        ));
                    }
                    return Ok(ApiKeyAuth { key_id: id, key_name: name });
                }
            }

            // 2. 缓存未命中，查询数据库
            let pool = parts.extensions.get::<SqlitePool>().ok_or_else(|| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(serde_json::json!({
                        "error": { "message": "数据库配置缺失", "type": "server_error" }
                    })),
                )
            })?;

            let result = sqlx::query_as::<_, (String, String, bool)>(
                "SELECT id, name, enabled FROM api_keys WHERE api_key = ?",
            )
            .bind(&api_key)
            .fetch_optional(pool)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(serde_json::json!({
                        "error": { "message": "数据库查询失败", "type": "server_error" }
                    })),
                )
            })?;

            match result {
                Some((id, name, enabled)) => {
                    if let Some(cache) = parts.extensions.get::<ApiKeyCache>() {
                        cache.set(api_key, id.clone(), name.clone(), enabled).await;
                    }
                    if !enabled {
                        return Err((
                            StatusCode::FORBIDDEN,
                            axum::Json(serde_json::json!({
                                "error": { "message": "API Key 已禁用", "type": "authentication_error" }
                            })),
                        ));
                    }
                    Ok(ApiKeyAuth { key_id: id, key_name: name })
                }
                None => Err((
                    StatusCode::UNAUTHORIZED,
                    axum::Json(serde_json::json!({
                        "error": { "message": "无效的 API Key", "type": "authentication_error" }
                    })),
                )),
            }
        }
    }
}
