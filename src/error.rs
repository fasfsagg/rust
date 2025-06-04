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
    /// 404 Not Found - 表示请求的资源（如特定任务）未能找到。
    /// 【关联状态码】: `StatusCode::NOT_FOUND` (404)
    NotFound(String),

    /// 400 Bad Request - 表示客户端发送的请求无效，例如格式错误、缺少必要参数、参数值不合法等。
    /// 【关联状态码】: `StatusCode::BAD_REQUEST` (400)
    BadRequest(String),

    /// 500 Internal Server Error - 表示服务器在处理请求时遇到了意外的内部问题。
    ///                      这通常是代码中的 bug 或外部服务故障导致的。
    /// 【关联状态码】: `StatusCode::INTERNAL_SERVER_ERROR` (500)
    /// 【注意】: 应尽量避免向客户端暴露过多内部细节，消息应通用化。
    InternalServerError(String),

    /// 409 Conflict - 表示请求与服务器当前状态冲突，通常用于创建已存在的资源或更新冲突。
    /// 【关联状态码】: `StatusCode::CONFLICT` (409)
    Conflict(String),
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
        // --- 步骤 1: 根据错误变体确定状态码和错误消息 ---
        // 使用 `match` 表达式解构 `self` (即 AppError 实例)。
        // 每个分支对应 `AppError` 的一个变体。
        // 返回一个元组 `(StatusCode, String)`。
        let (status, message) = match self {
            // 如果错误是 NotFound，状态码是 404，消息是内部携带的 String。
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            // 如果错误是 BadRequest，状态码是 400。
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            // 如果错误是 InternalServerError，状态码是 500。
            AppError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            // 如果错误是 Conflict，状态码是 409。
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg),
        };

        // --- 步骤 2: 构建 JSON 格式的响应体 ---
        // 使用 `serde_json::json!` 宏创建一个 JSON `Value`。
        // 这个宏提供了一种方便的方式来构造 JSON 对象。
        // 响应体包含一个 "error" 对象，其中有 "message" 和 "code" 字段。
        let body: Value =
            json!({
            "error": {
                "message": message, // 从 match 语句获取的错误消息
                "code": status.as_u16() // 将 StatusCode 转换为 u16 数字码
            }
        });

        // --- 步骤 3: 组合状态码和 JSON 响应体为最终响应 ---
        // `(status, Json(body))` 创建一个元组。
        // Axum 为 `(StatusCode, Json<T>)` 类型也实现了 `IntoResponse`。
        // 所以我们直接调用 `.into_response()` 将这个元组转换为最终的 `Response` 对象。
        // 这会自动设置正确的 HTTP 状态码和 `Content-Type: application/json` 响应头。
        (status, Json(body)).into_response()
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

/// 辅助函数：创建"任务未找到"错误 (`AppError::NotFound`)
///
/// 【目的】: 提供一个标准化的方式来创建特定类型的 `AppError`。
///          避免在代码中重复构造错误消息字符串。
///          提高代码可读性和一致性。
///
/// # 参数
/// * `id: Uuid` - 未找到的任务的 UUID。
/// # 返回值
/// * `AppError` - 一个配置好的 `AppError::NotFound` 实例。
pub fn task_not_found(id: Uuid) -> AppError {
    AppError::NotFound(format!("未找到ID为 {} 的任务", id))
}

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
