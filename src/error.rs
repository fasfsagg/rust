// 文件路径: src/error.rs

// /--------------------------------------------------------------------------------------------------\
// |                                      【模块功能图示】 (error.rs)                                   |
// |--------------------------------------------------------------------------------------------------|
// |                                                                                                  |
// | [应用内部逻辑] (例如: 服务层 Service, 控制器层 Controller)                                          |
// |      |                                                                                           |
// |      V  (发生错误或特定业务条件)                                                                      |
// | [创建 AppError 实例] (例如: AppError::UserAlreadyExists("用户名已存在".to_string()))              |
// |      |                                                                                           |
// |      V  (通过 Result<T, AppError> 返回, 或在 Axum Handler 中直接返回 AppError)                       |
// | [Axum 框架捕获到 AppError]                                                                         |
// |      |                                                                                           |
// |      V  (自动调用 AppError 的 into_response 方法)                                                    |
// | [impl IntoResponse for AppError]                                                                 |
// |   - match self { ... } (根据 AppError 的具体变体)                                                |
// |   - 确定 HTTP 状态码 (例如: StatusCode::CONFLICT)                                                |
// |   - 构建 JSON 响应体 (例如: {"error": {"message": "...", "code": 409}})                         |
// |      |                                                                                           |
// |      V                                                                                           |
// | [HTTP 错误响应] (发送给客户端, 例如: 409 Conflict + JSON Body)                                   |
// |                                                                                                  |
// \--------------------------------------------------------------------------------------------------/
//
// 【模块核心职责】 (Module Core Responsibilities)
// 1. **定义自定义错误类型 (`AppError`)**: `AppError` 枚举 (enum) 封装了应用程序中所有可预期的业务逻辑错误和请求处理错误。
//    每种错误变体代表一种特定的失败场景。
// 2. **标准化错误响应**: 通过为 `AppError` 实现 Axum 的 `IntoResponse` trait，确保所有自定义错误都能被自动、一致地转换为
//    对客户端友好的 HTTP 错误响应 (通常是 JSON 格式，包含错误信息和相应的 HTTP 状态码)。
// 3. **简化错误处理**: 提供 `Result<T>` 类型别名 (`core::result::Result<T, AppError>`)，简化函数签名。
//    同时，提供辅助函数 (helper functions) 来方便地创建特定类型的 `AppError` 实例。
// 4. **实现标准错误 Traits**: 实现 `std::fmt::Debug`, `std::fmt::Display`, 和 `std::error::Error` traits，
//    使得 `AppError` 能够与 Rust 的标准错误处理生态系统更好地集成 (例如，用于日志记录、错误链)。
//
// 【关键技术点】 (Key Technologies)
// - **枚举 (`enum AppError`)**: Rust 的枚举用于定义一个可以表示多种可能状态的类型。在这里，每种状态就是一个特定的应用错误。
// - **Trait 实现 (`impl Trait for Type`)**: Rust 通过 trait 实现来为类型添加行为。
//   - `Debug`: 用于调试输出。
//   - `Display`: 用于用户友好的字符串表示。
//   - `std::error::Error`: 标记类型为标准错误，并可提供错误来源 (source)。
//   - `axum::response::IntoResponse`: Axum 的核心 trait，用于将自定义类型转换为 HTTP 响应。
// - **`match` 语句**: Rust 强大的模式匹配工具，用于根据 `AppError` 的不同变体执行不同的逻辑 (例如，在 `fmt` 和 `into_response` 中)。
// - **HTTP 状态码 (`axum::http::StatusCode`)**: 用于表示 HTTP 响应的状态，例如 200 OK, 404 Not Found, 500 Internal Server Error。
// - **JSON 响应 (`axum::Json`, `serde_json::json!`)**: 使用 `axum::Json` 包装器和 `serde_json::json!` 宏来创建 JSON 格式的 HTTP 响应体。
// - **类型别名 (`pub type Result<T> = ...`)**: 为复杂的类型签名创建一个更简洁的名称。

