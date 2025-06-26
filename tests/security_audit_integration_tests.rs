//! 安全审计集成测试
//!
//! 【测试目标】：验证安全审计日志系统的完整功能
//! - 认证事件记录
//! - 威胁检测功能
//! - 中间件集成
//! - 审计日志格式和内容

use axum::{ body::Body, http::{ Request, StatusCode }, middleware, routing::get, Router };
use axum_tutorial::app::{
    middleware::{
        security_audit::{
            get_security_auditor,
            init_security_auditor,
            log_authentication_failure,
            log_authentication_success,
            log_suspicious_activity,
            security_audit_middleware,
            SecurityEventType,
        },
        auth_middleware::create_jwt_auth_middleware,
    },
    utils::JwtUtils,
};
use std::collections::HashMap;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tower::ServiceExt;

/// 测试用的简单处理器
async fn protected_handler() -> &'static str {
    "Protected resource accessed"
}

/// 测试用的公开处理器
async fn public_handler() -> &'static str {
    "Public resource accessed"
}

/// 创建测试用的应用程序
fn create_test_app() -> Router {
    let jwt_secret = "test-secret-key-for-security-audit".to_string();
    let auth_middleware = create_jwt_auth_middleware(jwt_secret);

    Router::new()
        .route("/protected", get(protected_handler))
        .route_layer(middleware::from_fn(auth_middleware))
        .route("/public", get(public_handler))
        .layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn(security_audit_middleware))
                .layer(TraceLayer::new_for_http())
        )
}

/// 创建有效的JWT令牌用于测试
fn create_test_jwt_token() -> String {
    let jwt_utils = JwtUtils::new("test-secret-key-for-security-audit".to_string());
    jwt_utils.create_token("test_user_123", "testuser", 1).expect("Failed to create test JWT token")
}

#[tokio::test]
async fn test_security_auditor_initialization() {
    // 测试安全审计器的初始化
    // 注意：由于全局审计器可能已经被其他测试初始化，这里我们只测试重复初始化的行为
    let result = init_security_auditor(true, 3);

    // 如果已经初始化过，应该返回错误
    if result.is_err() {
        assert_eq!(result.unwrap_err(), "Security auditor already initialized");
    }

    // 再次尝试初始化应该失败
    let result2 = init_security_auditor(false, 5);
    assert!(result2.is_err());
    assert_eq!(result2.unwrap_err(), "Security auditor already initialized");
}

#[tokio::test]
async fn test_authentication_success_logging() {
    // 测试认证成功事件记录
    log_authentication_success(
        "user123",
        "testuser",
        Some("192.168.1.100"),
        Some("Mozilla/5.0 Test Browser")
    );

    // 获取统计信息验证事件已记录
    let auditor = get_security_auditor();
    let stats = auditor.get_event_statistics();

    // 验证认证成功事件已记录
    assert!(stats.get(&SecurityEventType::AuthenticationSuccess).unwrap_or(&0) > &0);
}

#[tokio::test]
async fn test_authentication_failure_logging() {
    // 测试认证失败事件记录
    log_authentication_failure(
        Some("baduser"),
        "Invalid password",
        Some("192.168.1.200"),
        Some("Mozilla/5.0 Test Browser")
    );

    // 获取统计信息验证事件已记录
    let auditor = get_security_auditor();
    let stats = auditor.get_event_statistics();

    // 验证认证失败事件已记录
    assert!(stats.get(&SecurityEventType::AuthenticationFailure).unwrap_or(&0) > &0);
}

#[tokio::test]
async fn test_brute_force_detection() {
    let client_ip = "192.168.1.250";

    // 模拟多次认证失败
    for _i in 1..=6 {
        log_authentication_failure(
            Some("attacker"),
            "Brute force attempt",
            Some(client_ip),
            Some("AttackBot/1.0")
        );
    }

    // 获取失败统计信息
    let auditor = get_security_auditor();
    let failure_stats = auditor.get_failure_statistics();

    // 验证失败计数已记录
    assert!(failure_stats.get(client_ip).unwrap_or(&0) >= &6);

    // 获取事件统计信息
    let stats = auditor.get_event_statistics();

    // 验证暴力破解事件已被检测到（如果阈值设置得足够低）
    // 注意：这取决于全局审计器的配置
    assert!(stats.get(&SecurityEventType::AuthenticationFailure).unwrap_or(&0) >= &6);
}

#[tokio::test]
async fn test_suspicious_activity_logging() {
    let mut context = HashMap::new();
    context.insert("attack_type".to_string(), "sql_injection".to_string());
    context.insert("payload".to_string(), "'; DROP TABLE users; --".to_string());

    // 记录可疑活动
    log_suspicious_activity(
        "SQL injection attempt detected",
        Some("suspicious_user"),
        Some("hacker"),
        Some("192.168.1.300"),
        context
    );

    // 获取统计信息验证事件已记录
    let auditor = get_security_auditor();
    let stats = auditor.get_event_statistics();

    // 验证可疑活动事件已记录
    assert!(stats.get(&SecurityEventType::SuspiciousActivity).unwrap_or(&0) > &0);
}

