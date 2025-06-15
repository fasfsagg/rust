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
    http::{ header::AUTHORIZATION, StatusCode },
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{ decode, DecodingKey, Validation };
use serde::{ Deserialize, Serialize };

/// JWT 声明结构体
/// 用于解析和验证 JWT 令牌中的用户信息
///
/// 注意：这个结构体必须与 auth_service.rs 中的 Claims 结构体保持一致
#[derive(Debug, Serialize, Deserialize, Clone)]
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

/// JWT 认证中间件的核心实现
///
/// 这个函数验证请求头中的 JWT 令牌，并将认证后的用户信息
/// 注入到请求的扩展中，供下游的处理器使用。
///
/// # 工作流程
/// 1. 从 `Authorization: Bearer <token>` 请求头中提取 JWT
/// 2. 使用 `jsonwebtoken::decode` 验证令牌的签名和有效期
/// 3. 验证成功后，将用户信息存入请求的 `extensions` 中
/// 4. 调用下一个中间件或处理器
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

    // 1. 从请求头中提取 Authorization 头
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let auth_header = match auth_header {
        Some(header) => header,
        None => {
            println!("AUTH_MIDDLEWARE: 缺少 Authorization 头");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // 2. 检查是否是 Bearer 令牌格式
    if !auth_header.starts_with("Bearer ") {
        println!("AUTH_MIDDLEWARE: Authorization 头格式错误，应为 'Bearer <token>'");
        return Err(StatusCode::UNAUTHORIZED);
    }

    // 3. 提取令牌部分
    let token = &auth_header[7..]; // 跳过 "Bearer " 前缀

    // 4. 验证 JWT 令牌
    let token_data = match
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(jwt_secret.as_ref()),
            &Validation::default()
        )
    {
        Ok(data) => data,
        Err(err) => {
            println!("AUTH_MIDDLEWARE: JWT 验证失败: {:?}", err);
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // 5. 将认证用户信息注入到请求扩展中
    let authenticated_user = AuthenticatedUser::from(token_data.claims);
    println!(
        "AUTH_MIDDLEWARE: JWT 验证成功，用户: {} (ID: {})",
        authenticated_user.username,
        authenticated_user.user_id
    );

    req.extensions_mut().insert(authenticated_user);

    // 6. 调用下一个中间件或处理器
    Ok(next.run(req).await)
}

// --- 单元测试 ---
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{ Duration, Utc };
    use jsonwebtoken::{ encode, EncodingKey, Header };

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

    /// 测试 JWT 令牌的创建和验证
    #[test]
    fn test_jwt_token_creation_and_validation() {
        let secret = "test-secret-key";
        let now = Utc::now();

        let claims = Claims {
            sub: "test_user".to_string(),
            username: "testuser".to_string(),
            exp: (now + Duration::hours(1)).timestamp(),
            iat: now.timestamp(),
        };

        // 创建 JWT 令牌
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_ref())
        ).unwrap();

        // 验证令牌不为空
        assert!(!token.is_empty());
        assert!(token.contains('.'));

        // 验证令牌可以被解码
        let decoded = jsonwebtoken
            ::decode::<Claims>(
                &token,
                &jsonwebtoken::DecodingKey::from_secret(secret.as_ref()),
                &jsonwebtoken::Validation::default()
            )
            .unwrap();

        assert_eq!(decoded.claims.sub, "test_user");
        assert_eq!(decoded.claims.username, "testuser");
    }

    /// 测试过期令牌的验证失败
    #[test]
    fn test_expired_token_validation_fails() {
        let secret = "test-secret-key";
        let now = Utc::now();

        // 创建一个已过期的令牌
        let claims = Claims {
            sub: "test_user".to_string(),
            username: "testuser".to_string(),
            exp: (now - Duration::hours(1)).timestamp(), // 1小时前过期
            iat: (now - Duration::hours(2)).timestamp(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_ref())
        ).unwrap();

        // 验证过期令牌应该失败
        let result = jsonwebtoken::decode::<Claims>(
            &token,
            &jsonwebtoken::DecodingKey::from_secret(secret.as_ref()),
            &jsonwebtoken::Validation::default()
        );

        assert!(result.is_err());
    }

    /// 测试错误密钥的验证失败
    #[test]
    fn test_wrong_secret_validation_fails() {
        let secret = "correct-secret";
        let wrong_secret = "wrong-secret";
        let now = Utc::now();

        let claims = Claims {
            sub: "test_user".to_string(),
            username: "testuser".to_string(),
            exp: (now + Duration::hours(1)).timestamp(),
            iat: now.timestamp(),
        };

        // 用正确密钥创建令牌
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_ref())
        ).unwrap();

        // 用错误密钥验证应该失败
        let result = jsonwebtoken::decode::<Claims>(
            &token,
            &jsonwebtoken::DecodingKey::from_secret(wrong_secret.as_ref()),
            &jsonwebtoken::Validation::default()
        );

        assert!(result.is_err());
    }
}
