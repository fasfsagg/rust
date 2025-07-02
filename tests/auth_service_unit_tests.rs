// 认证服务单元测试
// 目标：提升auth_service.rs的测试覆盖率

use axum_tutorial::app::service::auth_service::{ register_user, login_user };
use axum_tutorial::app::model::auth::{ RegisterRequest, LoginRequest };
use axum_tutorial::app::repository::user_repository::{ UserRepository, UserRepositoryContract };
use uuid::Uuid;
use tempfile::tempdir;
use sea_orm::Database;
use migration::{ Migrator, MigratorTrait };
use std::sync::Arc;

/// 创建测试用的数据库连接池
async fn create_test_db_pool() -> sea_orm::DatabaseConnection {
    let temp_dir = tempdir().expect("创建临时目录失败");
    let db_path = temp_dir.path().join("test.db");
    let database_url = format!("sqlite://{}?mode=rwc", db_path.display());

    let db_pool = Database::connect(&database_url).await.expect("创建数据库连接池失败");

    // 运行迁移
    Migrator::up(&db_pool, None).await.expect("数据库迁移失败");

    db_pool
}

#[tokio::test]
async fn test_auth_service_register_success() {
    // 创建测试数据库
    let db = create_test_db_pool().await;
    let repo: Arc<dyn UserRepositoryContract> = Arc::new(UserRepository::new(db));

    // 创建注册请求
    let register_request = RegisterRequest {
        username: "testuser".to_string(),
        password: "testpass123".to_string(),
        confirm_password: "testpass123".to_string(),
    };

    // 测试用户注册
    let result = register_user(repo, register_request).await;

    // 验证结果
    assert!(result.is_ok(), "用户注册应该成功");

    if let Ok(user) = result {
        assert_eq!(user.username, "testuser");
        assert!(!user.id.to_string().is_empty());
    }
}

#[tokio::test]
async fn test_auth_service_register_duplicate_username() {
    // 创建测试数据库
    let db = create_test_db_pool().await;
    let repo: Arc<dyn UserRepositoryContract> = Arc::new(UserRepository::new(db));

    // 第一次注册
    let register_request1 = RegisterRequest {
        username: "duplicate_user".to_string(),
        password: "pass12345".to_string(),
        confirm_password: "pass12345".to_string(),
    };
    let result1 = register_user(repo.clone(), register_request1).await;
    assert!(result1.is_ok(), "第一次注册应该成功");

    // 第二次注册相同用户名
    let register_request2 = RegisterRequest {
        username: "duplicate_user".to_string(),
        password: "different_pass123".to_string(),
        confirm_password: "different_pass123".to_string(),
    };
    let result2 = register_user(repo, register_request2).await;
    assert!(result2.is_err(), "重复用户名注册应该失败");
}

#[tokio::test]
async fn test_auth_service_login_success() {
    // 创建测试数据库
    let db = create_test_db_pool().await;
    let repo: Arc<dyn UserRepositoryContract> = Arc::new(UserRepository::new(db));

    // 先注册用户
    let register_request = RegisterRequest {
        username: "loginuser".to_string(),
        password: "loginpass123".to_string(),
        confirm_password: "loginpass123".to_string(),
    };
    let _register_result = register_user(repo.clone(), register_request).await;
    assert!(_register_result.is_ok(), "用户注册应该成功");

    // 测试登录
    let login_request = LoginRequest {
        username: "loginuser".to_string(),
        password: "loginpass123".to_string(),
    };
    let result = login_user(repo, login_request, "test_secret").await;

    // 验证登录成功
    assert!(result.is_ok(), "用户登录应该成功");
    if let Ok(auth_response) = result {
        assert_eq!(auth_response.user.username, "loginuser");
        assert!(!auth_response.access_token.is_empty());
        assert_eq!(auth_response.token_type, "Bearer");
    }
}

#[tokio::test]
async fn test_auth_service_login_invalid_username() {
    // 创建测试数据库
    let db = create_test_db_pool().await;
    let repo: Arc<dyn UserRepositoryContract> = Arc::new(UserRepository::new(db));

    // 测试不存在的用户名登录
    let login_request = LoginRequest {
        username: "nonexistent".to_string(),
        password: "anypass".to_string(),
    };
    let result = login_user(repo, login_request, "test_secret").await;

    // 验证登录失败
    assert!(result.is_err(), "不存在的用户名登录应该失败");
}

#[tokio::test]
async fn test_auth_service_login_invalid_password() {
    // 创建测试数据库
    let db = create_test_db_pool().await;
    let repo: Arc<dyn UserRepositoryContract> = Arc::new(UserRepository::new(db));

    // 先注册用户
    let register_request = RegisterRequest {
        username: "passuser".to_string(),
        password: "correctpass".to_string(),
        confirm_password: "correctpass".to_string(),
    };
    let _register_result = register_user(repo.clone(), register_request).await;
    assert!(_register_result.is_ok(), "用户注册应该成功");

    // 测试错误密码登录
    let login_request = LoginRequest {
        username: "passuser".to_string(),
        password: "wrongpass".to_string(),
    };
    let result = login_user(repo, login_request, "test_secret").await;

    // 验证登录失败
    assert!(result.is_err(), "错误密码登录应该失败");
}

#[tokio::test]
async fn test_auth_service_register_empty_username() {
    // 创建测试数据库
    let db = create_test_db_pool().await;
    let repo: Arc<dyn UserRepositoryContract> = Arc::new(UserRepository::new(db));

    // 测试空用户名注册
    let register_request = RegisterRequest {
        username: "".to_string(),
        password: "somepass123".to_string(),
        confirm_password: "somepass123".to_string(),
    };
    let result = register_user(repo, register_request).await;

    // 验证注册失败
    assert!(result.is_err(), "空用户名注册应该失败");
}

#[tokio::test]
async fn test_auth_service_register_empty_password() {
    // 创建测试数据库
    let db = create_test_db_pool().await;
    let repo: Arc<dyn UserRepositoryContract> = Arc::new(UserRepository::new(db));

    // 测试空密码注册
    let register_request = RegisterRequest {
        username: "someuser".to_string(),
        password: "".to_string(),
        confirm_password: "".to_string(),
    };
    let result = register_user(repo, register_request).await;

    // 验证注册失败
    assert!(result.is_err(), "空密码注册应该失败");
}
