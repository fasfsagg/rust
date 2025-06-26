//! `auth_middleware.rs`
//!
//! 【JWT 认证中间件模块】
//! 这个模块实现了 JWT 认证中间件，用于保护需要认证的路由。
//! 它验证请求头中的 JWT 令牌，并将认证后的用户信息注入到请求中。
//!
//! ## 核心功能
//! - **JWT 验证**: 从 Authorization 头中提取并验证 JWT 令牌
//! - **用户身份注入**: 将验证成功的用户信息注入到请求扩展中
//! - **错误处理**: 处理各种认证失败情况
//!
//! ## 安全特性
//! - 验证 JWT 签名和有效期
//! - 统一的错误响应格式
//! - 防止令牌重放攻击（通过过期时间）
//!
//! ## 设计原则
//! - **单一职责**: 专注于 JWT 认证逻辑
//! - **可复用**: 可以应用到任何需要认证的路由
//! - **错误处理**: 统一的错误处理和返回类型

use axum::{
    extract::Request,
    http::{ header::SEC_WEBSOCKET_PROTOCOL, StatusCode, HeaderMap, Uri },
    middleware::Next,
    response::Response,
};
use crate::app::utils::{ Claims, JwtUtils, AuthService };
use crate::app::middleware::security_audit::{
    log_authentication_success,
    log_authentication_failure,
    log_token_validation,
};

// Claims 结构体现在从 utils 模块导入，避免重复定义

/// 认证用户信息结构体
/// 用于在请求扩展中存储认证后的用户信息
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// 用户 ID
    pub user_id: String,
    /// 用户名
    pub username: String,
}

impl From<Claims> for AuthenticatedUser {
    fn from(claims: Claims) -> Self {
        Self {
            user_id: claims.sub,
            username: claims.username,
        }
    }
}

/// 创建 JWT 认证中间件
///
/// 这个函数接受 JWT 密钥作为参数，并返回一个中间件函数。
/// 这样可以在创建中间件时注入 JWT 密钥。
///
/// # 参数
/// - `jwt_secret`: JWT 签名密钥
///
/// # 返回
/// 返回一个可以用于验证 JWT 令牌的中间件函数
pub fn create_jwt_auth_middleware(
    jwt_secret: String
) -> impl (Fn(
    Request,
    Next
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, StatusCode>> + Send>>) +
    Clone {
    move |req, next| {
        let secret = jwt_secret.clone();
        Box::pin(async move { jwt_auth_impl(req, next, secret).await })
    }
}

/// JWT 认证中间件的核心实现（使用统一的 AuthService）
///
/// 这个函数使用统一的 AuthService 验证请求头中的 JWT 令牌，
/// 并将认证后的用户信息注入到请求的扩展中，供下游的处理器使用。
///
/// # 工作流程
/// 1. 使用 AuthService 从 HTTP 请求头中提取并验证 JWT
/// 2. 验证成功后，将用户信息存入请求的 `extensions` 中
/// 3. 调用下一个中间件或处理器
///
/// # 参数
/// - `req`: HTTP 请求对象
/// - `next`: 下一个中间件或处理器
/// - `jwt_secret`: JWT 签名密钥
///
/// # 返回
/// 成功时调用下游处理器，失败时返回 401 Unauthorized
async fn jwt_auth_impl(
    mut req: Request,
    next: Next,
    jwt_secret: String
) -> Result<Response, StatusCode> {
    println!("AUTH_MIDDLEWARE: 开始验证 JWT 令牌");

    // 提取客户端信息用于安全审计
    let headers = req.headers();
    let client_ip = extract_client_ip_from_headers(headers);
    let user_agent = headers.get("user-agent").and_then(|h| h.to_str().ok());

    // 使用统一的 AuthService 进行认证
    let auth_service = AuthService::new(jwt_secret);
    let claims = match auth_service.authenticate_http_request(req.headers()) {
        Ok(claims) => {
            // 记录令牌验证成功事件
            log_token_validation(
                true,
                Some(&claims.sub),
                Some(&claims.username),
                None,
                client_ip.as_deref(),
                user_agent
            );

            // 记录认证成功事件
            log_authentication_success(
                &claims.sub,
                &claims.username,
                client_ip.as_deref(),
                user_agent
            );

            claims
        }
        Err(err) => {
            println!("AUTH_MIDDLEWARE: JWT 验证失败: {:?}", err);

            // 记录令牌验证失败事件
            let failure_reason = format!("{:?}", err);
            log_token_validation(
                false,
                None,
                None,
                Some(&failure_reason),
                client_ip.as_deref(),
                user_agent
            );

            // 记录认证失败事件
            log_authentication_failure(
                None, // 无法从失败的令牌中获取用户名
                &failure_reason,
                client_ip.as_deref(),
                user_agent
            );

            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // 将认证用户信息注入到请求扩展中
    let authenticated_user = AuthenticatedUser::from(claims);
    println!(
        "AUTH_MIDDLEWARE: JWT 验证成功，用户: {} (ID: {})",
        authenticated_user.username,
        authenticated_user.user_id
    );

    req.extensions_mut().insert(authenticated_user);

    // 调用下一个中间件或处理器
    Ok(next.run(req).await)
}

/// 从HTTP头部提取客户端IP地址
///
/// 【功能】：尝试从多个可能的HTTP头部中提取真实的客户端IP地址
/// 优先级：X-Forwarded-For > X-Real-IP > X-Client-IP
///
/// # 参数
/// * `headers` - HTTP请求头部
///
/// # 返回值
/// * `Option<String>` - 客户端IP地址（如果找到）
fn extract_client_ip_from_headers(headers: &HeaderMap) -> Option<String> {
    // 尝试从 X-Forwarded-For 头部获取（最常见的代理头部）
    if let Some(forwarded_for) = headers.get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded_for.to_str() {
            // X-Forwarded-For 可能包含多个IP，取第一个
            if let Some(first_ip) = forwarded_str.split(',').next() {
                return Some(first_ip.trim().to_string());
            }
        }
    }

    // 尝试从 X-Real-IP 头部获取
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            return Some(ip_str.to_string());
        }
    }

    // 尝试从 X-Client-IP 头部获取
    if let Some(client_ip) = headers.get("x-client-ip") {
        if let Ok(ip_str) = client_ip.to_str() {
            return Some(ip_str.to_string());
        }
    }

    // 如果都没有找到，返回None
    None
}

