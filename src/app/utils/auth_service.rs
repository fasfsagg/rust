//! 统一身份验证服务
//!
//! 本模块提供统一的身份验证服务，避免 HTTP 和 WebSocket 认证逻辑的重复。
//! 
//! ## 核心功能
//! - **统一的 Token 提取**：支持多种 Token 传递方式
//! - **统一的 Token 验证**：HTTP 和 WebSocket 共用验证逻辑
//! - **统一的错误处理**：一致的错误类型和处理
//! - **可扩展设计**：便于添加新的认证方式
//!
//! ## 设计原则
//! - **DRY 原则**：避免重复代码
//! - **单一职责**：专注于身份验证
//! - **策略模式**：支持多种 Token 提取策略

use axum::http::{HeaderMap, Uri, header::AUTHORIZATION};
use crate::app::utils::{Claims, JwtError, JwtUtils};

/// Token 提取方法枚举
#[derive(Debug, Clone, PartialEq)]
pub enum TokenExtractionMethod {
    /// HTTP Authorization Bearer 头
    HttpBearer,
    /// WebSocket 查询参数
    WebSocketQuery,
    /// WebSocket Sec-WebSocket-Protocol 头
    WebSocketProtocol,
}

/// 统一身份验证服务
/// 
/// 提供 HTTP 和 WebSocket 通用的身份验证功能
pub struct AuthService {
    jwt_utils: JwtUtils,
}

impl AuthService {
    /// 创建新的身份验证服务实例
    pub fn new(jwt_secret: String) -> Self {
        Self {
            jwt_utils: JwtUtils::new(jwt_secret),
        }
    }

    /// 从 HTTP 请求中提取并验证 JWT token
    /// 
    /// # 参数
    /// - `headers`: HTTP 请求头
    /// 
    /// # 返回
    /// 成功时返回解析后的 Claims，失败时返回错误
    pub fn authenticate_http_request(&self, headers: &HeaderMap) -> Result<Claims, JwtError> {
        // 1. 提取 token
        let token = self.extract_token_from_http_headers(headers)?;
        
        // 2. 验证 token
        self.jwt_utils.validate_token(&token)
    }

    /// 从 WebSocket 请求中提取并验证 JWT token
    /// 
    /// # 参数
    /// - `uri`: WebSocket 请求 URI
    /// - `headers`: WebSocket 请求头
    /// 
    /// # 返回
    /// 成功时返回解析后的 Claims，失败时返回错误
    pub fn authenticate_websocket_request(
        &self, 
        uri: &Uri, 
        headers: &HeaderMap
    ) -> Result<Claims, JwtError> {
        // 1. 提取 token
        let token = self.extract_token_from_websocket_request(uri, headers)?;
        
        // 2. 验证 token
        self.jwt_utils.validate_token(&token)
    }

    /// 从 HTTP 请求头中提取 JWT token
    fn extract_token_from_http_headers(&self, headers: &HeaderMap) -> Result<String, JwtError> {
        // 从 Authorization 头中提取 Bearer token
        let auth_header = headers
            .get(AUTHORIZATION)
            .and_then(|header| header.to_str().ok())
            .ok_or(JwtError::TokenMissing)?;

        if !auth_header.starts_with("Bearer ") {
            return Err(JwtError::TokenInvalid);
        }

        Ok(auth_header[7..].to_string()) // 跳过 "Bearer " 前缀
    }

    /// 从 WebSocket 请求中提取 JWT token
    fn extract_token_from_websocket_request(
        &self, 
        uri: &Uri, 
        headers: &HeaderMap
    ) -> Result<String, JwtError> {
        // 1. 尝试从查询参数中提取 token
        if let Some(query) = uri.query() {
            if let Some(token) = self.extract_token_from_query(query) {
                return Ok(token);
            }
        }

        // 2. 尝试从 Sec-WebSocket-Protocol 头中提取 token
        if let Some(token) = self.extract_token_from_protocol_header(headers) {
            return Ok(token);
        }

        Err(JwtError::TokenMissing)
    }

