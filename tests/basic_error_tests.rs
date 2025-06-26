// tests/basic_error_tests.rs
//
// /-----------------------------------------------------------------------------\
// |                        【基础错误处理测试模块】                             |
// |-----------------------------------------------------------------------------|
// | 简化的错误处理功能测试，专注于核心功能验证                                   |
// | 测试覆盖：                                                                  |
// | 1. AppError 基本功能                                                        |
// | 2. 错误响应转换                                                             |
// | 3. SpanTrace 功能                                                           |
// | 4. 错误分类                                                                 |
// \-----------------------------------------------------------------------------/

use axum_tutorial::error::{ AppError, InstrumentResult };
use axum::http::StatusCode;
use axum::response::IntoResponse;
use std::io;
use tracing_test::traced_test;
use uuid::Uuid;

/// 测试 AppError 的基本功能
#[cfg(test)]
mod app_error_basic_tests {
    use super::*;

    #[test]
    fn test_task_not_found_error_display() {
        let task_id = Uuid::new_v4();
        let error = AppError::TaskNotFound(task_id);

        let display_msg = format!("{}", error);
        assert!(display_msg.contains(&task_id.to_string()));
        assert!(display_msg.contains("未找到ID为"));
    }

    #[test]
    fn test_bad_request_error_display() {
        let message = "无效的请求参数";
        let error = AppError::BadRequest(message.to_string());

        let display_msg = format!("{}", error);
        assert!(display_msg.contains(message));
        assert!(display_msg.contains("请求错误"));
    }

    #[test]
    fn test_user_already_exists_error_display() {
        let username = "test_user";
        let error = AppError::UserAlreadyExists(username.to_string());

        let display_msg = format!("{}", error);
        assert!(display_msg.contains(username));
        assert!(display_msg.contains("已存在"));
    }

    #[test]
    fn test_invalid_credentials_error_display() {
        let error = AppError::InvalidCredentials;

        let display_msg = format!("{}", error);
        assert!(display_msg.contains("用户名或密码错误"));
    }

    #[test]
    fn test_invalid_token_error_display() {
        let token_msg = "令牌已过期";
        let error = AppError::InvalidToken(token_msg.to_string());

        let display_msg = format!("{}", error);
        assert!(display_msg.contains(token_msg));
        assert!(display_msg.contains("无效的令牌"));
    }
}

/// 测试 AppError 的 IntoResponse 实现
#[cfg(test)]
mod into_response_basic_tests {
    use super::*;