// --- 导入依赖 ---
// `use` 关键字用于将其他模块或库中定义的项（如结构体、枚举、函数、trait等）引入到当前文件的作用域中，
// 这样就可以直接使用它们，而不需要写完整的路径。

// 从 `axum` crate 中导入:
use axum::{
    http::StatusCode, // `StatusCode` 枚举用于表示 HTTP 状态码，例如 404 (NOT_FOUND), 200 (OK)。
    response::{IntoResponse, Response}, // `IntoResponse` 是一个 trait，实现了它的类型可以被转换为 HTTP 响应。`Response` 是 HTTP 响应类型。
    Json, // `Json` 是一个 Axum 提取器和响应类型，用于处理 JSON 数据。当作为响应时，它会将数据序列化为 JSON 并设置正确的 `Content-Type` 头。
};
// 从 `serde_json` crate 中导入:
use serde_json::{json, Value}; // `json!` 是一个宏，用于方便地创建 `serde_json::Value` (动态 JSON 值) 的实例。`Value` 是 `serde_json` 中代表任意 JSON 数据的枚举。

// `use uuid::Uuid;` // 此行被注释掉，因为之前用于任务ID的 Uuid 相关错误辅助函数 (如 task_not_found, invalid_uuid) 已被移除或修改。
                     // 如果将来其他地方需要基于 Uuid 的错误，可以取消注释。

// --- Result 类型别名 ---

// `pub type Result<T> = core::result::Result<T, AppError>;`
// - `pub type`: `pub` 表示这个类型别名是公共的，可以在其他模块中使用。`type` 关键字用于创建类型别名。
// - `Result<T>`: 这是我们定义的新类型别名的名称。`<T>` 表示它是一个泛型类型别名，`T` 是一个类型参数，代表成功时 `Ok` 变体中包含的值的类型。
// - `core::result::Result<T, AppError>`: 这是 Rust 核心库中定义的标准 `Result` 枚举。
//   - `Result<T, E>` 是一个泛型枚举，有两个变体：
//     - `Ok(T)`: 表示操作成功，并包含一个类型为 `T` 的值。
//     - `Err(E)`: 表示操作失败，并包含一个类型为 `E` 的错误值。
//   - 在这里，我们将错误类型 `E` 固定为我们自定义的 `AppError`。
//
// 【目的与好处】
// 这个类型别名的主要目的是**简化函数签名**。在整个应用程序中，凡是可能返回 `AppError` 的函数，
// 其返回类型现在可以写成 `Result<T>`，而不是更冗长的 `core::result::Result<T, crate::error::AppError>`。
// 这使得代码更简洁，可读性更高。
// 例如，一个可能返回 `String` 或 `AppError` 的函数，可以这样声明：`fn my_function() -> Result<String> { ... }`
pub type Result<T> = core::result::Result<T, AppError>;

// --- 自定义错误枚举 (`AppError`) ---

// `#[derive(Debug)]` 是一个【派生宏 (derive macro)】。
// 它告诉 Rust 编译器自动为 `AppError` 枚举实现 `std::fmt::Debug` trait。
// - `std::fmt::Debug` trait 用于生成一个主要供开发者使用的、调试目的的字符串表示。
// - 实现此 trait 后，我们就可以使用 `{:?}` (普通调试输出) 或 `{:#?}` (美化调试输出) 格式化占位符来打印 `AppError` 的实例。
//   例如: `let err = AppError::InvalidToken; println!("{:?}", err);`
//   这对于调试错误流程、记录错误日志等非常有用。
//
// 把 `AppError`想象成一个容器，这个容器可以装不同种类的“错误标签”。每个标签代表一种特定的问题。
#[derive(Debug)]
// `pub enum AppError { ... }`: 定义一个公共的枚举 `AppError`。
// - `pub`: 表示这个枚举及其变体 (variants) 可以在其他模块中使用。
// - `enum`: 关键字，用于定义枚举类型。枚举允许我们定义一个可以拥有多种可能“变体”或“状态”的类型。
pub enum AppError {
    // `NotFound(String)`: 表示请求的资源未找到。
    // - `NotFound`: 是这个错误变体的名称。
    // - `(String)`: 表示这个变体携带一个 `String` 类型的值。这个 `String` 通常用于存储更具体的错误信息，
    //   例如 "用户 ID '123' 未找到" 或 "产品 'abc' 不存在"。
    // 【关联状态码】: 通常映射到 HTTP `404 Not Found`。
    // 【真实场景】: 比如用户请求一个不存在的商品页面。
    NotFound(String),

