//! JWT 认证中间件集成测试
//!
//! 这个测试文件验证 JWT 认证中间件的基本功能。

use axum::{
    body::Body,
    http::{ header::AUTHORIZATION, Method, Request, StatusCode },
    middleware::from_fn,
    response::Response,
    routing::get,
    Router,
};
use axum_tutorial::app::middleware::auth_middleware::{
    create_jwt_auth_middleware,
    AuthenticatedUser,
    Claims,
};
use chrono::{ Duration, Utc };
use jsonwebtoken::{ encode, EncodingKey, Header };
use tower::util::ServiceExt;

/// 测试处理器：需要认证的受保护路由
async fn protected_handler(req: Request<Body>) -> Response<Body> {
    if let Some(user) = req.extensions().get::<AuthenticatedUser>() {
        let response_body = format!(
            r#"{{"message": "访问成功", "user": "{}", "user_id": "{}"}}"#,
            user.username,
            user.user_id
        );
        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Body::from(response_body))
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("未找到认证用户信息"))
            .unwrap()
    }
}

/// 集成测试：JWT 中间件基本功能
#[tokio::test]
async fn test_jwt_middleware_basic_functionality() {
    let secret = "test-jwt-secret-key";

    // 创建带有认证中间件的测试路由
    let auth_middleware = create_jwt_auth_middleware(secret.to_string());
    let protected_app = Router::new()
        .route("/protected", get(protected_handler))
        .layer(from_fn(auth_middleware));

    // 创建有效的 JWT 令牌
    let now = Utc::now();
    let claims = Claims {
        sub: "user123".to_string(),
        username: "testuser".to_string(),
        exp: (now + Duration::hours(1)).timestamp(),
        iat: now.timestamp(),
    };

    let valid_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref())
    ).unwrap();

    // 测试使用有效 JWT 访问受保护路由
    let protected_request = Request::builder()
        .method(Method::GET)
        .uri("/protected")
        .header(AUTHORIZATION, format!("Bearer {}", valid_token))
        .body(Body::empty())
        .unwrap();

    let protected_response = protected_app.clone().oneshot(protected_request).await.unwrap();
    assert_eq!(protected_response.status(), StatusCode::OK);

    // 验证响应内容
    let body = axum::body::to_bytes(protected_response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("访问成功"));
    assert!(body_str.contains("testuser"));
    assert!(body_str.contains("user123"));

    // 测试无 JWT 访问受保护路由（应该失败）
    let unauthorized_request = Request::builder()
        .method(Method::GET)
        .uri("/protected")
        .body(Body::empty())
        .unwrap();

    let unauthorized_response = protected_app.clone().oneshot(unauthorized_request).await.unwrap();
    assert_eq!(unauthorized_response.status(), StatusCode::UNAUTHORIZED);

    // 测试无效 JWT 访问受保护路由（应该失败）
    let invalid_token_request = Request::builder()
        .method(Method::GET)
        .uri("/protected")
        .header(AUTHORIZATION, "Bearer invalid-token")
        .body(Body::empty())
        .unwrap();

    let invalid_token_response = protected_app
        .clone()
        .oneshot(invalid_token_request).await
        .unwrap();
    assert_eq!(invalid_token_response.status(), StatusCode::UNAUTHORIZED);
}

/// 集成测试：JWT 中间件与不同密钥的兼容性
#[tokio::test]
async fn test_jwt_middleware_with_different_secrets() {
    let secret1 = "secret-key-1";
    let secret2 = "secret-key-2";

    // 创建两个使用不同密钥的中间件
    let middleware1 = create_jwt_auth_middleware(secret1.to_string());
    let middleware2 = create_jwt_auth_middleware(secret2.to_string());

    let app1 = Router::new()
        .route("/protected", get(protected_handler))
        .layer(from_fn(middleware1));

    let app2 = Router::new()
        .route("/protected", get(protected_handler))
        .layer(from_fn(middleware2));

    // 使用 secret1 创建令牌
    let claims = Claims {
        sub: "user123".to_string(),
        username: "testuser".to_string(),
        exp: (Utc::now() + Duration::hours(1)).timestamp(),
        iat: Utc::now().timestamp(),
    };

    let token1 = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret1.as_ref())
    ).unwrap();

    // 使用 token1 访问 app1（应该成功）
    let request1 = Request::builder()
        .method(Method::GET)
        .uri("/protected")
        .header(AUTHORIZATION, format!("Bearer {}", token1))
        .body(Body::empty())
        .unwrap();

    let response1 = app1.oneshot(request1).await.unwrap();
    assert_eq!(response1.status(), StatusCode::OK);

    // 使用 token1 访问 app2（应该失败，因为密钥不匹配）
    let request2 = Request::builder()
        .method(Method::GET)
        .uri("/protected")
        .header(AUTHORIZATION, format!("Bearer {}", token1))
        .body(Body::empty())
        .unwrap();

    let response2 = app2.oneshot(request2).await.unwrap();
    assert_eq!(response2.status(), StatusCode::UNAUTHORIZED);
}

/// 集成测试：过期令牌处理
#[tokio::test]
async fn test_expired_token_handling() {
    let secret = "test-secret";
    let auth_middleware = create_jwt_auth_middleware(secret.to_string());
    let app = Router::new()
        .route("/protected", get(protected_handler))
        .layer(from_fn(auth_middleware));

    // 创建过期的令牌
    let now = Utc::now();
    let expired_claims = Claims {
        sub: "user123".to_string(),
        username: "testuser".to_string(),
        exp: (now - Duration::hours(1)).timestamp(), // 1小时前过期
        iat: (now - Duration::hours(2)).timestamp(),
    };

    let expired_token = encode(
        &Header::default(),
        &expired_claims,
        &EncodingKey::from_secret(secret.as_ref())
    ).unwrap();

    // 使用过期令牌访问受保护路由（应该失败）
    let request = Request::builder()
        .method(Method::GET)
        .uri("/protected")
        .header(AUTHORIZATION, format!("Bearer {}", expired_token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