    /// 从查询参数中提取 JWT token
    fn extract_token_from_query(&self, query: &str) -> Option<String> {
        use std::collections::HashMap;
        let params: HashMap<String, String> = serde_urlencoded::from_str(query).ok()?;
        params.get("token").cloned()
    }

    /// 从 Sec-WebSocket-Protocol 头中提取 JWT token
    fn extract_token_from_protocol_header(&self, headers: &HeaderMap) -> Option<String> {
        use axum::http::header::SEC_WEBSOCKET_PROTOCOL;
        
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

    /// 创建 JWT token（用于登录等场景）
    pub fn create_token(
        &self,
        user_id: &str,
        username: &str,
        expires_in_hours: i64,
    ) -> Result<String, JwtError> {
        self.jwt_utils.create_token(user_id, username, expires_in_hours)
    }

    /// 获取底层的 JwtUtils 实例（用于测试等特殊场景）
    pub fn jwt_utils(&self) -> &JwtUtils {
        &self.jwt_utils
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderValue, header::SEC_WEBSOCKET_PROTOCOL};

    const TEST_SECRET: &str = "test-auth-service-secret";

    fn create_auth_service() -> AuthService {
        AuthService::new(TEST_SECRET.to_string())
    }

    #[test]
    fn test_http_authentication_success() {
        let auth_service = create_auth_service();
        
        // 创建测试 token
        let token = auth_service.create_token("user123", "testuser", 1).unwrap();
        
        // 创建 HTTP 请求头
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", token)).unwrap()
        );
        
        // 测试认证
        let claims = auth_service.authenticate_http_request(&headers).unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.username, "testuser");
    }

    #[test]
    fn test_http_authentication_missing_header() {
        let auth_service = create_auth_service();
        let headers = HeaderMap::new();
        
        let result = auth_service.authenticate_http_request(&headers);
        assert_eq!(result, Err(JwtError::TokenMissing));
    }

    #[test]
    fn test_websocket_authentication_query_param() {
        let auth_service = create_auth_service();
        
        // 创建测试 token
        let token = auth_service.create_token("user456", "wsuser", 1).unwrap();
        
        // 创建 WebSocket 请求
        let uri: Uri = format!("ws://localhost:3000/ws?token={}", token).parse().unwrap();
        let headers = HeaderMap::new();
        
        // 测试认证
        let claims = auth_service.authenticate_websocket_request(&uri, &headers).unwrap();
        assert_eq!(claims.sub, "user456");
        assert_eq!(claims.username, "wsuser");
    }

    #[test]
    fn test_websocket_authentication_protocol_header() {
        let auth_service = create_auth_service();
        
        // 创建测试 token
        let token = auth_service.create_token("user789", "protocoluser", 1).unwrap();
        
        // 创建 WebSocket 请求
        let uri: Uri = "ws://localhost:3000/ws".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            SEC_WEBSOCKET_PROTOCOL,
            HeaderValue::from_str(&format!("access_token.{}", token)).unwrap()
        );
        
        // 测试认证
        let claims = auth_service.authenticate_websocket_request(&uri, &headers).unwrap();
        assert_eq!(claims.sub, "user789");
        assert_eq!(claims.username, "protocoluser");
    }

    #[test]
    fn test_websocket_authentication_missing_token() {
        let auth_service = create_auth_service();
        
        let uri: Uri = "ws://localhost:3000/ws".parse().unwrap();
        let headers = HeaderMap::new();
        
        let result = auth_service.authenticate_websocket_request(&uri, &headers);
        assert_eq!(result, Err(JwtError::TokenMissing));
    }

    #[test]
    fn test_expired_token_authentication() {
        let auth_service = create_auth_service();
        
        // 创建过期 token
        let expired_token = auth_service.jwt_utils().create_expired_test_token("user123", "testuser");
        
        // 测试 HTTP 认证
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", expired_token)).unwrap()
        );
        
        let result = auth_service.authenticate_http_request(&headers);
        assert_eq!(result, Err(JwtError::TokenExpired));
    }
}
