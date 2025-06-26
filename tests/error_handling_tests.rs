// tests/error_handling_tests.rs
//
// /-----------------------------------------------------------------------------\
// |                        【错误处理功能测试模块】                             |
// |-----------------------------------------------------------------------------|
// | 基于 Axum 0.8.4 和 tracing-test 实现的错误处理功能全面测试                 |
// | 测试覆盖：                                                                  |
// | 1. AppError 错误类型转换和响应生成                                          |
// | 2. 错误处理中间件功能验证                                                   |
// | 3. SpanTrace 错误上下文跟踪                                                 |
// | 4. 错误恢复机制测试                                                         |
// | 5. 结构化错误响应格式验证                                                   |
// \-----------------------------------------------------------------------------/

use axum::{ http::{ Method, StatusCode, Uri }, response::IntoResponse, routing::get, Router };
use axum_test::TestServer;
use axum_tutorial::{
    error::{ AppError, InstrumentResult, Result },
    app::middleware::error_handling::{ handle_auth_error, handle_validation_error },
};
use serde_json::Value;
use std::{ io, time::Duration };
use tokio::time::sleep;
use tracing_test::traced_test;
use uuid::Uuid;

/// 测试 AppError 的基本功能
#[cfg(test)]
mod app_error_tests {
    use super::*;

    #[test]
    fn test_task_not_found_error() {
        let task_id = Uuid::new_v4();
        let error = AppError::TaskNotFound(task_id);

        // 测试 Display trait
        let display_msg = format!("{}", error);
        assert!(display_msg.contains(&task_id.to_string()));
        assert!(display_msg.contains("未找到ID为"));

        // 测试 Debug trait
        let debug_msg = format!("{:?}", error);
        assert!(debug_msg.contains("TaskNotFound"));
    }

    #[test]
    fn test_bad_request_error() {
        let message = "无效的请求参数";
        let error = AppError::BadRequest(message.to_string());

        let display_msg = format!("{}", error);
        assert!(display_msg.contains(message));
        assert!(display_msg.contains("请求错误"));
    }

    #[test]
    fn test_user_already_exists_error() {
        let username = "test_user";
        let error = AppError::UserAlreadyExists(username.to_string());

        let display_msg = format!("{}", error);
        assert!(display_msg.contains(username));
        assert!(display_msg.contains("已存在"));
    }

    #[test]
    fn test_invalid_credentials_error() {
        let error = AppError::InvalidCredentials;

        let display_msg = format!("{}", error);
        assert!(display_msg.contains("用户名或密码错误"));
    }

    #[test]
    fn test_invalid_token_error() {
        let token_msg = "令牌已过期";
        let error = AppError::InvalidToken(token_msg.to_string());

        let display_msg = format!("{}", error);
        assert!(display_msg.contains(token_msg));
        assert!(display_msg.contains("无效的令牌"));
    }
}

/// 测试 AppError 的 IntoResponse 实现
#[cfg(test)]
mod into_response_tests {
    use super::*;
    use axum::response::IntoResponse;

    #[tokio::test]
    async fn test_task_not_found_response() {
        let task_id = Uuid::new_v4();
        let error = AppError::TaskNotFound(task_id);

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        // 验证响应体格式
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["error"]["code"], 404);
        assert!(json["error"]["message"].as_str().unwrap().contains(&task_id.to_string()));
    }

    #[tokio::test]
    async fn test_bad_request_response() {
        let message = "无效的UUID格式";
        let error = AppError::BadRequest(message.to_string());

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["error"]["code"], 400);
        assert!(json["error"]["message"].as_str().unwrap().contains(message));
    }

    #[tokio::test]
    async fn test_user_already_exists_response() {
        let username = "duplicate_user";
        let error = AppError::UserAlreadyExists(username.to_string());

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["error"]["code"], 409);
        assert!(json["error"]["message"].as_str().unwrap().contains(username));
    }

    #[tokio::test]
    async fn test_invalid_credentials_response() {
        let error = AppError::InvalidCredentials;

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["error"]["code"], 401);
        assert!(json["error"]["message"].as_str().unwrap().contains("用户名或密码错误"));
    }

    #[tokio::test]
    async fn test_invalid_token_response() {
        let token_msg = "令牌格式错误";
        let error = AppError::InvalidToken(token_msg.to_string());

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["error"]["code"], 401);
        assert!(json["error"]["message"].as_str().unwrap().contains(token_msg));
    }
}

