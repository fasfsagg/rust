// /-----------------------------------------------------------------------------\
// |                              【模块功能图示】                             |
// |-----------------------------------------------------------------------------|
// | 内部逻辑 (例如 服务层 Service, 控制器层 Controller)                         |
// |      |                                                                      |
// |      V                                                                      |
// | +---------------------------+      +-------------------------------------+ |
// | | 业务逻辑/解析失败           | ---> | 创建/返回 `Err(AppError::Variant)`  | |
// | +---------------------------+      +-------------------------------------+ |
// |                                      | 通过 `?` 或 `return Err(...)` 传播错误 |
// |                                      V                                       |
// | +-------------------------------------------------------------------------+ |
// | | Axum 处理函数返回 `Result<T, AppError>`                                  | |
// | +-------------------------------------------------------------------------+ |
// |      | Axum 检测到 `Err(AppError)`                                          |
// |      V                                                                      |
// | +-------------------------------------------------------------------------+ |
// | | 调用 `AppError::into_response(self)` (本模块 `impl IntoResponse`)       | |
// | +-------------------------------------------------------------------------+ |
// |      |                                                                      |
// |      V                                                                      |
// | +------------------------------------------------+                          |
// | | `impl IntoResponse for AppError` 内部逻辑:      |                          |
// | | 1. `match self` 确定状态码 (Status Code) 和消息 (Message) |                |
// | | 2. 构建 `json!({"error": ...})` 响应体 (Body)   |                          |
// | | 3. 返回 `(StatusCode, Json<Value>)`            |                          |
// | +------------------------------------------------+                          |
// |      |                                                                      |
// |      V                                                                      |
// | HTTP 响应 (例如 404 Not Found + JSON 响应体)                              |
// \-----------------------------------------------------------------------------/
//
// 文件路径: src/error.rs
//
// 【模块核心职责】
// 这个模块是应用程序的【错误处理中心】。
// 它定义了一个统一的错误枚举 `AppError`，封装了应用中可能出现的各种预期错误。
// 最关键的是，它为 `AppError` 实现了 Axum 的 `IntoResponse` trait，
// 这使得任何返回 `Result<_, AppError>` 的 Axum Handler 都能在出错时，
// 【自动地】将 `AppError` 转换为格式良好、带有合适状态码的 HTTP 错误响应。
// 同时，它还提供了一个方便的 `Result<T>` 类型别名和一些辅助函数来简化错误处理代码。
//
// 【关键概念】: `enum`, `impl IntoResponse`, `Result` 类型别名, 错误传播 (`?`), HTTP 状态码。
//
// 【面向初学者提示】: 把这里想象成一个"错误翻译器"，将内部的技术错误（如"找不到 ID"）
//                  翻译成外部用户和客户端能理解的 HTTP 错误（如"404 Not Found"和一个 JSON 说明）。

// --- 导入依赖 ---
use axum::{ // 导入 Axum 框架相关的类型
    http::StatusCode, // 用于表示 HTTP 状态码 (e.g., 404, 500)
    response::{ IntoResponse, Response }, // `IntoResponse` 是将类型转换为 HTTP 响应的核心 trait；`Response` 是 HTTP 响应类型
    Json, // 用于将数据序列化为 JSON 响应体
};
use serde_json::{ json, Value }; // 导入 `serde_json` 用于创建 JSON 值 (`Value`)
use uuid::Uuid; // 导入 UUID 类型，用于错误消息
use sea_orm::DbErr; // 导入 SeaORM 的数据库错误类型
use tracing_error::{ SpanTrace, ExtractSpanTrace }; // 导入 tracing-error 用于错误上下文跟踪

// --- 自定义错误枚举 ---

/// 应用程序统一错误枚举 (`AppError`)
///
/// 【用途】: 定义了应用程序中所有【可预期的】业务逻辑错误或请求处理错误。
///          每种错误变体 (variant) 代表一种特定的失败场景。
/// 【设计】: 每个变体通常携带一个 `String` 类型的消息，提供错误的具体上下文。
/// 【目标】: 提供比标准库 `Error` trait 更具体的错误类型，并方便地映射到 HTTP 状态码。
///
/// # 【`#[derive(Debug)]`】 [[关键语法要素: derive 宏]]
///   - 自动为 `AppError` 实现 `std::fmt::Debug` trait。
///   - 这允许我们使用 `{:?}` 或 `{:#?}` 格式化符号来打印 `AppError` 的实例，
///     对于调试和日志记录非常有用。
///
/// # 示例
/// ```
/// // 在 Service 层或 Controller 层创建错误实例:
/// // let not_found_error = AppError::NotFound("任务 ID 不存在".to_string());
/// // let bad_request_error = AppError::BadRequest("请求参数格式错误".to_string());
/// ```
#[derive(Debug)]
pub enum AppError {
    /// 404 Not Found - 表示请求的任务资源未能找到。
    TaskNotFound(Uuid),