    // `BadRequest(String)`: 表示客户端发送的请求本身是无效的。
    // - 例如，请求的 JSON 体格式错误、缺少必要的参数、参数值不合法等。
    // - `(String)`: 同样携带一个 `String` 用于描述具体的请求问题。
    // 【关联状态码】: 通常映射到 HTTP `400 Bad Request`。
    // 【真实场景】: 用户注册时，密码字段为空，或者邮箱格式不正确。
    BadRequest(String),

    // `InternalServerError(String)`: 表示服务器在处理请求时遇到了未预料到的内部问题。
    // - 这通常指示了服务器端的代码缺陷 (bug)、外部依赖服务故障 (如数据库连接断开) 或其他意外情况。
    // - `(String)`: 消息应尽量通用，避免向客户端泄露过多敏感的内部实现细节。
    // 【关联状态码】: 通常映射到 HTTP `500 Internal Server Error`。
    // 【真实场景】: 服务器尝试写入数据库时，数据库突然崩溃。
    InternalServerError(String),

    // `Conflict(String)`: 表示请求与服务器当前状态冲突，无法完成。
    // - 通常用于尝试创建一个已存在的资源 (如用户名已被注册)，或进行一个会导致数据冲突的更新。
    // - `(String)`: 描述冲突的具体原因。
    // 【关联状态码】: 通常映射到 HTTP `409 Conflict`。
    // 【真实场景】: 用户尝试用一个已经被其他用户占用的邮箱地址注册新账户。
    Conflict(String),

    // --- 新增的认证和数据库相关错误变体 ---

    // `UserAlreadyExists(String)`: 特指用户注册时，用户名已存在导致的冲突。
    // - `(String)`: 存在冲突的用户名。
    // 【关联状态码】: HTTP `409 Conflict`。
    UserAlreadyExists(String),

    // `InvalidCredentials`: 表示用户登录时提供的凭证 (如用户名或密码) 无效或不匹配。
    // - 这个变体不携带额外数据，因为具体的错误信息（“无效凭证”）是固定的。
    // 【关联状态码】: HTTP `401 Unauthorized`。
    // 【真实场景】: 用户登录时输错了密码。
    InvalidCredentials,

    // `InvalidToken`: 表示客户端提供的认证令牌 (如 JWT) 无效。
    // - 原因可能包括：令牌格式错误、签名校验失败、令牌已过期、令牌被篡改等。
    // 【关联状态码】: 通常是 HTTP `401 Unauthorized`，有时也可能是 `403 Forbidden` (如果令牌有效但权限不足，不过我们有 `UnauthorizedAccess` 专门处理权限问题)。
    // 【真实场景】: 用户使用一个过期的 JWT 访问受保护资源。
    InvalidToken,

    // `MissingToken`: 表示客户端发起的请求中缺少必要的认证令牌。
    // - 通常是 HTTP 请求头中没有 `Authorization` 字段，或者该字段为空。
    // 【关联状态码】: HTTP `401 Unauthorized`。
    // 【真实场景】: 用户未登录就尝试访问需要登录才能查看的页面。
    MissingToken,

    // `UnauthorizedAccess`: 表示用户已通过认证 (令牌有效)，但无权访问所请求的特定资源或执行特定操作。
    // - 这是关于“授权”(Authorization) 而非“认证”(Authentication) 的错误。
    // 【关联状态码】: HTTP `403 Forbidden`。
    // 【真实场景】: 普通用户尝试访问管理员才能操作的后台管理功能。
    UnauthorizedAccess,

