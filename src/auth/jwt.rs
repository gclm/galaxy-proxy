use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

/// JWT Claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub exp: usize,
    pub iat: usize,
}

/// 构建统一的 JWT Validation（HS256 only）
pub fn jwt_validation() -> Validation {
    Validation::new(Algorithm::HS256)
}

/// 解码并验证 JWT token
pub fn decode_jwt(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &jwt_validation(),
    )?;
    Ok(token_data.claims)
}

/// JWT 服务
#[derive(Clone)]
pub struct JwtService {
    secret: String,
    expiry_hours: u64,
}

impl JwtService {
    /// 创建 JWT 服务
    pub fn new(secret: &str, expiry_hours: u64) -> Self {
        Self {
            secret: secret.to_string(),
            expiry_hours,
        }
    }

    /// 生成 Token
    pub fn generate_token(
        &self,
        user_id: &str,
        username: &str,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let now = Utc::now();
        let expires_at = now + Duration::hours(self.expiry_hours as i64);

        let claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            exp: expires_at.timestamp() as usize,
            iat: now.timestamp() as usize,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
    }

    /// 验证 Token
    #[allow(dead_code)]
    pub fn verify_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        decode_jwt(token, &self.secret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_verify() {
        let service = JwtService::new("test_secret", 24);
        let token = service.generate_token("1", "admin").unwrap();
        let claims = service.verify_token(&token).unwrap();

        assert_eq!(claims.sub, "1");
        assert_eq!(claims.username, "admin");
    }

    #[test]
    fn test_invalid_token() {
        let service = JwtService::new("test_secret", 24);
        let result = service.verify_token("invalid_token");
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_secret() {
        let service1 = JwtService::new("secret1", 24);
        let service2 = JwtService::new("secret2", 24);

        let token = service1.generate_token("1", "admin").unwrap();
        let result = service2.verify_token(&token);
        assert!(result.is_err());
    }
}
