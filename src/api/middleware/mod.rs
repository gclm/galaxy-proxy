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

/// JWT Claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub exp: usize,
    pub iat: usize,
}

/// 从请求中提取 Claims
pub struct AuthClaims(pub Claims);

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for AuthClaims
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 提取 Authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| (StatusCode::UNAUTHORIZED, "缺少认证令牌".to_string()))?;

        // 从 extensions 获取 JWT secret
        let jwt_secret = parts
            .extensions
            .get::<String>()
            .ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, "JWT 配置缺失".to_string()))?;

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
