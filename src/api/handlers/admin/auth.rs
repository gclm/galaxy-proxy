use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::auth::{JwtService, PasswordService};

/// 认证状态
#[derive(Clone)]
pub struct AuthState {
    pub pool: SqlitePool,
    pub jwt_service: JwtService,
}

/// 初始化请求
#[derive(Deserialize)]
pub struct SetupRequest {
    pub username: String,
    pub password: String,
}

/// 登录请求
#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 修改密码请求
#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

/// 认证响应
#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub expires_in: u64,
}

/// 用户信息响应
#[derive(Serialize)]
pub struct UserInfoResponse {
    pub id: i64,
    pub username: String,
}

/// 初始化管理员
pub async fn setup(
    State(state): State<AuthState>,
    Json(req): Json<SetupRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    // 检查是否已有用户
    let count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if count > 0 {
        return Err((StatusCode::CONFLICT, "管理员已存在".to_string()));
    }

    // 验证输入
    if req.username.len() < 3 {
        return Err((StatusCode::BAD_REQUEST, "用户名至少 3 个字符".to_string()));
    }
    if req.password.len() < 8 {
        return Err((StatusCode::BAD_REQUEST, "密码至少 8 个字符".to_string()));
    }

    // 哈希密码
    let password_hash = PasswordService::hash_password(&req.password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 插入用户
    let user_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (username, password_hash) VALUES (?, ?) RETURNING id"
    )
    .bind(&req.username)
    .bind(&password_hash)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 生成 Token
    let token = state.jwt_service.generate_token(&user_id.to_string(), &req.username)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AuthResponse {
        token,
        expires_in: 86400, // 24 小时
    }))
}

/// 登录
pub async fn login(
    State(state): State<AuthState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    // 查询用户
    let user = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, username, password_hash FROM users WHERE username = ?"
    )
    .bind(&req.username)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let (user_id, username, password_hash) = user
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "用户名或密码错误".to_string()))?;

    // 验证密码
    let valid = PasswordService::verify_password(&req.password, &password_hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !valid {
        return Err((StatusCode::UNAUTHORIZED, "用户名或密码错误".to_string()));
    }

    // 生成 Token
    let token = state.jwt_service.generate_token(&user_id.to_string(), &username)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AuthResponse {
        token,
        expires_in: 86400,
    }))
}

/// 获取当前用户信息
pub async fn me(
    auth: crate::api::middleware::AuthClaims,
) -> Result<Json<UserInfoResponse>, (StatusCode, String)> {
    let user_id: i64 = auth.0.sub.parse()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "无效的用户 ID".to_string()))?;

    Ok(Json(UserInfoResponse {
        id: user_id,
        username: auth.0.username,
    }))
}

/// 修改密码
pub async fn change_password(
    State(state): State<AuthState>,
    auth: crate::api::middleware::AuthClaims,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let user_id: i64 = auth.0.sub.parse()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "无效的用户 ID".to_string()))?;

    // 查询当前密码
    let password_hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 验证旧密码
    let valid = PasswordService::verify_password(&req.old_password, &password_hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !valid {
        return Err((StatusCode::UNAUTHORIZED, "旧密码错误".to_string()));
    }

    // 验证新密码
    if req.new_password.len() < 8 {
        return Err((StatusCode::BAD_REQUEST, "新密码至少 8 个字符".to_string()));
    }

    // 哈希新密码
    let new_hash = PasswordService::hash_password(&req.new_password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 更新密码
    sqlx::query("UPDATE users SET password_hash = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&new_hash)
        .bind(user_id)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}
