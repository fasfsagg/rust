//! `auth_service.rs`
//!
//! 【认证服务模块】
//! 这个模块实现了用户认证相关的业务逻辑，包括用户注册和登录功能。
//! 它是服务层的一部分，负责处理认证相关的业务规则和流程。
//!
//! ## 核心功能
//! - **用户注册**: 验证用户输入，使用 Argon2 哈希密码，创建新用户
//! - **用户登录**: 验证用户凭据，生成 JWT 令牌
//!
//! ## 安全特性
//! - 使用 Argon2id 算法进行密码哈希（比 bcrypt 更安全的现代算法）
//! - JWT 令牌生成和管理
//! - 输入验证和错误处理
//!
//! ## 设计原则
//! - **单一职责**: 专注于认证相关的业务逻辑
//! - **依赖注入**: 通过参数传递仓库实例，便于测试
//! - **错误处理**: 统一的错误处理和返回类型

use crate::app::model::auth::{ LoginRequest, RegisterRequest };
use crate::app::model::user_entity::UserResponse;
use crate::app::repository::user_repository::UserRepositoryContract;
use crate::error::{ AppError, Result };
use argon2::{
    password_hash::{ rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString },
    Argon2,
};
use chrono::{ Duration, Utc };
use jsonwebtoken::{ encode, EncodingKey, Header };
use migration::user_entity::ActiveModel;
use sea_orm::{ prelude::Uuid, ActiveValue };
use serde::{ Deserialize, Serialize };
use std::sync::Arc;

/// JWT 声明结构体
/// 包含用户身份信息和令牌有效期
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// 用户 ID
    pub sub: String,
    /// 用户名
    pub username: String,
    /// 令牌过期时间（Unix 时间戳）
    pub exp: i64,
    /// 令牌签发时间（Unix 时间戳）
    pub iat: i64,
}

/// JWT 响应结构体
/// 用于返回登录成功后的令牌和用户信息
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    /// JWT 访问令牌
    pub access_token: String,
    /// 令牌类型（通常是 "Bearer"）
    pub token_type: String,
    /// 令牌过期时间（秒）
    pub expires_in: i64,
    /// 用户信息
    pub user: UserResponse,
}

/// 服务函数：用户注册
///
/// 处理用户注册请求，包括：
/// 1. 验证用户名是否已存在
/// 2. 使用 Argon2 哈希密码
/// 3. 创建新用户记录
///
/// # 参数
/// - `repo`: 用户仓库实例
/// - `payload`: 注册请求数据
///
/// # 返回
/// 成功时返回新创建的用户信息，失败时返回相应错误
pub async fn register_user(
    repo: Arc<dyn UserRepositoryContract>,
    payload: RegisterRequest
) -> Result<UserResponse> {
    tracing::info!(username = %payload.username, "开始处理用户注册请求");

    // 1. 检查用户名是否已存在
    if let Some(_existing_user) = repo.find_by_username(&payload.username).await? {
        tracing::warn!(username = %payload.username, "用户注册失败，用户名已存在");
        return Err(AppError::UserAlreadyExists(payload.username));
    }

    // 2. 使用 Argon2 哈希密码
    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = argon2
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|e| AppError::PasswordHashError(e.to_string()))?
        .to_string();

    // 3. 创建新用户的 ActiveModel
    let new_user = ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        username: ActiveValue::Set(payload.username.clone()),
        password_hash: ActiveValue::Set(password_hash),
        ..Default::default()
    };

    // 4. 保存到数据库
    let created_user = repo.create(new_user).await?;

    tracing::info!(username = %payload.username, user_id = %created_user.id, "用户注册成功");
    Ok(created_user.into())
}

/// 服务函数：用户登录
///
/// 处理用户登录请求，包括：
/// 1. 根据用户名查找用户
/// 2. 验证密码
/// 3. 生成 JWT 令牌
///
/// # 参数
/// - `repo`: 用户仓库实例
/// - `payload`: 登录请求数据
/// - `jwt_secret`: JWT 签名密钥
///
/// # 返回
/// 成功时返回认证响应（包含令牌和用户信息），失败时返回相应错误
pub async fn login_user(
    repo: Arc<dyn UserRepositoryContract>,
    payload: LoginRequest,
    jwt_secret: &str
) -> Result<AuthResponse> {
    tracing::info!(username = %payload.username, "开始处理用户登录请求");

    // 1. 根据用户名查找用户
    let user = repo
        .find_by_username(&payload.username).await?
        .ok_or_else(|| AppError::InvalidCredentials)?;

    // 2. 验证密码
    let argon2 = Argon2::default();
    let parsed_hash = PasswordHash::new(&user.password_hash).map_err(|e|
        AppError::PasswordHashError(e.to_string())
    )?;

    if argon2.verify_password(payload.password.as_bytes(), &parsed_hash).is_err() {
        tracing::warn!(username = %payload.username, "用户登录失败，密码验证失败");
        return Err(AppError::InvalidCredentials);
    }

    // 3. 生成 JWT 令牌
    let now = Utc::now();
    let expires_in = 24 * 60 * 60; // 24小时，单位：秒
    let exp = now + Duration::seconds(expires_in);

    let claims = Claims {
        sub: user.id.to_string(),
        username: user.username.clone(),
        exp: exp.timestamp(),
        iat: now.timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_ref())
    ).map_err(|e| AppError::TokenGenerationError(e.to_string()))?;

    tracing::info!(username = %payload.username, user_id = %user.id, "用户登录成功");

    Ok(AuthResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_in,
        user: user.into(),
    })
}