    /// 400 Bad Request - 表示客户端发送的请求无效。
    BadRequest(String),

    /// 500 Internal Server Error - 包装了来自数据库的错误。
    DbErr(DbErr),

    // --- 认证相关错误 ---
    /// 409 Conflict - 用户名已存在（注册时）
    UserAlreadyExists(String),

    /// 401 Unauthorized - 无效的登录凭据（用户名或密码错误）
    InvalidCredentials,

    /// 500 Internal Server Error - 密码哈希处理错误
    PasswordHashError(String),

    /// 500 Internal Server Error - JWT 令牌生成错误
    TokenGenerationError(String),

    /// 401 Unauthorized - JWT 令牌无效或过期
    InvalidToken(String),

    /// 400 Bad Request - 输入验证错误
    ValidationError(String),

    /// 带有 SpanTrace 上下文的错误包装器
    /// 用于捕获错误发生时的 tracing span 上下文信息
    TracedError {
        /// 原始错误信息
        message: String,
        /// 错误发生时的 span 跟踪信息
        span_trace: SpanTrace,
        /// HTTP 状态码
        status_code: StatusCode,
    },
}

// --- 实现 Display trait ---

/// 为 `AppError` 实现 `Display` trait
///
/// 【目的】: 允许将 `AppError` 转换为字符串，用于日志记录和错误消息显示
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::TaskNotFound(id) => write!(f, "未找到ID为 {} 的任务", id),
            AppError::BadRequest(msg) => write!(f, "请求错误: {}", msg),
            AppError::DbErr(db_err) => write!(f, "数据库错误: {}", db_err),
            AppError::UserAlreadyExists(username) => write!(f, "用户名 '{}' 已存在", username),
            AppError::InvalidCredentials => write!(f, "用户名或密码错误"),
            AppError::PasswordHashError(msg) => write!(f, "密码哈希错误: {}", msg),
            AppError::TokenGenerationError(msg) => write!(f, "令牌生成错误: {}", msg),
            AppError::InvalidToken(msg) => write!(f, "无效的令牌: {}", msg),
            AppError::ValidationError(msg) => write!(f, "输入验证错误: {}", msg),
            AppError::TracedError { message, .. } => write!(f, "{}", message),
        }
    }
}

// --- 实现 IntoResponse ---

