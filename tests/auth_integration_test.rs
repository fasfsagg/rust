//! 认证服务集成测试
//!
//! 这个文件包含了认证服务的集成测试，验证认证服务与数据库的完整交互。
//! 集成测试确保多个组件（服务层、仓库层、数据库）能够正确协作。

use axum_tutorial::app::model::auth::{ LoginRequest, RegisterRequest };
use axum_tutorial::app::repository::user_repository::{ UserRepository, UserRepositoryContract };
use axum_tutorial::app::service::auth_service::{ login_user, register_user };
use axum_tutorial::error::AppError;
use migration::{ Migrator, MigratorTrait };
use sea_orm::{ Database, DatabaseConnection };
use std::sync::Arc;

/// 设置测试数据库
/// 创建一个内存 SQLite 数据库用于测试
async fn setup_test_db() -> DatabaseConnection {
    let db = Database::connect("sqlite::memory:").await.expect(
        "Failed to connect to test database"
    );

    // 运行迁移
    Migrator::up(&db, None).await.expect("Failed to run migrations");

    db
}

#[tokio::test]
async fn test_auth_service_integration_register_and_login() {
    // 设置测试数据库
    let db = setup_test_db().await;
    let user_repo: Arc<dyn UserRepositoryContract> = Arc::new(UserRepository::new(db));

    // 测试数据
    let username = "integration_test_user";
    let password = "test_password_123";

    // 1. 测试用户注册
    let register_payload = RegisterRequest {
        username: username.to_string(),
        password: password.to_string(),
        confirm_password: password.to_string(),
    };

    let register_result = register_user(user_repo.clone(), register_payload).await;
    assert!(register_result.is_ok(), "用户注册应该成功");

    let user_response = register_result.unwrap();
    assert_eq!(user_response.username, username);
    assert!(!user_response.id.to_string().is_empty());

    // 2. 测试重复注册（应该失败）
    let duplicate_register_payload = RegisterRequest {
        username: username.to_string(),
        password: "different_password".to_string(),
        confirm_password: "different_password".to_string(),
    };

    let duplicate_result = register_user(user_repo.clone(), duplicate_register_payload).await;
    assert!(duplicate_result.is_err(), "重复注册应该失败");
    match duplicate_result.err().unwrap() {
        AppError::UserAlreadyExists(name) => assert_eq!(name, username),
        _ => panic!("应该返回 UserAlreadyExists 错误"),
    }

    // 3. 测试用户登录（正确密码）
    let login_payload = LoginRequest {
        username: username.to_string(),
        password: password.to_string(),
    };

    let login_result = login_user(user_repo.clone(), login_payload, "test_jwt_secret").await;
    assert!(login_result.is_ok(), "正确密码登录应该成功");

    let auth_response = login_result.unwrap();
    assert_eq!(auth_response.user.username, username);
    assert_eq!(auth_response.token_type, "Bearer");
    assert!(!auth_response.access_token.is_empty());
    assert!(auth_response.expires_in > 0);

    // 4. 测试用户登录（错误密码）
    let wrong_password_payload = LoginRequest {
        username: username.to_string(),
        password: "wrong_password".to_string(),
    };

    let wrong_password_result = login_user(
        user_repo.clone(),
        wrong_password_payload,
        "test_jwt_secret"
    ).await;
    assert!(wrong_password_result.is_err(), "错误密码登录应该失败");
    match wrong_password_result.err().unwrap() {
        AppError::InvalidCredentials => {} // 期望的错误
        _ => panic!("应该返回 InvalidCredentials 错误"),
    }

    // 5. 测试不存在的用户登录
    let nonexistent_user_payload = LoginRequest {
        username: "nonexistent_user".to_string(),
        password: "any_password".to_string(),
    };

    let nonexistent_result = login_user(
        user_repo,
        nonexistent_user_payload,
        "test_jwt_secret"
    ).await;
    assert!(nonexistent_result.is_err(), "不存在的用户登录应该失败");
    match nonexistent_result.err().unwrap() {
        AppError::InvalidCredentials => {} // 期望的错误
        _ => panic!("应该返回 InvalidCredentials 错误"),
    }
}

#[tokio::test]
async fn test_password_hashing_security() {
    // 设置测试数据库
    let db = setup_test_db().await;
    let user_repo: Arc<dyn UserRepositoryContract> = Arc::new(UserRepository::new(db));

    // 测试相同密码产生不同哈希
    let username1 = "user1";
    let username2 = "user2";
    let same_password = "same_password_123";

    // 注册两个用户使用相同密码
    let register1 = RegisterRequest {
        username: username1.to_string(),
        password: same_password.to_string(),
        confirm_password: same_password.to_string(),
    };

    let register2 = RegisterRequest {
        username: username2.to_string(),
        password: same_password.to_string(),
        confirm_password: same_password.to_string(),
    };

    let result1 = register_user(user_repo.clone(), register1).await;
    let result2 = register_user(user_repo.clone(), register2).await;

    assert!(result1.is_ok() && result2.is_ok(), "两个用户注册都应该成功");

    // 验证两个用户都能用相同密码登录
    let login1 = LoginRequest {
        username: username1.to_string(),
        password: same_password.to_string(),
    };

    let login2 = LoginRequest {
        username: username2.to_string(),
        password: same_password.to_string(),
    };

    let login_result1 = login_user(user_repo.clone(), login1, "test_secret").await;
    let login_result2 = login_user(user_repo, login2, "test_secret").await;

    assert!(login_result1.is_ok() && login_result2.is_ok(), "两个用户都应该能够登录");
}