// --- 单元测试 ---
#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use migration::user_entity;
    use sea_orm::{ prelude::Uuid, DbErr };
    use std::sync::{ Arc, Mutex };

    // 1. 创建模拟用户仓库 (Mock User Repository)
    #[derive(Default)]
    struct MockUserRepository {
        find_by_username_result: Mutex<
            Option<std::result::Result<Option<user_entity::Model>, DbErr>>
        >,
        create_result: Mutex<Option<std::result::Result<user_entity::Model, DbErr>>>,
    }

    // 2. 为模拟仓库实现 `UserRepositoryContract` Trait
    #[async_trait]
    impl UserRepositoryContract for MockUserRepository {
        async fn find_by_username(
            &self,
            _username: &str
        ) -> std::result::Result<Option<user_entity::Model>, DbErr> {
            self.find_by_username_result.lock().unwrap().take().unwrap()
        }

        async fn create(
            &self,
            _data: user_entity::ActiveModel
        ) -> std::result::Result<user_entity::Model, DbErr> {
            self.create_result.lock().unwrap().take().unwrap()
        }
    }

    // 辅助函数，创建一个包含预设数据的 user model
    fn create_dummy_user_model(
        id: Uuid,
        username: &str,
        password_hash: &str
    ) -> user_entity::Model {
        user_entity::Model {
            id,
            username: username.to_string(),
            password_hash: password_hash.to_string(),
        }
    }

    #[tokio::test]
    async fn test_register_user_success() {
        // --- 准备 (Arrange) ---
        let mock_repo = MockUserRepository::default();
        let user_id = Uuid::new_v4();
        let username = "testuser";
        let expected_user = create_dummy_user_model(user_id, username, "hashed_password");

        // 模拟用户名不存在（注册可以进行）
        *mock_repo.find_by_username_result.lock().unwrap() = Some(Ok(None));
        // 模拟创建用户成功
        *mock_repo.create_result.lock().unwrap() = Some(Ok(expected_user.clone()));

        let repo: Arc<dyn UserRepositoryContract> = Arc::new(mock_repo);
        let payload = RegisterRequest {
            username: username.to_string(),
            password: "password123".to_string(),
            confirm_password: "password123".to_string(),
        };

        // --- 执行 (Act) ---
        let result = register_user(repo, payload).await;

        // --- 断言 (Assert) ---
        assert!(result.is_ok());
        let user_response = result.unwrap();
        assert_eq!(user_response.id, user_id);
        assert_eq!(user_response.username, username);
    }

    #[tokio::test]
    async fn test_register_user_already_exists() {
        // --- 准备 (Arrange) ---
        let mock_repo = MockUserRepository::default();
        let username = "existinguser";
        let existing_user = create_dummy_user_model(Uuid::new_v4(), username, "hashed_password");

        // 模拟用户名已存在
        *mock_repo.find_by_username_result.lock().unwrap() = Some(Ok(Some(existing_user)));

        let repo: Arc<dyn UserRepositoryContract> = Arc::new(mock_repo);
        let payload = RegisterRequest {
            username: username.to_string(),
            password: "password123".to_string(),
            confirm_password: "password123".to_string(),
        };

        // --- 执行 (Act) ---
        let result = register_user(repo, payload).await;

        // --- 断言 (Assert) ---
        assert!(result.is_err());
        match result.err().unwrap() {
            AppError::UserAlreadyExists(name) => assert_eq!(name, username),
            _ => panic!("Expected UserAlreadyExists error"),
        }
    }

    #[tokio::test]
    async fn test_login_user_success() {
        // --- 准备 (Arrange) ---
        let mock_repo = MockUserRepository::default();
        let user_id = Uuid::new_v4();
        let username = "testuser";
        let password = "password123";

        // 使用 Argon2 生成真实的密码哈希用于测试
        let argon2 = Argon2::default();
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = argon2.hash_password(password.as_bytes(), &salt).unwrap().to_string();

        let user = create_dummy_user_model(user_id, username, &password_hash);

        // 模拟找到用户
        *mock_repo.find_by_username_result.lock().unwrap() = Some(Ok(Some(user)));

        let repo: Arc<dyn UserRepositoryContract> = Arc::new(mock_repo);
        let payload = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };

        // --- 执行 (Act) ---
        let result = login_user(repo, payload, "test_secret").await;

        // --- 断言 (Assert) ---
        assert!(result.is_ok());
        let auth_response = result.unwrap();
        assert_eq!(auth_response.token_type, "Bearer");
        assert_eq!(auth_response.user.username, username);
        assert!(!auth_response.access_token.is_empty());
    }

    #[tokio::test]
    async fn test_login_user_not_found() {
        // --- 准备 (Arrange) ---
        let mock_repo = MockUserRepository::default();
        let username = "nonexistentuser";

        // 模拟用户不存在
        *mock_repo.find_by_username_result.lock().unwrap() = Some(Ok(None));

        let repo: Arc<dyn UserRepositoryContract> = Arc::new(mock_repo);
        let payload = LoginRequest {
            username: username.to_string(),
            password: "password123".to_string(),
        };

        // --- 执行 (Act) ---
        let result = login_user(repo, payload, "test_secret").await;

        // --- 断言 (Assert) ---
        assert!(result.is_err());
        match result.err().unwrap() {
            AppError::InvalidCredentials => {} // 期望的错误
            _ => panic!("Expected InvalidCredentials error"),
        }
    }
}