/// 为 `AppError` 实现 `IntoResponse` trait [[Axum 核心特性: IntoResponse]]
///
/// 【目的】: 这是 Axum 错误处理的核心机制。
///          实现了 `IntoResponse` 的类型可以被 Axum Handler 直接返回（通常在 `Result::Err` 中）。
///          Axum 会自动调用此 `into_response` 方法将错误转换为标准的 `axum::response::Response`。
/// 【流程】: 当 Handler 返回 `Err(app_error)` 时:
///          1. Axum 捕获到 `Err`。
///          2. Axum 调用 `app_error.into_response()`。
///          3. 此方法内部逻辑执行，生成 `Response` 对象。
///          4. Axum 将生成的 `Response` 发送给客户端。
impl IntoResponse for AppError {
    /// 将 `AppError` 实例转换为 HTTP 响应 (`Response`)。
    fn into_response(self) -> Response {
        let (status, message) = match self {
            // 如果是任务未找到错误，返回 404 和标准化的消息。
            AppError::TaskNotFound(id) =>
                (StatusCode::NOT_FOUND, format!("未找到ID为 {} 的任务", id)),
            // 如果是数据库错误，记录到日志（重要！），并返回通用的 500 错误。
            // 注意：为了安全，不应将原始的 `db_err` 细节暴露给客户端。
            AppError::DbErr(db_err) => {
                // 在服务器端打印详细的错误日志以供调试。
                eprintln!("[DB_ERROR] 数据库操作失败: {:?}", db_err);
                (StatusCode::INTERNAL_SERVER_ERROR, "服务器内部错误".to_string())
            }
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),

            // --- 认证相关错误处理 ---
            AppError::UserAlreadyExists(username) =>
                (StatusCode::CONFLICT, format!("用户名 '{}' 已存在", username)),
            AppError::InvalidCredentials =>
                (StatusCode::UNAUTHORIZED, "用户名或密码错误".to_string()),
            AppError::PasswordHashError(msg) => {
                eprintln!("[PASSWORD_HASH_ERROR] 密码哈希处理失败: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "服务器内部错误".to_string())
            }
            AppError::TokenGenerationError(msg) => {
                eprintln!("[TOKEN_ERROR] JWT令牌生成失败: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "服务器内部错误".to_string())
            }
            AppError::InvalidToken(msg) =>
                (StatusCode::UNAUTHORIZED, format!("无效的令牌: {}", msg)),
            AppError::ValidationError(msg) =>
                (StatusCode::BAD_REQUEST, format!("输入验证错误: {}", msg)),

            // 处理带有 SpanTrace 的错误
            AppError::TracedError { message, span_trace, status_code } => {
                // 记录详细的错误信息，包括 span 跟踪
                tracing::error!(
                    error_message = %message,
                    span_trace = %span_trace,
                    status_code = %status_code,
                    "TracedError occurred with span context"
                );
                (status_code, message)
            }
        };

        let body: Value =
            json!({
            "error": {
                "message": message,
                "code": status.as_u16()
            }
        });

        (status, Json(body)).into_response()
    }
}

// --- 实现 From Trait for DbErr ---

/// 实现 `From<DbErr>` for `AppError`
///
/// 【目的】: 这是实现 `?` 错误传播的关键。
///          它告诉编译器如何将一个 `sea_orm::DbErr` 自动转换为一个 `AppError`。
/// 【流程】: 当你在一个返回 `Result<_, AppError>` 的函数中使用 `?` 操作符处理一个
///          返回 `Result<_, DbErr>` 的表达式时，如果结果是 `Err(db_err)`，
///          编译器会自动调用 `AppError::from(db_err)` 来转换错误类型。
impl From<DbErr> for AppError {
    fn from(err: DbErr) -> Self {
        // 直接将传入的 `DbErr` 包装到 `AppError::DbErr` 变体中。
        AppError::DbErr(err)
    }
}

// --- Result 类型别名 ---

/// 定义应用程序范围的 `Result` 类型别名。
///
/// 【类型】: `Result<T, AppError>` 的别名。
/// 【目的】: 简化代码书写。
///          在函数签名中，可以用 `Result<T>` 代替冗长的 `std::result::Result<T, crate::error::AppError>`。
/// 【用法】: 在整个应用程序中，凡是可能返回 `AppError` 的函数，都可以使用这个别名。
///
/// # 示例
/// ```
/// // 不使用别名:
/// // fn might_fail() -> std::result::Result<(), crate::error::AppError> { ... }
///
/// // 使用别名:
/// // use crate::error::Result; // 导入别名
/// // fn might_fail() -> Result<()> { ... }
/// ```
pub type Result<T> = std::result::Result<T, AppError>;

// --- 辅助函数 (可选，但推荐) ---

/// 辅助函数：创建"无效 UUID"错误 (`AppError::BadRequest`)
///
/// 【目的】: 同上，用于标准化创建 UUID 解析失败时的错误。
///
/// # 参数
/// * `id: &str` - 尝试解析但失败的字符串。
/// # 返回值
/// * `AppError` - 一个配置好的 `AppError::BadRequest` 实例。
pub fn invalid_uuid(id: &str) -> AppError {
    AppError::BadRequest(format!("无效的UUID格式: {}", id))
}

// --- SpanTrace 错误处理扩展 ---

impl AppError {
    /// 创建一个带有当前 span 跟踪信息的错误
    ///
    /// 【功能】：捕获当前 tracing span 的上下文信息，并创建一个 TracedError
    ///
    /// # 参数
    /// * `message` - 错误消息
    /// * `status_code` - HTTP 状态码
    ///
    /// # 返回值
    /// * `AppError::TracedError` - 包含 span 跟踪信息的错误
    ///
    /// # 示例
    /// ```rust,no_run
    /// use axum_tutorial::error::AppError;
    /// use axum::http::StatusCode;
    ///
    /// let error = AppError::with_span_trace(
    ///     "数据库连接失败".to_string(),
    ///     StatusCode::INTERNAL_SERVER_ERROR
    /// );
    /// ```
    pub fn with_span_trace(message: String, status_code: StatusCode) -> Self {
        Self::TracedError {
            message,
            span_trace: SpanTrace::capture(),
            status_code,
        }
    }

