// 文件路径: src/app/controller/protected_controller.rs

// /--------------------------------------------------------------------------------------------------\
// |                              【模块功能图示】 (protected_controller.rs)                             |
// |--------------------------------------------------------------------------------------------------|
// |                                                                                                  |
// | [客户端 HTTP GET 请求 /api/protected_data]                                                          |
// |   (Header: `Authorization: Bearer <valid_jwt_token>`)                                            |
// |      |                                                                                           |
// |      V (由 routes.rs 路由配置)                                                                     |
// |  [JWT认证中间件 (`jwt_auth_middleware`)]                                                            |
// |   1. 提取并验证 JWT。                                                                              |
// |   2. 如果有效, 将解码后的 `Claims` 存入请求的扩展 (Request Extensions)。                             |
// |   3. 调用 `next.run(req)` 将请求传递给此 Handler。                                                 |
// |   (如果 JWT 无效或缺失, 中间件会直接返回错误响应, 不会到达此 Handler)                                  |
// |      |                                                                                           |
// |      V                                                                                           |
// |  [受保护控制器 (`protected_controller.rs`)]                                                        |
// |   - `protected_data_handler(Extension(claims))`:                                                 |
// |     1. 使用 `Extension` 提取器从请求扩展中获取 `Claims`。                                          |
// |     2. `claims` 变量现在持有已认证用户的信息 (如用户ID `claims.sub`)。                             |
// |     3. 可以基于 `claims` 中的信息执行用户相关的逻辑 (例如, 返回用户特定的数据)。                       |
// |     4. 构建并返回一个 JSON 响应, 其中可能包含来自 `claims` 的数据。                                 |
// |      |                                                                                           |
// |      V (Axum 将 Handler 的返回值转换为 HTTP 响应)                                                   |
// | [HTTP 响应] (例如: 200 OK + JSON Body: {"message": "...", "user_id": "..."})                    |
// |                                                                                                  |
// \--------------------------------------------------------------------------------------------------/
//
// 【模块核心职责】 (Module Core Responsibilities)
// 1. **定义受保护的请求处理函数 (Define Protected Request Handlers)**: 此模块包含那些只有经过认证的用户才能访问的 API 端点的处理逻辑。
// 2. **访问认证信息 (Access Authentication Information)**: 处理函数依赖于前置的认证中间件 (如 `jwt_auth_middleware`) 来验证用户身份。
//    一旦身份验证成功，中间件会将解码后的 JWT 声明 (`Claims`) 放入请求的“扩展 (extensions)”中。
//    此模块中的处理函数随后使用 Axum 的 `Extension` 提取器来获取这些 `Claims`，从而得知当前是哪个用户在访问。
// 3. **返回受保护的数据或执行受保护的操作 (Return Protected Data / Perform Protected Actions)**:
//    根据认证用户的身份 (`claims.sub` 通常是用户ID)，处理函数可以：
//    - 返回该用户有权访问的特定数据。
//    - 执行只有认证用户才能进行的操作。
//    - 在本示例中，它简单地返回一条消息，确认访问受保护数据成功，并回显部分声明信息。
//
// 【关键技术点】 (Key Technologies)
// - **Axum 框架**:
//   - **请求处理函数 (Handlers)**: 异步函数 (`async fn`)，用于处理特定的 HTTP 请求。
//   - **`Extension` 提取器 (`axum::extract::Extension`)**: Axum 提供的一种机制，用于从请求的“扩展 (extensions)”中提取数据。
//     请求扩展是一个类型映射 (type map)，允许中间件将任意类型的数据附加到请求上，供后续的中间件或处理函数使用。
//     这是在 Axum 中实现中间件向处理函数传递数据（如认证信息）的标准方式。
//   - **JSON 响应 (`axum::Json`)**: 用于将 Rust 结构体或 `serde_json::Value` 序列化为 JSON HTTP 响应体。
//   - **`IntoResponse` Trait**: 处理函数的返回值必须实现此 trait，以便 Axum 能将其转换为标准的 HTTP `Response`。
//     元组如 `(StatusCode, Json<T>)` 或单独的 `Json<T>` 都实现了 `IntoResponse`。
// - **JWT `Claims` 结构体**: 在 `src/app/model/auth_dtos.rs` 中定义，代表从 JWT 中解码出的声明数据 (如用户ID `sub`，过期时间 `exp`)。
// - **`serde_json::json!` 宏**: 一个方便的宏，用于动态创建 `serde_json::Value` 类型的 JSON 值，常用于构建临时的 JSON 响应体。
// - **中间件协作**: 此控制器的功能高度依赖于前置的 `jwt_auth_middleware`。如果认证中间件未能成功验证并注入 `Claims`，
//   `Extension<Claims>` 提取器会导致请求失败（通常是 HTTP 500 内部服务器错误，因为 Axum 无法找到所需的扩展）。