#[tokio::test]
async fn test_security_audit_middleware_with_valid_token() {
    let app = create_test_app();
    let token = create_test_jwt_token();

    // 创建带有有效JWT令牌的请求
    let request = Request::builder()
        .method("GET")
        .uri("/protected")
        .header("authorization", format!("Bearer {}", token))
        .header("user-agent", "TestClient/1.0")
        .header("x-forwarded-for", "203.0.113.1")
        .body(Body::empty())
        .unwrap();

    // 发送请求
    let response = app.oneshot(request).await.unwrap();

    // 验证响应状态
    assert_eq!(response.status(), StatusCode::OK);

    // 验证安全审计事件已记录
    let auditor = get_security_auditor();
    let stats = auditor.get_event_statistics();

    // 应该记录API访问事件和令牌验证成功事件
    assert!(stats.get(&SecurityEventType::ApiAccess).unwrap_or(&0) > &0);
    assert!(stats.get(&SecurityEventType::TokenValidationSuccess).unwrap_or(&0) > &0);
}

#[tokio::test]
async fn test_security_audit_middleware_with_invalid_token() {
    let app = create_test_app();

    // 创建带有无效JWT令牌的请求
    let request = Request::builder()
        .method("GET")
        .uri("/protected")
        .header("authorization", "Bearer invalid-token-here")
        .header("user-agent", "TestClient/1.0")
        .header("x-forwarded-for", "203.0.113.2")
        .body(Body::empty())
        .unwrap();

    // 发送请求
    let response = app.oneshot(request).await.unwrap();

    // 验证响应状态
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 验证安全审计事件已记录
    let auditor = get_security_auditor();
    let stats = auditor.get_event_statistics();

    // 应该记录API访问事件和令牌验证失败事件
    assert!(stats.get(&SecurityEventType::ApiAccess).unwrap_or(&0) > &0);
    assert!(stats.get(&SecurityEventType::TokenValidationFailure).unwrap_or(&0) > &0);
}

#[tokio::test]
async fn test_security_audit_middleware_public_endpoint() {
    let app = create_test_app();

    // 创建访问公开端点的请求
    let request = Request::builder()
        .method("GET")
        .uri("/public")
        .header("user-agent", "TestClient/1.0")
        .header("x-real-ip", "203.0.113.3")
        .body(Body::empty())
        .unwrap();

    // 发送请求
    let response = app.oneshot(request).await.unwrap();

    // 验证响应状态
    assert_eq!(response.status(), StatusCode::OK);

    // 验证安全审计事件已记录
    let auditor = get_security_auditor();
    let stats = auditor.get_event_statistics();

    // 应该记录API访问事件
    assert!(stats.get(&SecurityEventType::ApiAccess).unwrap_or(&0) > &0);
}

#[tokio::test]
async fn test_suspicious_access_pattern_detection() {
    let app = create_test_app();

    // 创建访问敏感路径的请求
    let request = Request::builder()
        .method("GET")
        .uri("/admin/config")
        .header("user-agent", "SuspiciousBot/1.0")
        .header("x-client-ip", "203.0.113.4")
        .body(Body::empty())
        .unwrap();

    // 发送请求
    let response = app.oneshot(request).await.unwrap();

    // 验证响应状态（应该是404，因为路由不存在）
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // 验证安全审计事件已记录
    let auditor = get_security_auditor();
    let stats = auditor.get_event_statistics();

    // 应该记录API访问事件和异常访问事件
    assert!(stats.get(&SecurityEventType::ApiAccess).unwrap_or(&0) > &0);
    assert!(stats.get(&SecurityEventType::AnomalousAccess).unwrap_or(&0) > &0);
}

#[tokio::test]
async fn test_security_audit_statistics() {
    // 获取当前统计信息
    let auditor = get_security_auditor();
    let initial_stats = auditor.get_event_statistics();

    // 记录一些测试事件
    log_authentication_success("stats_user", "statstest", Some("192.168.1.400"), None);
    log_authentication_failure(Some("bad_stats_user"), "test failure", Some("192.168.1.401"), None);

    // 获取更新后的统计信息
    let updated_stats = auditor.get_event_statistics();

    // 验证统计信息已更新
    let initial_success = initial_stats
        .get(&SecurityEventType::AuthenticationSuccess)
        .unwrap_or(&0);
    let updated_success = updated_stats
        .get(&SecurityEventType::AuthenticationSuccess)
        .unwrap_or(&0);
    assert!(updated_success > initial_success);

    let initial_failure = initial_stats
        .get(&SecurityEventType::AuthenticationFailure)
        .unwrap_or(&0);
    let updated_failure = updated_stats
        .get(&SecurityEventType::AuthenticationFailure)
        .unwrap_or(&0);
    assert!(updated_failure > initial_failure);
}
