//! WebSocket 集成测试
//!
//! 本模块包含 WebSocket JWT 验证逻辑的集成测试。
//! 测试覆盖以下场景：
//! - WebSocket JWT token 提取和验证的集成测试
//! - 不同的 token 传递方式的集成测试
//! - WebSocket 处理器的身份验证流程测试
//!
//! 注意：完整的 WebSocket 连接端到端测试将在 Playwright 中进行

use axum::http::{ HeaderMap, HeaderValue, Uri };
use chrono::{ Utc, Duration };
use jsonwebtoken::{ encode, EncodingKey, Header };

// 使用项目中定义的类型
use axum_tutorial::app::utils::Claims;
use axum_tutorial::app::middleware::auth_middleware::{
    extract_websocket_token,
    validate_websocket_jwt_token,
    WebSocketAuthError,
};

/// 测试用的 JWT 密钥
const TEST_JWT_SECRET: &str = "test-websocket-integration-jwt-secret-key";

/// 创建有效的 JWT token
fn create_valid_jwt_token(user_id: &str, username: &str) -> String {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        iat: now.timestamp(),
        exp: (now + Duration::hours(1)).timestamp(), // 1小时后过期
    };

    encode(&Header::default(), &claims, &EncodingKey::from_secret(TEST_JWT_SECRET.as_ref())).expect(
        "Failed to create JWT token"
    )
}

/// 创建过期的 JWT token
fn create_expired_jwt_token(user_id: &str, username: &str) -> String {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        iat: (now - Duration::hours(2)).timestamp(), // 2小时前签发
        exp: (now - Duration::hours(1)).timestamp(), // 1小时前过期
    };

    encode(&Header::default(), &claims, &EncodingKey::from_secret(TEST_JWT_SECRET.as_ref())).expect(
        "Failed to create expired JWT token"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 WebSocket 集成 - 从查询参数提取有效 token
    #[tokio::test]
    async fn test_websocket_integration_valid_token_from_query() {
        let token = create_valid_jwt_token("user123", "testuser");
        let uri: Uri = format!("ws://localhost:3000/ws?token={}", token).parse().unwrap();
        let headers = HeaderMap::new();

        // 1. 测试 token 提取
        let extracted_token = extract_websocket_token(&uri, &headers);
        assert!(extracted_token.is_some());
        assert_eq!(extracted_token.unwrap(), token);

        // 2. 测试 token 验证
        let result = validate_websocket_jwt_token(&token, TEST_JWT_SECRET).await;
        assert!(result.is_ok());

        let claims = result.unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.username, "testuser");
    }

    /// 测试 WebSocket 集成 - 从协议头提取有效 token
    #[tokio::test]
    async fn test_websocket_integration_valid_token_from_protocol_header() {
        let token = create_valid_jwt_token("user456", "anotheruser");
        let uri: Uri = "ws://localhost:3000/ws".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            "sec-websocket-protocol",
            HeaderValue::from_str(&format!("access_token.{}", token)).unwrap()
        );

        // 1. 测试 token 提取
        let extracted_token = extract_websocket_token(&uri, &headers);
        assert!(extracted_token.is_some());
        assert_eq!(extracted_token.unwrap(), token);

        // 2. 测试 token 验证
        let result = validate_websocket_jwt_token(&token, TEST_JWT_SECRET).await;
        assert!(result.is_ok());

        let claims = result.unwrap();
        assert_eq!(claims.sub, "user456");
        assert_eq!(claims.username, "anotheruser");
    }

    /// 测试 WebSocket 集成 - 过期 token 被拒绝
    #[tokio::test]
    async fn test_websocket_integration_expired_token_rejected() {
        let expired_token = create_expired_jwt_token("user123", "testuser");
        let uri: Uri = format!("ws://localhost:3000/ws?token={}", expired_token).parse().unwrap();
        let headers = HeaderMap::new();

        // 1. 测试 token 提取成功
        let extracted_token = extract_websocket_token(&uri, &headers);
        assert!(extracted_token.is_some());

        // 2. 测试 token 验证失败
        let result = validate_websocket_jwt_token(&expired_token, TEST_JWT_SECRET).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), WebSocketAuthError::TokenExpired);
    }

    /// 测试 WebSocket 集成 - 无效 token 被拒绝
    #[tokio::test]
    async fn test_websocket_integration_invalid_token_rejected() {
        let invalid_token = "invalid.jwt.token";
        let uri: Uri = format!("ws://localhost:3000/ws?token={}", invalid_token).parse().unwrap();
        let headers = HeaderMap::new();

        // 1. 测试 token 提取成功
        let extracted_token = extract_websocket_token(&uri, &headers);
        assert!(extracted_token.is_some());

        // 2. 测试 token 验证失败
        let result = validate_websocket_jwt_token(invalid_token, TEST_JWT_SECRET).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), WebSocketAuthError::TokenInvalid);
    }

    /// 测试 WebSocket 集成 - 缺失 token 被拒绝
    #[tokio::test]
    async fn test_websocket_integration_missing_token_rejected() {
        let uri: Uri = "ws://localhost:3000/ws".parse().unwrap();
        let headers = HeaderMap::new();

        // 1. 测试 token 提取失败
        let extracted_token = extract_websocket_token(&uri, &headers);
        assert!(extracted_token.is_none());

        // 2. 测试空 token 验证失败
        let result = validate_websocket_jwt_token("", TEST_JWT_SECRET).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), WebSocketAuthError::TokenMissing);
    }
}