// --- WebSocket JWT 认证相关功能 ---

// WebSocket 认证错误类型现在使用统一的 JwtError
pub use crate::app::utils::JwtError as WebSocketAuthError;

/// 从 WebSocket 请求中提取 JWT token（保持向后兼容）
///
/// 支持两种方式：
/// 1. 查询参数: `ws://localhost:3000/ws?token=<jwt-token>`
/// 2. Sec-WebSocket-Protocol 头: `access_token.<jwt-token>`
///
/// 注意：这个函数保持向后兼容，新代码建议使用 AuthService
pub fn extract_websocket_token(uri: &Uri, headers: &HeaderMap) -> Option<String> {
    let auth_service = AuthService::new("dummy".to_string()); // 只用于提取，不需要真实密钥
    match auth_service.authenticate_websocket_request(uri, headers) {
        Ok(_) => {
            // 如果认证成功，说明 token 存在，我们需要重新提取它
            // 这里为了保持向后兼容，我们仍然使用原来的逻辑
            extract_websocket_token_legacy(uri, headers)
        }
        Err(_) => extract_websocket_token_legacy(uri, headers), // 即使失败也尝试提取
    }
}

/// 传统的 WebSocket token 提取逻辑（内部使用）
fn extract_websocket_token_legacy(uri: &Uri, headers: &HeaderMap) -> Option<String> {
    // 1. 尝试从查询参数中提取 token
    if let Some(query) = uri.query() {
        if let Some(token) = extract_token_from_query(query) {
            return Some(token);
        }
    }

    // 2. 尝试从 Sec-WebSocket-Protocol 头中提取 token
    extract_token_from_protocol_header(headers)
}

/// 从查询参数中提取 JWT token
fn extract_token_from_query(query: &str) -> Option<String> {
    use std::collections::HashMap;
    let params: HashMap<String, String> = serde_urlencoded::from_str(query).ok()?;
    params.get("token").cloned()
}

