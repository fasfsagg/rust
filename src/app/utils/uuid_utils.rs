//! UUID 工具函数
//!
//! 提供统一的 UUID 解析和验证功能，避免在控制器中重复相同的逻辑。

use crate::error::{AppError, Result};
use sea_orm::prelude::Uuid;

/// 解析字符串为 UUID
///
/// 这个函数提供了统一的 UUID 解析逻辑，包含详细的错误处理。
/// 
/// # 参数
/// * `uuid_str` - 要解析的 UUID 字符串
/// 
/// # 返回值
/// * `Ok(Uuid)` - 解析成功时返回 UUID
/// * `Err(AppError)` - 解析失败时返回应用错误
/// 
/// # 示例
/// ```rust
/// use crate::app::utils::parse_uuid_string;
/// 
/// let uuid = parse_uuid_string("550e8400-e29b-41d4-a716-446655440000")?;
/// ```
pub fn parse_uuid_string(uuid_str: &str) -> Result<Uuid> {
    tracing::debug!(uuid_str = %uuid_str, "开始解析 UUID 字符串");
    
    Uuid::parse_str(uuid_str).map_err(|_| {
        tracing::warn!(uuid_str = %uuid_str, "UUID 解析失败，格式无效");
        AppError::BadRequest(format!("无效的 UUID 格式: {}", uuid_str))
    })
}

/// 解析用户ID字符串为 UUID
///
/// 专门用于解析用户ID的函数，提供更具体的错误信息。
/// 
/// # 参数
/// * `user_id_str` - 用户ID字符串
/// 
/// # 返回值
/// * `Ok(Uuid)` - 解析成功时返回用户 UUID
/// * `Err(AppError)` - 解析失败时返回应用错误
pub fn parse_user_id(user_id_str: &str) -> Result<Uuid> {
    tracing::debug!(user_id = %user_id_str, "开始解析用户ID");
    
    Uuid::parse_str(user_id_str).map_err(|_| {
        tracing::warn!(user_id = %user_id_str, "用户ID解析失败，格式无效");
        AppError::BadRequest(format!("无效的用户ID格式: {}", user_id_str))
    })
}

/// 验证 UUID 是否有效
///
/// 检查给定的字符串是否为有效的 UUID 格式，但不进行解析。
/// 
/// # 参数
/// * `uuid_str` - 要验证的 UUID 字符串
/// 
/// # 返回值
/// * `true` - UUID 格式有效
/// * `false` - UUID 格式无效
pub fn is_valid_uuid(uuid_str: &str) -> bool {
    Uuid::parse_str(uuid_str).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_uuid() {
        let valid_uuid = "550e8400-e29b-41d4-a716-446655440000";
        let result = parse_uuid_string(valid_uuid);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_invalid_uuid() {
        let invalid_uuid = "invalid-uuid";
        let result = parse_uuid_string(invalid_uuid);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_user_id_valid() {
        let valid_user_id = "550e8400-e29b-41d4-a716-446655440000";
        let result = parse_user_id(valid_user_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_valid_uuid() {
        assert!(is_valid_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!is_valid_uuid("invalid-uuid"));
        assert!(!is_valid_uuid(""));
    }
}