/// 测试 SpanTrace 错误跟踪功能
#[cfg(test)]
mod span_trace_tests {
    use super::*;
    use tracing_error::ExtractSpanTrace;

    #[traced_test]
    #[tokio::test]
    async fn test_with_span_trace() {
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
    #[tokio::test]
    async fn test_wrap_with_span_trace() {
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

    #[traced_test]
    #[tokio::test]
    async fn test_traced_error_response() {
        let error = AppError::with_span_trace("跟踪错误测试".to_string(), StatusCode::BAD_GATEWAY);

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["error"]["code"], 502);
        assert!(json["error"]["message"].as_str().unwrap().contains("跟踪错误测试"));
    }
}

/// 测试 InstrumentResult trait
#[cfg(test)]
mod instrument_result_tests {
    use super::*;

    #[traced_test]
    #[tokio::test]
    async fn test_instrument_result_success() {
        let success_result: std::result::Result<String, io::Error> = Ok("成功".to_string());
        let instrumented = success_result.in_current_span(StatusCode::INTERNAL_SERVER_ERROR);

        assert!(instrumented.is_ok());
        assert_eq!(instrumented.unwrap(), "成功");
    }

    #[traced_test]
    #[tokio::test]
    async fn test_instrument_result_error() {
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

/// 测试错误处理中间件
#[cfg(test)]
mod middleware_tests {
    use super::*;

    async fn timeout_handler() -> Result<String> {
        // 模拟超时场景
        sleep(Duration::from_secs(35)).await;
        Ok("不应该到达这里".to_string())
    }

    #[tokio::test]
    async fn test_timeout_middleware() {
        use tower::ServiceBuilder;
        use axum::{ BoxError, error_handling::HandleErrorLayer, http::StatusCode };
        use std::time::Duration;

        // 创建一个带有超时中间件的应用来测试超时处理
        let app = Router::new()
            .route("/timeout", get(timeout_handler))
            .layer(
                ServiceBuilder::new()
                    // 错误处理层必须在超时层之上
                    .layer(
                        HandleErrorLayer::new(|_: BoxError| async { StatusCode::REQUEST_TIMEOUT })
                    )
                    .timeout(Duration::from_secs(30)) // 30秒超时，但处理器需要35秒
            );

        let server = TestServer::new(app).unwrap();

        // 由于我们的超时处理器会睡眠35秒，而超时中间件设置为30秒，
        // 应该会触发超时错误并返回408 REQUEST_TIMEOUT
        let response = server.get("/timeout").await;

        // 验证超时中间件正确返回超时错误状态码
        assert_eq!(response.status_code(), 408); // REQUEST_TIMEOUT
    }

    #[tokio::test]
    async fn test_validation_error_handler() {
        let method = Method::POST;
        let uri = Uri::from_static("/test");
        let error: axum::BoxError = Box::new(
            io::Error::new(io::ErrorKind::InvalidInput, "验证失败")
        );

        let result = handle_validation_error(method, uri, error).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["error"]["code"], "VALIDATION_ERROR");
        assert!(json["error"]["message"].as_str().unwrap().contains("请求验证失败"));
        assert_eq!(json["error"]["request"]["method"], "POST");
        assert_eq!(json["error"]["request"]["uri"], "/test");
    }

    #[tokio::test]
    async fn test_auth_error_handler() {
        let error: axum::BoxError = Box::new(
            io::Error::new(io::ErrorKind::PermissionDenied, "认证失败")
        );

        let result = handle_auth_error(error).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["error"]["code"], "AUTHENTICATION_ERROR");
        assert!(json["error"]["message"].as_str().unwrap().contains("认证失败"));
    }
}
