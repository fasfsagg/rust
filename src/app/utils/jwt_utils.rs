//! JWT 工具模块
//!
//! 本模块提供统一的 JWT 相关工具函数，避免代码重复。
//! 包含 JWT token 的创建、验证、解析等通用功能。
//!
//! ## 核心功能
//! - **统一的 JWT 验证逻辑**：HTTP 和 WebSocket 共用
//! - **Token 创建工具**：测试和生产环境共用
//! - **错误处理统一**：统一的错误类型和处理
//!
//! ## 设计原则
//! - **DRY 原则**：避免重复代码
//! - **单一职责**：专注于 JWT 相关操作
//! - **可测试性**：便于单元测试

use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

/// JWT 声明结构体
///
/// 这是项目中统一使用的 JWT Claims 结构体，
/// 确保所有模块使用相同的字段定义
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Claims {
    /// 用户 ID (Subject)
    pub sub: String,
    /// 用户名
    pub username: String,
    /// 令牌过期时间（Unix 时间戳）
    pub exp: i64,
    /// 令牌签发时间（Unix 时间戳）
    pub iat: i64,
}

/// JWT 验证错误类型
#[derive(Debug, PartialEq, Clone)]
pub enum JwtError {
    /// Token 缺失
    TokenMissing,
    /// Token 无效（签名错误、格式错误等）
    TokenInvalid,
    /// Token 已过期
    TokenExpired,
    /// Token 创建失败
    TokenCreationFailed(String),
}

/// JWT 工具结构体
///
/// 封装 JWT 相关操作，提供统一的接口
pub struct JwtUtils {
    secret: String,
}

impl JwtUtils {
    /// 创建新的 JWT 工具实例
    pub fn new(secret: String) -> Self {
        Self { secret }
    }

    /// 创建 JWT token
    ///
    /// # 参数
    /// - `user_id`: 用户 ID
    /// - `username`: 用户名
    /// - `expires_in_hours`: 过期时间（小时）
    ///
    /// # 返回
    /// 成功时返回 JWT token 字符串，失败时返回错误
    pub fn create_token(
        &self,
        user_id: &str,
        username: &str,
        expires_in_hours: i64,
    ) -> Result<String, JwtError> {
        let now = Utc::now();
        let claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            iat: now.timestamp(),
            exp: (now + Duration::hours(expires_in_hours)).timestamp(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_ref()),
        )
        .map_err(|e| JwtError::TokenCreationFailed(e.to_string()))
    }

    /// 验证 JWT token
    ///
    /// # 参数
    /// - `token`: JWT token 字符串
    ///
    /// # 返回
    /// 成功时返回解析后的 Claims，失败时返回错误
    pub fn validate_token(&self, token: &str) -> Result<Claims, JwtError> {
        if token.is_empty() {
            return Err(JwtError::TokenMissing);
        }

        match decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_ref()),
            &Validation::default(),
        ) {
            Ok(token_data) => Ok(token_data.claims),
            Err(err) => match err.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => Err(JwtError::TokenExpired),
                _ => Err(JwtError::TokenInvalid),
            },
        }
    }

    /// 从 Authorization 头中提取 Bearer token
    ///
    /// # 参数
    /// - `auth_header`: Authorization 头的值
    ///
    /// # 返回
    /// 成功时返回提取的 token，失败时返回 None
    pub fn extract_bearer_token(auth_header: &str) -> Option<String> {
        auth_header
            .strip_prefix("Bearer ")
            .map(|stripped| stripped.to_string())
    }

    /// 创建测试用的 JWT token
    ///
    /// 这个函数专门用于测试，创建短期有效的 token
    ///
    /// # 参数
    /// - `user_id`: 用户 ID
    /// - `username`: 用户名
    ///
    /// # 返回
    /// 测试用的 JWT token
    pub fn create_test_token(&self, user_id: &str, username: &str) -> String {
        self.create_token(user_id, username, 1)
            .expect("Failed to create test token")
    }

    /// 创建过期的测试 token
    ///
    /// 这个函数专门用于测试过期 token 的场景
    ///
    /// # 参数
    /// - `user_id`: 用户 ID
    /// - `username`: 用户名
    ///
    /// # 返回
    /// 已过期的 JWT token
    pub fn create_expired_test_token(&self, user_id: &str, username: &str) -> String {
        let now = Utc::now();
        let claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            iat: (now - Duration::hours(2)).timestamp(), // 2小时前签发
            exp: (now - Duration::hours(1)).timestamp(), // 1小时前过期
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_ref()),
        )
        .expect("Failed to create expired test token")
    }

    /// 创建无效签名的测试 token
    ///
    /// 这个函数专门用于测试无效签名的场景
    ///
    /// # 参数
    /// - `user_id`: 用户 ID
    /// - `username`: 用户名
    ///
    /// # 返回
    /// 使用错误密钥签名的 JWT token
    pub fn create_invalid_test_token(&self, user_id: &str, username: &str) -> String {
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
            &EncodingKey::from_secret("wrong-secret-key".as_ref()),
        )
        .expect("Failed to create invalid test token")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-jwt-utils-secret";

    #[test]
    fn test_create_and_validate_token() {
        let jwt_utils = JwtUtils::new(TEST_SECRET.to_string());

        // 创建 token
        let token = jwt_utils.create_token("user123", "testuser", 1).unwrap();
        assert!(!token.is_empty());

        // 验证 token
        let claims = jwt_utils.validate_token(&token).unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.username, "testuser");
    }

    #[test]
    fn test_validate_expired_token() {
        let jwt_utils = JwtUtils::new(TEST_SECRET.to_string());

        let expired_token = jwt_utils.create_expired_test_token("user123", "testuser");
        let result = jwt_utils.validate_token(&expired_token);

        assert_eq!(result, Err(JwtError::TokenExpired));
    }

    #[test]
    fn test_validate_invalid_token() {
        let jwt_utils = JwtUtils::new(TEST_SECRET.to_string());

        let invalid_token = jwt_utils.create_invalid_test_token("user123", "testuser");
        let result = jwt_utils.validate_token(&invalid_token);

        assert_eq!(result, Err(JwtError::TokenInvalid));
    }

    #[test]
    fn test_extract_bearer_token() {
        let auth_header = "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9";
        let token = JwtUtils::extract_bearer_token(auth_header);
        assert_eq!(
            token,
            Some("eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9".to_string())
        );

        let invalid_header = "Basic dXNlcjpwYXNz";
        let token = JwtUtils::extract_bearer_token(invalid_header);
        assert_eq!(token, None);
    }

    #[test]
    fn test_validate_empty_token() {
        let jwt_utils = JwtUtils::new(TEST_SECRET.to_string());
        let result = jwt_utils.validate_token("");
        assert_eq!(result, Err(JwtError::TokenMissing));
    }
}
