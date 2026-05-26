use std::future::Future;

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
            // 提取 Authorization header
            let TypedHeader(Authorization(bearer)) = parts
                .extract::<TypedHeader<Authorization<Bearer>>>()
                .await
                .map_err(|_| {
                    (
                        StatusCode::UNAUTHORIZED,
                        axum::Json(serde_json::json!({
                            "error": { "message": "缺少 API Key", "type": "authentication_error" }
                        })),
                    )
                })?;

            // 从 extensions 获取数据库连接池
            let pool = parts.extensions.get::<SqlitePool>().ok_or_else(|| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(serde_json::json!({
                        "error": { "message": "数据库配置缺失", "type": "server_error" }
                    })),
                )
            })?;

            // 查询 API Key
            let api_key = bearer.token();
            let result = sqlx::query_as::<_, (String, String, bool)>(
                "SELECT id, name, enabled FROM api_keys WHERE api_key = ?",
            )
            .bind(api_key)
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
                    if !enabled {
                        return Err((
                            StatusCode::FORBIDDEN,
                            axum::Json(serde_json::json!({
                                "error": { "message": "API Key 已禁用", "type": "authentication_error" }
                            })),
                        ));
                    }
                    Ok(ApiKeyAuth {
                        key_id: id,
                        key_name: name,
                    })
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