    /// 将现有错误包装为带有 span 跟踪信息的错误
    ///
    /// 【功能】：将任何实现了 std::error::Error 的错误类型包装为 TracedError
    ///
    /// # 参数
    /// * `error` - 原始错误
    /// * `status_code` - HTTP 状态码
    ///
    /// # 返回值
    /// * `AppError::TracedError` - 包含 span 跟踪信息的错误
    pub fn wrap_with_span_trace<E: std::error::Error>(error: E, status_code: StatusCode) -> Self {
        Self::TracedError {
            message: error.to_string(),
            span_trace: SpanTrace::capture(),
            status_code,
        }
    }
}

/// 为 AppError 实现 ExtractSpanTrace trait
///
/// 【功能】：允许从 AppError 中提取 SpanTrace 信息
impl ExtractSpanTrace for AppError {
    fn span_trace(&self) -> Option<&SpanTrace> {
        match self {
            AppError::TracedError { span_trace, .. } => Some(span_trace),
            _ => None,
        }
    }
}

// --- Result 扩展 trait ---

/// Result 扩展 trait，提供便捷的错误跟踪方法
///
/// 【功能】：为 Result 类型添加便捷方法，自动捕获当前 span 的跟踪信息
pub trait InstrumentResult<T, E> {
    /// 在当前 span 中包装错误
    ///
    /// 【功能】：如果 Result 是 Err，则自动捕获当前 span 的跟踪信息并包装错误
    ///
    /// # 参数
    /// * `status_code` - 要使用的 HTTP 状态码
    ///
    /// # 返回值
    /// * `Result<T, AppError>` - 包装后的结果
    ///
    /// # 示例
    /// ```rust,no_run
    /// use axum_tutorial::error::{InstrumentResult, Result};
    /// use axum::http::StatusCode;
    ///
    /// fn example_function() -> Result<String> {
    ///     std::fs::read_to_string("file.txt")
    ///         .in_current_span(StatusCode::INTERNAL_SERVER_ERROR)
    /// }
    /// ```
    fn in_current_span(self, status_code: StatusCode) -> Result<T>;
}

impl<T, E: std::error::Error> InstrumentResult<T, E> for std::result::Result<T, E> {
    fn in_current_span(self, status_code: StatusCode) -> Result<T> {
        self.map_err(|e| AppError::wrap_with_span_trace(e, status_code))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use std::io;

    #[test]
    fn test_app_error_with_span_trace() {
        let error = AppError::with_span_trace(
            "测试错误".to_string(),
            StatusCode::INTERNAL_SERVER_ERROR
        );

        match error {
            AppError::TracedError { message, status_code, .. } => {
                assert_eq!(message, "测试错误");
                assert_eq!(status_code, StatusCode::INTERNAL_SERVER_ERROR);
            }
            _ => panic!("Expected TracedError variant"),
        }
    }

    #[test]
    fn test_app_error_wrap_with_span_trace() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "文件未找到");
        let app_error = AppError::wrap_with_span_trace(io_error, StatusCode::NOT_FOUND);

        match app_error {
            AppError::TracedError { message, status_code, .. } => {
                assert!(message.contains("文件未找到"));
                assert_eq!(status_code, StatusCode::NOT_FOUND);
            }
            _ => panic!("Expected TracedError variant"),
        }
    }

    #[test]
    fn test_extract_span_trace() {
        let traced_error = AppError::with_span_trace(
            "测试错误".to_string(),
            StatusCode::INTERNAL_SERVER_ERROR
        );

        // 测试 ExtractSpanTrace trait
        assert!(traced_error.span_trace().is_some());

        // 测试其他错误类型不包含 span trace
        let regular_error = AppError::BadRequest("普通错误".to_string());
        assert!(regular_error.span_trace().is_none());
    }

    #[test]
    fn test_instrument_result_success() {
        let success_result: std::result::Result<String, io::Error> = Ok("成功".to_string());
        let instrumented = success_result.in_current_span(StatusCode::INTERNAL_SERVER_ERROR);

        assert!(instrumented.is_ok());
        assert_eq!(instrumented.unwrap(), "成功");
    }

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
