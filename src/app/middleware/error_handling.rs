// src/app/middleware/error_handling.rs
//
// /-----------------------------------------------------------------------------\
// |                        【增强错误处理中间件模块】                           |
// |-----------------------------------------------------------------------------|
// | 基于 Axum 0.8.4 HandleErrorLayer 实现企业级错误处理中间件                  |
// | 功能包括：                                                                  |
// | 1. 全局错误捕获和处理                                                      |
// | 2. 超时错误处理                                                            |
// | 3. 请求验证错误映射                                                        |
// | 4. 自定义错误类型转换                                                      |
// | 5. 结构化错误响应                                                          |
// | 6. 错误上下文跟踪                                                          |
// \-----------------------------------------------------------------------------/

use axum::{
    BoxError,
    extract::Request,
    http::{Method, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::time::Duration;
use tower::timeout::TimeoutLayer;
use tracing::{error, info, instrument, warn};
use tracing_error::SpanTrace;

// 定义类型别名以简化复杂类型
type ErrorHandlerFuture = std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<Response, std::convert::Infallible>> + Send>,
>;
type ErrorHandlerFn = fn(BoxError) -> ErrorHandlerFuture;
type EnhancedErrorLayer = axum::error_handling::HandleErrorLayer<ErrorHandlerFn, Request>;

/// 增强的错误处理中间件构建器
///
/// 【功能】：提供一个统一的错误处理中间件栈，包含：
/// - 超时处理
/// - 全局错误捕获
/// - 结构化错误响应
/// - 错误上下文跟踪
pub fn enhanced_error_handling_layer() -> EnhancedErrorLayer {
    axum::error_handling::HandleErrorLayer::new(handle_error)
}

/// 创建超时中间件层
pub fn timeout_layer() -> TimeoutLayer {
    TimeoutLayer::new(Duration::from_secs(30))
}

/// 全局错误处理函数
///
/// 【功能】：处理所有中间件和服务产生的错误，将其转换为适当的HTTP响应
///
/// # 参数
/// * `error` - 捕获到的错误
///
/// # 返回值
/// * `Response` - 格式化的错误响应
#[instrument(skip(error), fields(error_type = %error))]
fn handle_error(
    error: BoxError,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<Response, std::convert::Infallible>> + Send>,
> {
    Box::pin(async move {
        // 捕获当前span的跟踪信息
        let span_trace = SpanTrace::capture();

        let (status, error_message, error_code) = if error.is::<tower::timeout::error::Elapsed>() {
            // 处理超时错误
            warn!("Request timeout occurred");
            (
                StatusCode::REQUEST_TIMEOUT,
                "请求超时，请稍后重试".to_string(),
                "REQUEST_TIMEOUT",
            )
        } else if let Some(source) = error.source() {
            // 处理有源错误的情况
            if source.to_string().contains("connection") {
                error!(error = %error, span_trace = %span_trace, "Connection error occurred");
                (
                    StatusCode::BAD_GATEWAY,
                    "服务暂时不可用，请稍后重试".to_string(),
                    "SERVICE_UNAVAILABLE",
                )
            } else if source.to_string().contains("timeout") {
                warn!(error = %error, "Service timeout occurred");
                (
                    StatusCode::GATEWAY_TIMEOUT,
                    "服务响应超时".to_string(),
                    "GATEWAY_TIMEOUT",
                )
            } else {
                error!(error = %error, span_trace = %span_trace, "Unhandled error with source");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "服务器内部错误".to_string(),
                    "INTERNAL_SERVER_ERROR",
                )
            }
        } else {
            // 处理其他未知错误
            error!(error = %error, span_trace = %span_trace, "Unhandled error occurred");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "服务器内部错误".to_string(),
                "INTERNAL_SERVER_ERROR",
            )
        };

        // 构建结构化错误响应
        let error_response = json!({
            "error": {
                "code": error_code,
                "message": error_message,
                "status": status.as_u16(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "trace_id": generate_trace_id()
            }
        });

        // 记录错误处理完成
        info!(
            status = status.as_u16(),
            error_code = error_code,
            "Error handled and response generated"
        );

        Ok((status, axum::Json(error_response)).into_response())
    })
}

/// 生成跟踪ID用于错误关联
///
/// 【功能】：生成唯一的跟踪ID，用于关联错误日志和响应
fn generate_trace_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// 请求验证错误处理器
///
/// 【功能】：专门处理请求验证相关的错误
///
/// # 参数
/// * `method` - HTTP方法
/// * `uri` - 请求URI
/// * `error` - 验证错误
///
/// # 返回值
/// * `Response` - 格式化的验证错误响应
#[instrument(skip(error), fields(method = %method, uri = %uri))]
pub async fn handle_validation_error(
    method: Method,
    uri: Uri,
    error: BoxError,
) -> Result<Response, std::convert::Infallible> {
    warn!(
        method = %method,
        uri = %uri,
        error = %error,
        "Validation error occurred"
    );

    let error_response = json!({
        "error": {
            "code": "VALIDATION_ERROR",
            "message": "请求验证失败",
            "details": error.to_string(),
            "status": 400,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "trace_id": generate_trace_id(),
            "request": {
                "method": method.to_string(),
                "uri": uri.to_string()
            }
        }
    });

    Ok((StatusCode::BAD_REQUEST, axum::Json(error_response)).into_response())
}

/// 认证错误处理器
///
/// 【功能】：专门处理认证相关的错误
///
/// # 参数
/// * `error` - 认证错误
///
/// # 返回值
/// * `Response` - 格式化的认证错误响应
#[instrument(skip(error))]
pub async fn handle_auth_error(error: BoxError) -> Result<Response, std::convert::Infallible> {
    warn!(error = %error, "Authentication error occurred");

    let error_response = json!({
        "error": {
            "code": "AUTHENTICATION_ERROR",
            "message": "认证失败",
            "details": "请检查您的认证凭据",
            "status": 401,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "trace_id": generate_trace_id()
        }
    });

    Ok((StatusCode::UNAUTHORIZED, axum::Json(error_response)).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::fmt;

    #[derive(Debug)]
    struct TestError {
        message: String,
    }

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.message)
        }
    }

    impl Error for TestError {}

    #[tokio::test]
    async fn test_handle_timeout_error() {
        let timeout_error = tower::timeout::error::Elapsed::new();
        let boxed_error: BoxError = Box::new(timeout_error);

        let result = handle_error(boxed_error).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);
    }

    #[tokio::test]
    async fn test_handle_generic_error() {
        let test_error = TestError {
            message: "Test error".to_string(),
        };
        let boxed_error: BoxError = Box::new(test_error);

        let result = handle_error(boxed_error).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_generate_trace_id() {
        let trace_id1 = generate_trace_id();
        let trace_id2 = generate_trace_id();

        assert_ne!(trace_id1, trace_id2);
        assert!(uuid::Uuid::parse_str(&trace_id1).is_ok());
        assert!(uuid::Uuid::parse_str(&trace_id2).is_ok());
    }
}
