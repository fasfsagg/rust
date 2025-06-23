//! WebSocket JWT 认证单元测试
//!
//! 本模块包含 WebSocket 连接身份验证的全面单元测试，遵循 TDD 原则。
//! 测试覆盖以下场景：
//! - 有效 JWT token 验证成功
//! - 过期 JWT token 被拒绝
//! - 无效 JWT token 被拒绝
//! - 缺失 JWT token 被拒绝
//! - 不同的 token 传递方式（查询参数、Sec-WebSocket-Protocol 头）

use axum::{ http::{ header::SEC_WEBSOCKET_PROTOCOL, HeaderMap, HeaderValue } };
use jsonwebtoken::{ encode, decode, EncodingKey, DecodingKey, Header, Validation };
use chrono::{ Utc, Duration };

// 使用 utils 模块中定义的 Claims 类型
use axum_tutorial::app::utils::Claims;

// WebSocket 连接查询参数（暂时不需要，已在中间件中实现）

/// 测试用的 JWT 密钥
const TEST_JWT_SECRET: &str = "test-websocket-jwt-secret-key-for-unit-tests";

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

/// 创建无效签名的 JWT token
fn create_invalid_jwt_token(user_id: &str, username: &str) -> String {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        iat: now.timestamp(),
        exp: (now + Duration::hours(1)).timestamp(),
    };

    // 使用错误的密钥签名
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret("wrong-secret-key".as_ref())
    ).expect("Failed to create invalid JWT token")
}

/// WebSocket JWT 验证函数（使用实际实现）
async fn validate_websocket_jwt_token(token: &str) -> Result<Claims, WebSocketAuthError> {
    // 使用实际的验证逻辑
    axum_tutorial::app::middleware::auth_middleware::validate_websocket_jwt_token(
        token,
        TEST_JWT_SECRET
    ).await
}

// 使用中间件中定义的错误类型
use axum_tutorial::app::middleware::auth_middleware::WebSocketAuthError;

// 使用中间件中定义的函数
use axum_tutorial::app::middleware::auth_middleware::extract_websocket_token;

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试创建有效 JWT token
    #[test]
    fn test_create_valid_jwt_token() {
        let token = create_valid_jwt_token("user123", "testuser");
        assert!(!token.is_empty());

        // 验证 token 可以被正确解码
        let token_data = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(TEST_JWT_SECRET.as_ref()),
            &Validation::default()
        );

        assert!(token_data.is_ok());
        let claims = token_data.unwrap().claims;
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.username, "testuser");
    }

    /// 测试创建过期 JWT token
    #[test]
    fn test_create_expired_jwt_token() {
        let token = create_expired_jwt_token("user123", "testuser");
        assert!(!token.is_empty());

        // 验证过期 token 被拒绝
        let token_data = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(TEST_JWT_SECRET.as_ref()),
            &Validation::default()
        );

        assert!(token_data.is_err());
        // 应该是过期错误
        assert!(token_data.unwrap_err().to_string().contains("ExpiredSignature"));
    }

    /// 测试创建无效签名的 JWT token
    #[test]
    fn test_create_invalid_jwt_token() {
        let token = create_invalid_jwt_token("user123", "testuser");
        assert!(!token.is_empty());

        // 验证无效签名的 token 被拒绝
        let token_data = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(TEST_JWT_SECRET.as_ref()),
            &Validation::default()
        );

        assert!(token_data.is_err());
        // 应该是签名验证错误
        assert!(token_data.unwrap_err().to_string().contains("InvalidSignature"));
    }

    /// 测试从 WebSocket 请求中提取 token
    #[test]
    fn test_extract_websocket_token() {
        use axum::http::Uri;

        // 测试从查询参数中提取 token
        let uri: Uri = "ws://localhost:3000/ws?token=eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9"
            .parse()
            .unwrap();
        let headers = HeaderMap::new();
        let token = extract_websocket_token(&uri, &headers);
        assert_eq!(token, Some("eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9".to_string()));

        // 测试从 Sec-WebSocket-Protocol 头中提取 token
        let uri: Uri = "ws://localhost:3000/ws".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            SEC_WEBSOCKET_PROTOCOL,
            HeaderValue::from_static("access_token.eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9")
        );
        let token = extract_websocket_token(&uri, &headers);
        assert_eq!(token, Some("eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9".to_string()));

        // 测试多个协议的情况
        headers.insert(
            SEC_WEBSOCKET_PROTOCOL,
            HeaderValue::from_static("chat, access_token.eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9")
        );
        let token = extract_websocket_token(&uri, &headers);
        assert_eq!(token, Some("eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9".to_string()));

        // 测试缺失 token
        let uri: Uri = "ws://localhost:3000/ws".parse().unwrap();
        let headers = HeaderMap::new();
        let token = extract_websocket_token(&uri, &headers);
        assert_eq!(token, None);
    }

    /// 测试 WebSocket JWT 验证 - 有效 token
    #[tokio::test]
    async fn test_validate_websocket_jwt_valid_token() {
        let token = create_valid_jwt_token("user123", "testuser");

        let result = validate_websocket_jwt_token(&token).await;
        assert!(result.is_ok());
        let claims = result.unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.username, "testuser");
    }

    /// 测试 WebSocket JWT 验证 - 过期 token
    #[tokio::test]
    async fn test_validate_websocket_jwt_expired_token() {
        let token = create_expired_jwt_token("user123", "testuser");

        let result = validate_websocket_jwt_token(&token).await;
        assert_eq!(result, Err(WebSocketAuthError::TokenExpired));
    }

    /// 测试 WebSocket JWT 验证 - 无效 token
    #[tokio::test]
    async fn test_validate_websocket_jwt_invalid_token() {
        let token = create_invalid_jwt_token("user123", "testuser");

        let result = validate_websocket_jwt_token(&token).await;
        assert_eq!(result, Err(WebSocketAuthError::TokenInvalid));
    }

    /// 测试 WebSocket JWT 验证 - 缺失 token
    #[tokio::test]
    async fn test_validate_websocket_jwt_missing_token() {
        let empty_token = "";

        let result = validate_websocket_jwt_token(empty_token).await;
        assert_eq!(result, Err(WebSocketAuthError::TokenMissing));
    }
}