    // `DatabaseError(String)`: 表示在与数据库交互过程中发生了错误。
    // - `(String)`: 存储来自数据库驱动或 ORM 的原始错误信息，或一个更友好的概括性描述。
    //   同样，应注意不要将过于敏感的数据库错误细节直接暴露给客户端。
    // 【关联状态码】: HTTP `500 Internal Server Error`。
    // 【真实场景】: 执行 SQL 查询时发生语法错误，或数据库连接池耗尽。
    DatabaseError(String),
}

// --- 为 AppError 实现标准错误处理相关的 Trait ---

// `impl std::fmt::Display for AppError { ... }`
// - `impl ... for ...`: Rust 中为类型实现 trait (特性) 的语法。
// - `std::fmt::Display`: 标准库中的一个 trait，用于为类型提供一个“用户友好”的、人类可读的字符串表示。
//   这不同于 `Debug` trait (用于调试)，`Display` 更侧重于最终用户看到的信息。
//   例如，当错误需要打印给用户看，或者作为错误日志的一部分时，会使用 `Display` 的实现。
impl std::fmt::Display for AppError {
    // `fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { ... }`: `Display` trait 要求实现的方法。
    // - `&self`: 表示该方法借用 `AppError` 实例的不可变引用。它不会消耗或修改错误实例。
    // - `f: &mut std::fmt::Formatter<'_>`: 一个格式化器 (formatter) 的可变引用。
    //   我们需要使用这个格式化器 `f` 来写入我们想要展示的字符串。
    //   `'_` 是一个省略的生命周期参数，表示格式化器的生命周期与被格式化的数据 (`&self`) 相关。
    // - `-> std::fmt::Result`: 返回一个 `Result`，表示格式化操作是否成功。
    //   通常，如果写入格式化器 `f` 的操作都成功，则返回 `Ok(())`。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `match self { ... }`: 使用 `match` 表达式来根据 `AppError` 的不同变体提供不同的字符串表示。
        // `self` 指的是当前的 `AppError` 实例。
        match self {
            // `AppError::NotFound(msg)`: 如果 `self` 是 `NotFound` 变体，`msg` 会绑定到内部的 `String` 值。
            // `write!(f, "Resource not found: {}", msg)`: `write!` 是一个宏，用于将格式化的字符串写入到 `f` (格式化器)。
            //   - `"Resource not found: {}"`: 格式化字符串模板。`{}` 是一个占位符。
            //   - `msg`: 占位符 `{}` 的值，即 `NotFound` 变体中携带的具体错误信息。
            AppError::NotFound(msg) => write!(f, "请求的资源未找到: {}", msg),
            AppError::BadRequest(msg) => write!(f, "无效的请求: {}", msg),
            AppError::InternalServerError(msg) => write!(f, "服务器内部错误: {}", msg),
            AppError::Conflict(msg) => write!(f, "操作冲突: {}", msg),
            AppError::UserAlreadyExists(username) => write!(f, "用户 '{}' 已存在", username),
            AppError::InvalidCredentials => write!(f, "无效的凭证 (用户名或密码错误)"), // 固定消息
            AppError::InvalidToken => write!(f, "无效或过期的认证令牌"),
            AppError::MissingToken => write!(f, "请求中缺少认证令牌"),
            AppError::UnauthorizedAccess => write!(f, "无权访问此资源"),
            AppError::DatabaseError(err_msg) => write!(f, "数据库操作失败: {}", err_msg),
        }
        // `write!` 宏本身返回 `std::fmt::Result`，所以可以直接作为 `fmt` 函数的返回值。
    }
}

