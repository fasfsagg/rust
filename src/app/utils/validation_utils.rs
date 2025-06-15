//! 验证工具函数
//!
//! 提供通用的输入验证功能，确保数据的有效性和安全性。

use crate::error::{AppError, Result};

/// 验证字符串长度是否在指定范围内
///
/// # 参数
/// * `value` - 要验证的字符串
/// * `min_len` - 最小长度
/// * `max_len` - 最大长度
/// * `field_name` - 字段名称（用于错误消息）
/// 
/// # 返回值
/// * `Ok(())` - 验证通过
/// * `Err(AppError)` - 验证失败
pub fn validate_string_length(
    value: &str,
    min_len: usize,
    max_len: usize,
    field_name: &str,
) -> Result<()> {
    let len = value.len();
    if len < min_len {
        return Err(AppError::BadRequest(format!(
            "{} 长度不能少于 {} 个字符，当前长度: {}",
            field_name, min_len, len
        )));
    }
    if len > max_len {
        return Err(AppError::BadRequest(format!(
            "{} 长度不能超过 {} 个字符，当前长度: {}",
            field_name, max_len, len
        )));
    }
    Ok(())
}

/// 验证字符串是否非空且不全为空白字符
///
/// # 参数
/// * `value` - 要验证的字符串
/// * `field_name` - 字段名称（用于错误消息）
/// 
/// # 返回值
/// * `Ok(())` - 验证通过
/// * `Err(AppError)` - 验证失败
pub fn validate_not_empty(value: &str, field_name: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(AppError::BadRequest(format!("{} 不能为空", field_name)));
    }
    Ok(())
}

/// 验证任务标题
///
/// 任务标题必须：
/// - 非空
/// - 长度在 1-200 字符之间
/// - 不全为空白字符
pub fn validate_task_title(title: &str) -> Result<()> {
    validate_not_empty(title, "任务标题")?;
    validate_string_length(title, 1, 200, "任务标题")?;
    Ok(())
}

/// 验证任务描述
///
/// 任务描述可以为空，但如果提供则：
/// - 长度不能超过 1000 字符
pub fn validate_task_description(description: &Option<String>) -> Result<()> {
    if let Some(desc) = description {
        validate_string_length(desc, 0, 1000, "任务描述")?;
    }
    Ok(())
}

/// 验证用户名
///
/// 用户名必须：
/// - 非空
/// - 长度在 3-50 字符之间
/// - 只包含字母、数字、下划线和连字符
pub fn validate_username(username: &str) -> Result<()> {
    validate_not_empty(username, "用户名")?;
    validate_string_length(username, 3, 50, "用户名")?;
    
    // 检查字符是否合法
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err(AppError::BadRequest(
            "用户名只能包含字母、数字、下划线和连字符".to_string(),
        ));
    }
    
    Ok(())
}

/// 验证密码强度
///
/// 密码必须：
/// - 长度至少 8 字符
/// - 包含至少一个字母
/// - 包含至少一个数字
pub fn validate_password_strength(password: &str) -> Result<()> {
    if password.len() < 8 {
        return Err(AppError::BadRequest(
            "密码长度不能少于 8 个字符".to_string(),
        ));
    }
    
    let has_letter = password.chars().any(|c| c.is_alphabetic());
    let has_digit = password.chars().any(|c| c.is_numeric());
    
    if !has_letter {
        return Err(AppError::BadRequest(
            "密码必须包含至少一个字母".to_string(),
        ));
    }
    
    if !has_digit {
        return Err(AppError::BadRequest(
            "密码必须包含至少一个数字".to_string(),
        ));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_string_length() {
        assert!(validate_string_length("hello", 3, 10, "测试字段").is_ok());
        assert!(validate_string_length("hi", 3, 10, "测试字段").is_err());
        assert!(validate_string_length("this is too long", 3, 10, "测试字段").is_err());
    }

    #[test]
    fn test_validate_not_empty() {
        assert!(validate_not_empty("hello", "测试字段").is_ok());
        assert!(validate_not_empty("", "测试字段").is_err());
        assert!(validate_not_empty("   ", "测试字段").is_err());
    }

    #[test]
    fn test_validate_task_title() {
        assert!(validate_task_title("有效的任务标题").is_ok());
        assert!(validate_task_title("").is_err());
        assert!(validate_task_title("   ").is_err());
    }

    #[test]
    fn test_validate_username() {
        assert!(validate_username("valid_user123").is_ok());
        assert!(validate_username("user-name").is_ok());
        assert!(validate_username("ab").is_err()); // 太短
        assert!(validate_username("invalid user").is_err()); // 包含空格
        assert!(validate_username("invalid@user").is_err()); // 包含特殊字符
    }

    #[test]
    fn test_validate_password_strength() {
        assert!(validate_password_strength("password123").is_ok());
        assert!(validate_password_strength("short").is_err()); // 太短
        assert!(validate_password_strength("12345678").is_err()); // 没有字母
        assert!(validate_password_strength("password").is_err()); // 没有数字
    }
}
