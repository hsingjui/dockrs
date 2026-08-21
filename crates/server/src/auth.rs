//! 登录认证：用户名 + 密码 → Argon2id 校验 → 服务端 Session → HttpOnly Cookie。

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use axum::{
    Router,
    extract::State,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tower_sessions::Session;
use utoipa::ToSchema;

use crate::{
    AppState,
    error::{ApiError, ApiJson, ApiResponse},
};

const USER_ID_KEY: &str = "user_id";

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/change-password", post(change_password))
        .route("/auth/me", get(me))
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: i64,
    username: String,
    password_hash: String,
}

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Serialize, ToSchema)]
pub struct UserResponse {
    pub id: i64,
    pub username: String,
}

/// 哈希密码，产出 Argon2id PHC 字符串（含随机盐与默认参数）
fn hash_password(password: &str) -> argon2::password_hash::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)?
        .to_string())
}

fn verify_password(hash: &str, password: &str) -> bool {
    PasswordHash::new(hash).ok().is_some_and(|parsed| {
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
}

#[utoipa::path(
	post,
	path = "/api/auth/login",
	request_body = LoginRequest,
	responses(
		(status = 200, body = UserResponse, description = "登录成功"),
		(status = 401, description = "账号或密码错误"),
	)
)]
async fn login(
    State(state): State<AppState>,
    session: Session,
    ApiJson(req): ApiJson<LoginRequest>,
) -> Result<ApiResponse<UserResponse>, ApiError> {
    let pool = state.pool.clone();
    let user = sqlx::query_as::<_, UserRow>(
        "SELECT id, username, password_hash FROM users WHERE username = ?",
    )
    .bind(&req.username)
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "查询用户失败");
        ApiError::internal("登录失败，请稍后重试")
    })?;

    let Some(user) = user else {
        // 用户不存在时也执行一次哈希，避免通过响应时间枚举用户名
        let _ = hash_password(&req.password);
        return Err(ApiError::unauthorized("账号或密码错误"));
    };

    if !verify_password(&user.password_hash, &req.password) {
        return Err(ApiError::unauthorized("账号或密码错误"));
    }

    // 登录成功后轮换 session id，防止会话固定攻击
    session.cycle_id().await.map_err(|e| {
        tracing::error!(error = %e, "轮换 session 失败");
        ApiError::internal("登录失败，请稍后重试")
    })?;
    session.insert(USER_ID_KEY, user.id).await.map_err(|e| {
        tracing::error!(error = %e, "写入 session 失败");
        ApiError::internal("登录失败，请稍后重试")
    })?;

    tracing::info!(user_id = user.id, username = %user.username, "用户登录");
    Ok(ApiResponse::success(UserResponse {
        id: user.id,
        username: user.username,
    }))
}

#[utoipa::path(
    post,
    path = "/api/auth/change-password",
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "密码修改成功"),
        (status = 400, description = "新密码无效"),
        (status = 401, description = "未登录或当前密码错误"),
    )
)]
async fn change_password(
    State(state): State<AppState>,
    session: Session,
    ApiJson(req): ApiJson<ChangePasswordRequest>,
) -> Result<ApiResponse<serde_json::Value>, ApiError> {
    let pool = state.pool.clone();
    let user_id = require_user(&session).await?;

    if req.new_password.is_empty() {
        return Err(ApiError::bad_request("新密码不能为空"));
    }

    let password_hash =
        sqlx::query_scalar::<_, String>("SELECT password_hash FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "查询用户失败");
                ApiError::internal("修改密码失败，请稍后重试")
            })?
            .ok_or_else(|| ApiError::unauthorized("未登录"))?;

    if !verify_password(&password_hash, &req.current_password) {
        return Err(ApiError::unauthorized("当前密码错误"));
    }

    let new_password_hash = hash_password(&req.new_password).map_err(|e| {
        tracing::error!(error = %e, "生成密码哈希失败");
        ApiError::internal("修改密码失败，请稍后重试")
    })?;

    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(new_password_hash)
        .bind(user_id)
        .execute(&pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "更新密码失败");
            ApiError::internal("修改密码失败，请稍后重试")
        })?;

    tracing::info!(user_id, "用户修改密码");
    Ok(ApiResponse::success(serde_json::json!({})))
}

#[utoipa::path(
	post,
	path = "/api/auth/logout",
	responses((status = 200, description = "登出成功"))
)]
async fn logout(session: Session) -> Result<ApiResponse<serde_json::Value>, ApiError> {
    session.flush().await.map_err(|e| {
        tracing::error!(error = %e, "清空 session 失败");
        ApiError::internal("登出失败，请稍后重试")
    })?;
    Ok(ApiResponse::success(serde_json::json!({})))
}

#[utoipa::path(
	get,
	path = "/api/auth/me",
	responses(
		(status = 200, body = UserResponse),
		(status = 401, description = "未登录"),
	)
)]
async fn me(
    State(state): State<AppState>,
    session: Session,
) -> Result<ApiResponse<UserResponse>, ApiError> {
    let pool = state.pool.clone();
    let user_id = require_user(&session).await?;

    let user =
        sqlx::query_as::<_, UserRow>("SELECT id, username, password_hash FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "查询用户失败");
                ApiError::internal("获取用户信息失败")
            })?;

    user.map(|u| {
        ApiResponse::success(UserResponse {
            id: u.id,
            username: u.username,
        })
    })
    .ok_or_else(|| ApiError::unauthorized("未登录"))
}

pub(crate) async fn require_user(session: &Session) -> Result<i64, ApiError> {
    session
        .get::<i64>(USER_ID_KEY)
        .await
        .map_err(|error| {
            tracing::error!(error = %error, "读取 session 失败");
            ApiError::internal("认证状态读取失败")
        })?
        .ok_or_else(|| ApiError::unauthorized("未登录"))
}

/// 首次启动（用户表为空）时创建初始管理员：
/// - 用户名取 `DOCKRS_ADMIN_USERNAME`，默认 `admin`
/// - 密码取 `DOCKRS_ADMIN_PASSWORD`；未设置时生成随机密码并打印到日志
pub async fn seed_admin(pool: &SqlitePool) -> sqlx::Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    if count > 0 {
        return Ok(());
    }

    let username = std::env::var("DOCKRS_ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_owned());
    let (password, generated) = match std::env::var("DOCKRS_ADMIN_PASSWORD") {
        Ok(p) => (p, false),
        Err(_) => (generate_password(), true),
    };

    let hash =
        hash_password(&password).map_err(|e| sqlx::Error::Configuration(e.to_string().into()))?;
    sqlx::query("INSERT INTO users (username, password_hash) VALUES (?, ?)")
        .bind(&username)
        .bind(&hash)
        .execute(pool)
        .await?;

    tracing::info!(username = %username, "已创建初始管理员账号");
    // 例外于「不记录密码」约定：随机密码不打印则无人可知，仅首次生成时输出一次
    if generated {
        tracing::info!(password = %password, "初始管理员密码（仅本次生成时显示，请妥善保存）");
    }
    Ok(())
}

fn generate_password() -> String {
    use argon2::password_hash::rand_core::RngCore;

    let mut bytes = [0u8; 9];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