// `impl std::error::Error for AppError { ... }`
// - `std::error::Error`: 这是 Rust 标准库中所有错误类型都应该（或可以）实现的“标记” trait。
//   实现此 trait 表明 `AppError` 是一个标准的错误类型。
//   它有几个可选的方法，最常用的是 `source()`，用于错误链 (error chaining)，即一个错误是由另一个底层错误引起的。
impl std::error::Error for AppError {
    // `fn source(&self) -> Option<&(dyn std::error::Error + 'static)> { ... }`: `Error` trait 的一个方法。
    // - `&self`: 不可变借用当前错误实例。
    // - `-> Option<&(dyn std::error::Error + 'static)>`: 返回一个可选的、对底层错误的引用。
    //   - `Option`: 表示可能没有底层错误 (返回 `None`)，或者有 (返回 `Some(...)`)。
    //   - `&(dyn std::error::Error + 'static)`: 这是一个【trait 对象引用】。
    //     - `dyn std::error::Error`: 表示任何实现了 `Error` trait 的类型。`dyn` 关键字用于动态分发。
    //     - `+ 'static`: 表示这个 trait 对象引用的底层错误必须拥有 `'static` 生命周期 (即不包含任何非静态的借用)。
    //       这对于错误链很重要，确保底层错误在需要时仍然有效。
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // 在当前的 `AppError` 实现中，我们没有显式地包装另一个错误作为其直接来源。
        // 例如，`DatabaseError(String)` 只是存储了一个错误消息字符串，而不是原始的数据库错误对象 (如 `sea_orm::DbErr`)。
        // 如果我们想实现更详细的错误链，可以这样做：
        //
        // enum AppError {
        //   DatabaseError(sea_orm::DbErr), // 直接存储 DbErr
        //   // ... 或 ...
        //   DatabaseErrorGeneric(Box<dyn std::error::Error + Send + Sync>), // 存储任意盒子里的错误
        // }
        //
        // impl std::error::Error for AppError {
        //   fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        //     match self {
        //       AppError::DatabaseError(ref underlying_db_err) => Some(underlying_db_err), // DbErr 实现了 std::error::Error
        //       AppError::DatabaseErrorGeneric(ref boxed_err) => Some(boxed_err.as_ref()), // .as_ref() 从 Box<dyn Error> 获取 &dyn Error
        //       _ => None, // 其他错误类型没有底层错误源
        //     }
        //   }
        // }
        //
        // 对于当前的简单实现，我们总是返回 `None`，表示 `AppError` 是错误的根源 (或者我们选择不暴露其来源)。
        None
    }
}


// --- 为 AppError 实现 Axum 的 IntoResponse Trait ---

