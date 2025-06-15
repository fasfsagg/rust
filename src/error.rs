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