// --- 导入依赖 ---
// `use axum::{...}`: 从 `axum` crate 中导入所需的组件。
use axum::{
    extract::Extension, // `Extension` 提取器，用于从请求的扩展中获取由中间件添加的数据。
    http::StatusCode,   // `StatusCode` 枚举，用于定义 HTTP 响应的状态码。
    response::IntoResponse, // `IntoResponse` trait，处理函数的返回值需要实现它。
    Json,               // `Json` 类型，用于将数据序列化为 JSON 响应体。
};
// `use serde_json::json;`
//   - 从 `serde_json` crate 导入 `json!` 宏。
//   - `json!` 宏允许我们使用类似 JSON 的语法在 Rust 代码中方便地创建 `serde_json::Value` 实例。
//     例如: `json!({"name": "Alice", "age": 30})`。
use serde_json::json;
// `use crate::app::model::Claims;`
//   - 从本项目的 `model` 模块 (具体是 `auth_dtos.rs`) 导入 `Claims` 结构体。
//   - `Claims` 结构体定义了 JWT 中包含的声明数据 (如用户ID `sub`、过期时间 `exp` 等)。
//   - 这个处理函数期望 `jwt_auth_middleware` 已经验证了 JWT 并将解码后的 `Claims` 实例放入了请求扩展中。
use crate::app::model::Claims;