    #[tokio::test]
    async fn test_task_not_found_response_status() {
        let task_id = Uuid::new_v4();
        let error = AppError::TaskNotFound(task_id);

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_bad_request_response_status() {
        let message = "无效的UUID格式";
        let error = AppError::BadRequest(message.to_string());

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_user_already_exists_response_status() {
        let username = "duplicate_user";
        let error = AppError::UserAlreadyExists(username.to_string());

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_invalid_credentials_response_status() {
        let error = AppError::InvalidCredentials;

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_invalid_token_response_status() {
        let token_msg = "令牌格式错误";
        let error = AppError::InvalidToken(token_msg.to_string());

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}

/// 测试 SpanTrace 错误跟踪功能
#[cfg(test)]
mod span_trace_basic_tests {
    use super::*;
    use tracing_error::ExtractSpanTrace;

    #[traced_test]
    #[test]
    fn test_with_span_trace_creation() {
        let error = AppError::with_span_trace(
            "测试错误消息".to_string(),
            StatusCode::INTERNAL_SERVER_ERROR
        );

        // 验证错误类型
        match &error {
            AppError::TracedError { message, status_code, .. } => {
                assert_eq!(message, "测试错误消息");
                assert_eq!(*status_code, StatusCode::INTERNAL_SERVER_ERROR);
            }
            _ => panic!("Expected TracedError variant"),
        }

        // 验证 ExtractSpanTrace trait
        assert!(error.span_trace().is_some());
    }

    #[traced_test]
    #[test]
    fn test_wrap_with_span_trace_creation() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "文件未找到");
        let app_error = AppError::wrap_with_span_trace(io_error, StatusCode::NOT_FOUND);

        match &app_error {
            AppError::TracedError { message, status_code, .. } => {
                assert!(message.contains("文件未找到"));
                assert_eq!(*status_code, StatusCode::NOT_FOUND);
            }
            _ => panic!("Expected TracedError variant"),
        }

        assert!(app_error.span_trace().is_some());
    }

    #[test]
    fn test_extract_span_trace_from_regular_error() {
        let regular_error = AppError::BadRequest("普通错误".to_string());
        assert!(regular_error.span_trace().is_none());
    }
}

/// 测试 InstrumentResult trait
#[cfg(test)]
mod instrument_result_basic_tests {
    use super::*;

    #[traced_test]
    #[test]
    fn test_instrument_result_success() {
        let success_result: std::result::Result<String, io::Error> = Ok("成功".to_string());
        let instrumented = success_result.in_current_span(StatusCode::INTERNAL_SERVER_ERROR);

        assert!(instrumented.is_ok());
        assert_eq!(instrumented.unwrap(), "成功");
    }

    #[traced_test]
    #[test]
    fn test_instrument_result_error() {
        let error_result: std::result::Result<String, io::Error> = Err(
            io::Error::new(io::ErrorKind::PermissionDenied, "权限被拒绝")
        );
        let instrumented = error_result.in_current_span(StatusCode::FORBIDDEN);

        assert!(instrumented.is_err());
        match instrumented.unwrap_err() {
            AppError::TracedError { message, status_code, .. } => {
                assert!(message.contains("权限被拒绝"));
                assert_eq!(status_code, StatusCode::FORBIDDEN);
            }
            _ => panic!("Expected TracedError variant"),
        }
    }
}

/// 测试错误辅助函数
#[cfg(test)]
mod error_helpers_tests {
    use super::*;
    use axum_tutorial::error::invalid_uuid;

    #[test]
    fn test_invalid_uuid_helper() {
        let error = invalid_uuid("invalid-uuid-string");
        match error {
            AppError::BadRequest(msg) => {
                assert!(msg.contains("无效的UUID格式"));
                assert!(msg.contains("invalid-uuid-string"));
            }
            _ => panic!("Expected BadRequest variant"),
        }
    }
}

/// 测试错误恢复配置
#[cfg(test)]
mod error_recovery_config_tests {
    use axum_tutorial::app::utils::error_recovery::{
        ErrorRecoveryConfig,
        ErrorClassificationConfig,
        ErrorCategory,
    };

    #[test]
    fn test_error_classification_transient() {
        let config = ErrorClassificationConfig {
            transient_status_codes: vec![500, 502, 503, 504],
            permanent_status_codes: vec![400, 401, 403, 404, 422],
            rate_limit_status_codes: vec![429],
            timeout_status_codes: vec![408, 504],
        };

        assert_eq!(config.classify_error(500), ErrorCategory::Transient);
        assert_eq!(config.classify_error(502), ErrorCategory::Transient);
        assert_eq!(config.classify_error(503), ErrorCategory::Transient);
    }

    #[test]
    fn test_error_classification_permanent() {
        let config = ErrorClassificationConfig {
            transient_status_codes: vec![500, 502, 503, 504],
            permanent_status_codes: vec![400, 401, 403, 404, 422],
            rate_limit_status_codes: vec![429],
            timeout_status_codes: vec![408, 504],
        };

        assert_eq!(config.classify_error(400), ErrorCategory::Permanent);
        assert_eq!(config.classify_error(401), ErrorCategory::Permanent);
        assert_eq!(config.classify_error(404), ErrorCategory::Permanent);
    }

    #[test]
    fn test_error_classification_rate_limit() {
        let config = ErrorClassificationConfig {
            transient_status_codes: vec![500, 502, 503, 504],
            permanent_status_codes: vec![400, 401, 403, 404, 422],
            rate_limit_status_codes: vec![429],
            timeout_status_codes: vec![408, 504],
        };

        assert_eq!(config.classify_error(429), ErrorCategory::RateLimit);
    }

    #[test]
    fn test_error_classification_timeout() {
        let config = ErrorClassificationConfig {
            transient_status_codes: vec![500, 502, 503, 504],
            permanent_status_codes: vec![400, 401, 403, 404, 422],
            rate_limit_status_codes: vec![429],
            timeout_status_codes: vec![408, 504],
        };

        assert_eq!(config.classify_error(408), ErrorCategory::Timeout);
        // 注意：504 在这里会被归类为 Timeout，因为它在 timeout_status_codes 中
        assert_eq!(config.classify_error(504), ErrorCategory::Timeout);
    }

    #[test]
    fn test_should_retry_logic() {
        let config = ErrorClassificationConfig {
            transient_status_codes: vec![500, 502, 503, 504],
            permanent_status_codes: vec![400, 401, 403, 404, 422],
            rate_limit_status_codes: vec![429],
            timeout_status_codes: vec![408, 504],
        };

        // 应该重试的错误
        assert!(config.should_retry(500)); // Transient
        assert!(config.should_retry(429)); // RateLimit
        assert!(config.should_retry(408)); // Timeout

        // 不应该重试的错误
        assert!(!config.should_retry(400)); // Permanent
        assert!(!config.should_retry(401)); // Permanent
        assert!(!config.should_retry(404)); // Permanent
    }

    #[test]
    fn test_default_error_recovery_config() {
        let config = ErrorRecoveryConfig::default();

        assert_eq!(config.retry.max_retries, 3);
        assert_eq!(config.circuit_breaker.failure_threshold, 5);
        assert!(config.degradation.enabled);
        assert_eq!(config.degradation.default_response, "服务暂时不可用，请稍后重试");
    }
}