/// 从 Sec-WebSocket-Protocol 头中提取 JWT token
fn extract_token_from_protocol_header(headers: &HeaderMap) -> Option<String> {
    let protocol_header = headers.get(SEC_WEBSOCKET_PROTOCOL)?;
    let protocol_str = protocol_header.to_str().ok()?;

    // 支持格式: "access_token.<jwt-token>" 或 "chat, access_token.<jwt-token>"
    for protocol in protocol_str.split(',') {
        let protocol = protocol.trim();
        if protocol.starts_with("access_token.") {
            return Some(protocol.strip_prefix("access_token.").unwrap().to_string());
        }
    }
    None
}

/// 验证 WebSocket JWT token（使用统一的 AuthService）
pub async fn validate_websocket_jwt_token(
    token: &str,
    jwt_secret: &str
) -> Result<Claims, WebSocketAuthError> {
    let jwt_utils = JwtUtils::new(jwt_secret.to_string());
    jwt_utils.validate_token(token)
}

/// 验证 WebSocket 请求的完整认证（推荐使用）
pub async fn authenticate_websocket_request(
    uri: &Uri,
    headers: &HeaderMap,
    jwt_secret: &str
) -> Result<Claims, WebSocketAuthError> {
    let auth_service = AuthService::new(jwt_secret.to_string());
    auth_service.authenticate_websocket_request(uri, headers)
}

// --- 单元测试 ---
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::utils::JwtError;

    /// 测试 JWT 声明结构体的序列化和反序列化
    #[test]
    fn test_claims_serialization() {
        let claims = Claims {
            sub: "user123".to_string(),
            username: "testuser".to_string(),
            exp: 1234567890,
            iat: 1234567800,
        };

        // 测试序列化
        let serialized = serde_json::to_string(&claims).unwrap();
        assert!(serialized.contains("user123"));
        assert!(serialized.contains("testuser"));

        // 测试反序列化
        let deserialized: Claims = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.sub, "user123");
        assert_eq!(deserialized.username, "testuser");
        assert_eq!(deserialized.exp, 1234567890);
        assert_eq!(deserialized.iat, 1234567800);
    }

    /// 测试 AuthenticatedUser 从 Claims 的转换
    #[test]
    fn test_authenticated_user_from_claims() {
        let claims = Claims {
            sub: "user456".to_string(),
            username: "anotheruser".to_string(),
            exp: 1234567890,
            iat: 1234567800,
        };

        let auth_user = AuthenticatedUser::from(claims);
        assert_eq!(auth_user.user_id, "user456");
        assert_eq!(auth_user.username, "anotheruser");
    }

    /// 测试 JWT 令牌的创建和验证（使用 JwtUtils）
    #[test]
    fn test_jwt_token_creation_and_validation() {
        let jwt_utils = JwtUtils::new("test-secret-key".to_string());

        // 创建 JWT 令牌
        let token = jwt_utils.create_token("test_user", "testuser", 1).unwrap();

        // 验证令牌不为空
        assert!(!token.is_empty());
        assert!(token.contains('.'));

        // 验证令牌可以被解码
        let claims = jwt_utils.validate_token(&token).unwrap();
        assert_eq!(claims.sub, "test_user");
        assert_eq!(claims.username, "testuser");
    }

    /// 测试过期令牌的验证失败（使用 JwtUtils）
    #[test]
    fn test_expired_token_validation_fails() {
        let jwt_utils = JwtUtils::new("test-secret-key".to_string());

        // 创建一个已过期的令牌
        let expired_token = jwt_utils.create_expired_test_token("test_user", "testuser");

        // 验证过期令牌应该失败
        let result = jwt_utils.validate_token(&expired_token);
        assert_eq!(result, Err(JwtError::TokenExpired));
    }

    /// 测试错误密钥的验证失败（使用 JwtUtils）
    #[test]
    fn test_wrong_secret_validation_fails() {
        let jwt_utils_correct = JwtUtils::new("correct-secret".to_string());
        let jwt_utils_wrong = JwtUtils::new("wrong-secret".to_string());

        // 用正确密钥创建令牌
        let token = jwt_utils_correct.create_token("test_user", "testuser", 1).unwrap();

        // 用错误密钥验证应该失败
        let result = jwt_utils_wrong.validate_token(&token);
        assert_eq!(result, Err(JwtError::TokenInvalid));
    }
}