// `impl IntoResponse for AppError { ... }`
// - `axum::response::IntoResponse`: 这是 Axum 框架中的一个核心 trait。
//   如果一个类型 `T` 实现了 `IntoResponse`，那么 Axum 的路由处理函数 (handler) 就可以直接返回 `T` 或者 `Result<U, T>` (其中 `U` 也实现了 `IntoResponse`)。
//   Axum 会自动调用 `T` 的 `into_response()` 方法来将其转换为一个标准的 HTTP `Response` 对象。
//   这极大地简化了错误处理流程：我们只需要在业务逻辑中返回自定义的 `AppError`，Axum 会负责将其转换成合适的 HTTP 错误响应。
impl IntoResponse for AppError {
    // `fn into_response(self) -> Response { ... }`: `IntoResponse` trait 要求实现的方法。
    // - `self`: 注意这里是 `self` (而不是 `&self` 或 `&mut self`)，表示这个方法会获取 `AppError` 实例的【所有权】。
    //   这意味着在调用 `into_response` 之后，原来的 `AppError` 实例会被消耗掉。这对于错误处理通常是合适的。
    // - `-> Response`: 返回一个 `axum::response::Response` 类型的实例。
    fn into_response(self) -> Response {
        // --- 步骤 1: 根据 AppError 的具体变体确定 HTTP 状态码和要发送给客户端的错误消息 ---
        // 再次使用 `match self { ... }` 来处理不同的错误变体。
        // 这个 `match` 表达式的目的是为每种 `AppError` 变体决定两件事：
        //   1. `status`: 对应的 `axum::http::StatusCode` (例如，`NotFound` 对应 `StatusCode::NOT_FOUND`)。
        //   2. `message`: 要包含在响应体中的错误消息字符串。
        // `let (status, message) = ...`: 将 `match` 表达式的结果 (一个元组 `(StatusCode, String)`) 解构赋值给 `status` 和 `message` 变量。
        let (status, error_message_for_client) = match self {
            // `AppError::NotFound(msg)`: 如果是 `NotFound` 错误，状态码是 `StatusCode::NOT_FOUND` (404)。
            //   `msg` 是 `NotFound` 变体中携带的原始错误信息字符串。我们直接将其用作客户端消息。
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            // 对于 `InternalServerError`，我们通常不希望将内部的 `msg` (可能包含敏感调试信息) 直接暴露给客户端。
            // 更好的做法是提供一个通用的错误消息。
            // 但为了简单起见，当前代码直接使用了内部消息。在生产中应予以注意。
            // 例如: `AppError::InternalServerError(_original_msg) => (StatusCode::INTERNAL_SERVER_ERROR, "服务器发生了一个内部错误，请稍后再试。".to_string()),`
            AppError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg),

            // 处理新增的认证和数据库错误
            AppError::UserAlreadyExists(username) => (
                StatusCode::CONFLICT, // 409 Conflict
                // 使用 `format!` 宏来构造一个更具体的错误消息。
                format!("用户 '{}' 已存在 (User '{}' already exists)", username, username),
            ),
            AppError::InvalidCredentials => (
                StatusCode::UNAUTHORIZED, // 401 Unauthorized
                // 对于这类错误，通常返回一个固定的、不泄露过多信息的消息。
                "无效的凭证 (Invalid credentials)".to_string(),
            ),
            AppError::InvalidToken => (
                StatusCode::UNAUTHORIZED, // 401 Unauthorized (或者有时用 403 Forbidden，取决于具体策略)
                "无效或过期的认证令牌 (Invalid or expired token)".to_string(),
            ),
            AppError::MissingToken => (
                StatusCode::UNAUTHORIZED, // 401 Unauthorized
                "请求头中缺少认证令牌 (Missing authentication token in request headers)".to_string(),
            ),
            AppError::UnauthorizedAccess => (
                StatusCode::FORBIDDEN, // 403 Forbidden
                "无权访问此资源 (Unauthorized to access this resource)".to_string(),
            ),
            // 同样，对于 `DatabaseError`，应谨慎考虑是否将原始 `err_msg` 直接暴露给客户端。
            // 例如: `AppError::DatabaseError(_original_db_err_msg) => (StatusCode::INTERNAL_SERVER_ERROR, "数据库操作时发生错误。".to_string()),`
            AppError::DatabaseError(err_msg) => (
                StatusCode::INTERNAL_SERVER_ERROR, // 500 Internal Server Error
                format!("数据库操作失败: {} (Database operation failed: {})", err_msg, err_msg),
            ),
        };

        // --- 步骤 2: 构建 JSON 格式的响应体 ---
        // `serde_json::json!({ ... })`: 这是一个由 `serde_json` crate 提供的宏，用于方便地创建 `serde_json::Value` 类型的 JSON 对象。
        //   - `{"error": { ... }}`: 创建一个顶层 JSON 对象，其中包含一个名为 "error" 的键。
        //   - `"error": { "message": ..., "code": ... }`: "error" 键的值是另一个 JSON 对象，包含两个字段：
        //     - `"message"`: 其值为之前从 `match` 语句中获取的 `error_message_for_client` 字符串。
        //     - `"code"`: 其值为 HTTP 状态码的数字表示 (例如 404, 500)。
        //       `status.as_u16()` 将 `StatusCode` 枚举转换为其对应的 `u16` 数字。
        // `let body: Value = ...`: 将创建的 JSON 值赋给变量 `body`，其类型是 `serde_json::Value`。
        let body: Value = json!({
            "error": { // 错误信息包装在一个 "error" 对象内，是一种常见的 API 设计模式。
                "message": error_message_for_client, // 具体的错误消息文本。
                "code": status.as_u16()              // HTTP 状态码的数字形式。
            }
        });