// `pub async fn protected_data_handler(...) -> impl IntoResponse`
//   - `pub async fn`: 定义一个公共的 (public) 异步 (async) 函数 `protected_data_handler`。
//     这是处理 `/api/protected_data` 路由的函数。
//   - `Extension(claims): Extension<Claims>`: 函数的唯一参数，使用了 Axum 的 `Extension` 提取器。
//     - `Extension<Claims>`: 告诉 Axum 我们希望从请求的“扩展 (extensions)”数据存储中提取一个 `Claims` 类型的实例。
//       请求扩展是一个类型安全的哈希映射 (type map)，允许中间件将任意类型的数据附加到请求对象上，供后续的中间件或处理函数使用。
//     - `Extension(claims)`: 这是 Rust 的模式匹配语法。如果请求扩展中存在 `Claims` 类型的实例，
//       `Extension` 提取器会成功提取它，并将其绑定到名为 `claims` 的变量。`claims` 的类型是 `Claims`。
//     - **关键依赖**: 此提取器能否成功，完全依赖于在它之前的某个中间件（在本项目中是 `jwt_auth_middleware`）
//       是否已经成功验证了 JWT，并将解码后的 `Claims` 实例通过 `request.extensions_mut().insert(decoded_claims)` 添加到了请求扩展中。
//     - **如果 `Claims` 未找到**: 如果 `jwt_auth_middleware` 因为令牌无效或缺失而没有运行到插入 `Claims` 的那一步，
//       或者中间件根本没有被应用到这个路由上，那么当 Axum 尝试为这个处理函数提取 `Extension<Claims>` 时，
//       它会发现扩展中不存在 `Claims` 类型的实例。在这种情况下，`Extension` 提取器会失败，
//       Axum 默认会返回一个 HTTP `500 Internal Server Error` 响应。
//       （可以通过实现自定义的 `FromRequestParts` 来改变这种默认行为，但通常我们依赖中间件确保 `Claims` 的存在。）
//   - `-> impl IntoResponse`: 函数的返回类型。
//     - `impl IntoResponse`: 表示此函数返回一个实现了 `IntoResponse` trait 的类型。
//       这让我们可以灵活地返回多种能被 Axum 转换为 HTTP 响应的类型，例如 `Json<T>`，或者元组 `(StatusCode, Json<T>)`。
/// 一个受保护的请求处理函数示例。
///
/// 此函数演示了如何访问由 `jwt_auth_middleware` 中间件验证和注入的 JWT `Claims`。
/// 只有当请求包含有效的 JWT (通过 `Authorization: Bearer <token>` 头提供) 时，
/// `jwt_auth_middleware` 才会将请求传递到此处理函数，并在此之前将 `Claims` 放入请求扩展中。
///
/// # 参数
/// * `Extension(claims): Extension<Claims>`: Axum 的 `Extension` 提取器。
///   它从请求的扩展中提取由前置中间件 (如 `jwt_auth_middleware`) 插入的 `Claims` 实例。
///   如果 `Claims` 不存在于扩展中 (例如，中间件未运行或验证失败后提前返回)，
///   则 `Extension` 提取器会失败，导致 Axum 返回一个 HTTP 500 错误。
///
/// # 返回值
/// * `impl IntoResponse`: 返回一个可以被 Axum 转换为 HTTP 响应的类型。
///   在本例中，它返回一个包含从 `Claims` 中提取的信息的 JSON 对象，以及一个成功状态码。
pub async fn protected_data_handler(
    Extension(claims): Extension<Claims>, // 从请求扩展中提取 JWT Claims
) -> impl IntoResponse {
    // `println!` (或在生产中使用 `tracing::info!`) 用于在服务器控制台记录信息。
    // 这里我们打印出已认证用户的 `sub` (subject，通常是用户ID)，以证明我们成功访问了 `Claims`。
    // 这也常用于审计日志，记录哪个用户访问了哪个受保护资源。
    println!("CONTROLLER (Protected): 用户 '{}' 成功访问受保护的数据。", claims.sub);

    // --- 构建并返回成功的 JSON 响应 ---
    // `(StatusCode::OK, Json(json!({ ... })))`: 返回一个元组，包含 HTTP 状态码和 JSON 响应体。
    //   - `StatusCode::OK` (200): 表示请求已成功处理。
    //   - `Json(...)`: 将后续的 `serde_json::Value` 包装在 `axum::Json` 中，以便 Axum 将其序列化为 JSON 响应体，
    //     并设置 `Content-Type: application/json` 头。
    //   - `json!({ ... })`: 使用 `serde_json::json!` 宏创建一个动态的 JSON 对象 (`serde_json::Value`)。
    //     - `"message"`: 一个简单的消息字符串。
    //     - `"user_id"`: 从 `claims.sub` 获取的已认证用户ID。
    //     - `"expires_at"`: 从 `claims.exp` 获取的令牌过期时间戳 (通常是 Unix 时间戳)。
    //     - `"issued_at"`: 从 `claims.iat` 获取的令牌签发时间戳。
    //     将这些信息返回给客户端，可以用于调试或向用户展示其会话信息。
    (
        StatusCode::OK, // HTTP 200 OK 状态码
        Json(json!({
            "message": "这是一条受保护的数据，只有经过认证的用户才能访问。",
            "user_id": claims.sub, // 从 JWT Claims 中获取的用户 ID
            "token_expires_at_timestamp": claims.exp, // 从 JWT Claims 中获取的过期时间戳
            "token_issued_at_timestamp": claims.iat,  // 从 JWT Claims 中获取的签发时间戳
        })),
    )
}