        // --- 步骤 3: 组合状态码和 JSON 响应体为最终的 HTTP 响应 ---
        // `(status, Json(body))`: 创建一个元组，包含 HTTP 状态码 `status` 和一个 `axum::Json` 包装的响应体 `body`。
        //   - `Json(body)`: 将 `serde_json::Value` 类型的 `body` 包装在 `axum::Json` 中。
        //     当 `Json` 类型用作响应时，Axum 会自动：
        //       1. 将 `body` 序列化为 JSON 字符串。
        //       2. 设置 HTTP 响应的 `Content-Type` 头为 `application/json`。
        // ` ( ... ).into_response()`:
        //   Axum 为 `(StatusCode, Json<T>)` 这种元组类型也实现了 `IntoResponse` trait。
        //   因此，我们可以直接对这个元组调用 `.into_response()`，它会生成一个配置了正确状态码、
        //   `Content-Type` 头和 JSON 响应体的 `axum::response::Response` 对象。
        (status, Json(body)).into_response()
    }
}


// --- 辅助函数 (Helper Functions) ---
// 这些函数提供了一种便捷的方式来创建特定类型的 `AppError` 实例。
// 它们封装了错误变体的构造逻辑，使得在代码的其他地方创建错误更加简单和一致。
// 例如， statt `AppError::UserAlreadyExists("some_user".to_string())` zu schreiben,
// kann man `error::user_already_exists("some_user".to_string())` verwenden (wenn importiert).

// `pub fn resource_not_found(resource_type: &str, identifier: &str) -> AppError { ... }`
// - 这是一个通用的辅助函数，用于创建 `AppError::NotFound` 错误。
// - `resource_type: &str`: 描述未找到的资源类型 (例如 "用户", "产品")。 `&str` 是字符串切片。
// - `identifier: &str`: 描述未找到资源的标识符 (例如 用户名, 产品ID)。
// - `-> AppError`: 返回一个 `AppError::NotFound` 实例。
/// 通用辅助函数：创建资源未找到错误 (`AppError::NotFound`)。
pub fn resource_not_found(resource_type: &str, identifier: &str) -> AppError {
    AppError::NotFound(format!("未找到 {} '{}'", resource_type, identifier))
}

// `task_not_found` 和 `invalid_uuid` 函数已被注释掉，因为它们与之前基于 UUID 的任务管理功能相关，
// 而当前项目已转向基于 i32 ID 的用户实体和认证功能。
// 如果将来需要处理其他基于 UUID 的实体，可以重新引入类似的辅助函数。

// `pub fn user_already_exists(username: String) -> AppError { ... }`
// - `username: String`: 已存在的用户名。`String` 类型表示函数获取了用户名的所有权。
// - `-> AppError`: 返回 `AppError::UserAlreadyExists` 实例。
/// 辅助函数：创建"用户已存在"错误 (`AppError::UserAlreadyExists`)。
pub fn user_already_exists(username: String) -> AppError {
    AppError::UserAlreadyExists(username)
}

// `pub fn database_error(error_message: String) -> AppError { ... }`
// - `error_message: String`: 描述数据库错误的具体信息。
// - `-> AppError`: 返回 `AppError::DatabaseError` 实例。
/// 辅助函数：创建"数据库错误" (`AppError::DatabaseError`)。
pub fn database_error(error_message: String) -> AppError {
    AppError::DatabaseError(error_message)
}

// `pub fn invalid_credentials() -> AppError { ... }`
// - 此函数不接收参数，因为它总是创建同一个 `AppError::InvalidCredentials` 变体。
// - `-> AppError`: 返回 `AppError::InvalidCredentials` 实例。
/// 辅助函数：创建"无效凭证"错误 (`AppError::InvalidCredentials`)。
pub fn invalid_credentials() -> AppError {
    AppError::InvalidCredentials
}

/// 辅助函数：创建"无效令牌"错误 (`AppError::InvalidToken`)。
pub fn invalid_token() -> AppError {
    AppError::InvalidToken
}

/// 辅助函数：创建"缺失令牌"错误 (`AppError::MissingToken`)。
pub fn missing_token() -> AppError {
    AppError::MissingToken
}

/// 辅助函数：创建"未授权访问"错误 (`AppError::UnauthorizedAccess`)。
pub fn unauthorized_access() -> AppError {
    AppError::UnauthorizedAccess
}

[end of src/error.rs]
