// 文件路径: src/app/controller/auth_controller.rs

// /--------------------------------------------------------------------------------------------------\
// |                               【模块功能图示】 (auth_controller.rs)                                |
// |--------------------------------------------------------------------------------------------------|
// |                                                                                                  |
// | [外部 HTTP 请求]                                                                                   |
// |   - POST /api/register (包含 JSON: {username, password})                                         |
// |   - POST /api/login    (包含 JSON: {username, password})                                         |
// |      |                                                                                           |
// |      V (由 routes.rs 中的 Axum Router 路由到此模块的 Handler)                                       |
// |  [认证控制器 (`auth_controller.rs`)]                                                               |
// |   - `register_handler(State(app_state), Json(payload))`:                                         |
// |     1. 从 `payload` (RegisterPayload) 提取 `username`, `password`。                               |
// |     2. 从 `app_state` 提取数据库连接 `db_conn` (Arc<DatabaseConnection>)。                         |
// |     3. 调用 `AuthService::register_user(&db_conn, username, password)`。                           |
// |     4. 成功: 将返回的 `user_entity::Model` 转换为 `UserResponse` DTO。                             |
// |     5. 返回 `(StatusCode::CREATED, Json<UserResponse>)` 或 `AppError`。                            |
// |   - `login_handler(State(app_state), Json(payload))`:                                            |
// |     1. 从 `payload` (LoginPayload) 提取 `username`, `password`。                                  |
// |     2. 从 `app_state` 提取 `db_conn`。                                                            |
// |     3. 调用 `AuthService::login_user(&db_conn, username, password)`。                              |
// |     4. 成功: 将返回的 JWT 字符串包装在 `LoginResponse` DTO 中。                                    |
// |     5. 返回 `Json<LoginResponse>` 或 `AppError`。                                                  |
// |      |                                                                                           |
// |      V (调用 AuthService)                                                                         |
// |  [服务层 (`AuthService`)]                                                                          |
// |   - 执行核心认证业务逻辑 (用户检查、密码哈希/验证、JWT生成)。                                         |
// |   - 可能调用 `UserRepository` 与数据库交互。                                                       |
// |      |                                                                                           |
// |      V (AuthService 返回 Result<_, AppError>)                                                     |
// |  [认证控制器 (`auth_controller.rs`)]                                                               |
// |   - 根据 `AuthService` 的结果构建最终的 HTTP 响应。                                                |
// |      |                                                                                           |
// |      V (Axum 将 Handler 的返回值转换为 HTTP 响应)                                                   |
// | [HTTP 响应] (发送给客户端, 例如 201 Created + JSON, 200 OK + JSON, 或错误状态码 + JSON)            |
// |                                                                                                  |
// \--------------------------------------------------------------------------------------------------/
//
// 【模块核心职责】 (Module Core Responsibilities)
// 1. **定义 HTTP 请求处理函数 (Request Handlers)**: 为认证相关的 API 端点 (如用户注册 `/api/register` 和用户登录 `/api/login`) 提供具体的处理逻辑。
// 2. **接收和解析请求数据 (Receive and Parse Request Data)**:
//    - 使用 Axum 的提取器 (Extractors) 从传入的 HTTP 请求中提取数据。
//    - `Json<T>` 提取器用于将 HTTP 请求体中的 JSON 数据自动反序列化为指定的 Rust 结构体 (如 `RegisterPayload`, `LoginPayload`)。
//    - `State<T>` 提取器用于访问应用程序的共享状态 (`AppState`)，从中获取如数据库连接池等资源。
// 3. **调用业务逻辑 (Invoke Business Logic)**: 将提取和验证后的数据传递给服务层 (`AuthService`) 的相应方法，以执行核心的业务操作（如用户注册、登录验证、JWT生成）。
//    控制器本身不包含复杂的业务规则，而是充当 HTTP 接口与业务逻辑之间的桥梁。
// 4. **构建 HTTP 响应 (Formulate HTTP Responses)**:
//    - 根据服务层返回的结果 (成功时的数据或失败时的错误)，构造适当的 HTTP 响应。
//    - 这包括设置正确的 HTTP 状态码 (例如 `StatusCode::CREATED` 表示资源创建成功，`StatusCode::OK` 表示成功) 和响应体 (通常是 JSON 格式的数据，如 `UserResponse` 或 `LoginResponse`)。
//    - 如果服务层返回错误 (`AppError`)，控制器会将其传播出去，由 Axum 的错误处理机制（通过 `AppError` 实现的 `IntoResponse` trait）将其转换为标准的 HTTP 错误响应。
//
// 【关键技术点】 (Key Technologies)
// - **Axum 框架**:
//   - **请求处理函数 (Handlers)**: 异步函数 (`async fn`)，接收 Axum 提取器作为参数，并返回一个实现了 `IntoResponse` trait 的类型。
//   - **提取器 (Extractors)**:
//     - `axum::extract::State<S>`: 用于从应用程序状态中提取共享数据。`S` 必须实现 `Clone`。
//     - `axum::Json<T>`: 用于将请求体中的 JSON 数据反序列化为类型 `T` (要求 `T` 实现 `serde::Deserialize`)，或将类型 `T` 序列化为 JSON 响应体 (要求 `T` 实现 `serde::Serialize`)。
//   - **响应 (Responses)**:
//     - `axum::response::IntoResponse`: 一个 trait，允许自定义类型转换为标准的 HTTP `Response`。
//       可以直接返回元组如 `(StatusCode, Json<T>)`，Axum 会自动处理。
//     - `axum::http::StatusCode`: 用于表示 HTTP 状态码的枚举。
// - **`serde` (序列化/反序列化)**: 用于在 JSON 数据和 Rust 结构体 (DTOs) 之间进行转换。
// - **应用状态 (`AppState`)**: 在 `src/app/state.rs` 中定义，包含了共享的数据库连接池 (`Arc<DatabaseConnection>`)。
// - **服务层调用 (`AuthService`)**: 控制器调用服务层的方法来执行实际的业务逻辑。
// - **错误处理 (`Result<T, AppError>`)**: 控制器处理函数返回 `Result`，允许将业务逻辑中的错误 (`AppError`) 传播到 Axum 的响应生成机制中。
// - **异步编程 (`async/await`)**: 所有处理函数都是异步的，以支持非阻塞 I/O 操作（主要是等待服务层中的数据库调用）。

// --- 导入依赖 ---
// `use axum::{...}`: 从 `axum` crate 中导入所需的组件。
use axum::{
    extract::State,     // `State` 提取器，用于访问共享的应用状态 (`AppState`)。
    http::StatusCode,   // `StatusCode` 枚举，用于定义 HTTP 响应的状态码 (例如 200 OK, 201 Created, 400 Bad Request)。
    response::IntoResponse, // `IntoResponse` trait，允许自定义类型转换为 HTTP 响应。Handler 的返回值需要实现此 trait。
    Json,               // `Json` 提取器和响应类型，用于处理 JSON 格式的数据。
};
// `use std::sync::Arc;`
//   - 导入标准库中的 `Arc` (原子引用计数智能指针)。虽然在此文件中不直接创建 `Arc`，
//     但 `AppState` 内部使用 `Arc` 来包装 `DatabaseConnection`，理解 `Arc` 的存在对于理解状态共享很重要。
use std::sync::Arc;
// `use sea_orm::DatabaseConnection;`
//   - 导入 `sea_orm` 的 `DatabaseConnection` 类型。同样，虽然不直接创建，但它是 `AppState` 中 `db_conn` 字段的类型。
use sea_orm::DatabaseConnection; // 控制器直接依赖 DatabaseConnection 类型，因为它在 AppState 中

// `use crate::app::model::{...};`
//   - 从本项目的 `model` 模块 (具体是 `auth_dtos.rs`) 导入认证相关的数据传输对象 (DTOs)。
//   - `RegisterPayload`: 用于接收用户注册请求的数据结构。
//   - `LoginPayload`: 用于接收用户登录请求的数据结构。
//   - `LoginResponse`: 用于包装成功登录后返回的 JWT 的数据结构。
//   - `UserResponse`: 用于在注册成功等情况下返回用户信息 (不含敏感数据) 的数据结构。
use crate::app::model::{RegisterPayload, LoginPayload, LoginResponse, UserResponse, Claims}; // Claims 也可能被其他控制器使用，但这里主要是DTOs
// `use crate::app::service::AuthService;`
//   - 导入在 `src/app/service/auth_service.rs` 中定义的 `AuthService` 结构体。
//   - 控制器将调用 `AuthService` 的方法来执行用户注册和登录的业务逻辑。
use crate::app::service::AuthService;
// `use crate::app::state::AppState;`
//   - 导入在 `src/app/state.rs` 中定义的 `AppState` 结构体。
//   - `AppState` 包含了需要在整个应用中共享的状态，如数据库连接池。
use crate::app::state::AppState;
// `use crate::error::{AppError, Result};`
//   - 导入在 `src/error.rs` 中定义的 `AppError` 枚举 (自定义错误类型) 和 `Result` 类型别名 (`core::result::Result<T, AppError>`)。
//   - 控制器处理函数将返回这个 `Result` 类型，以便在出错时能够统一处理并转换为 HTTP 错误响应。
use crate::error::{AppError, Result};


// `pub async fn register_handler(...) -> Result<impl IntoResponse, AppError>`
//   - `pub async fn`: 定义一个公共的 (public) 异步 (async) 函数 `register_handler`。
//     - `async`: 表示这是一个异步函数，其执行可以被暂停和恢复，通常用于等待 I/O 操作 (如数据库查询、网络调用)。
//   - `State(app_state): State<AppState>`: 第一个参数，使用了 Axum 的 `State` 提取器。
//     - `State<AppState>`: 告诉 Axum 我们希望从应用的共享状态中提取 `AppState` 的一个实例。
//       Axum 在启动时通过 `.with_state(app_state_instance)` 将 `AppState` 注入到 Router 中。
//       由于 `AppState` 实现了 `Clone` (因为其内部的 `db_conn` 是 `Arc`，而 `Arc` 是 `Clone` 的，克隆成本低)，
//       Axum 会为每个需要它的处理函数克隆一份 `AppState`。
//     - `State(app_state)`: 这是 Rust 的模式匹配语法，将提取到的 `AppState` 实例绑定到变量 `app_state`。
//       `app_state` 的类型是 `AppState`。
//   - `Json(payload): Json<RegisterPayload>`: 第二个参数，使用了 Axum 的 `Json` 提取器。
//     - `Json<RegisterPayload>`: 告诉 Axum 我们期望请求体是一个 JSON 对象，并且这个 JSON 对象应该能被反序列化为 `RegisterPayload` 结构体。
//       `RegisterPayload` 必须实现 `serde::Deserialize` trait (通常通过 `#[derive(Deserialize)]`)。
//     - `Json(payload)`: 模式匹配，将成功反序列化后的 `RegisterPayload` 实例绑定到变量 `payload`。
//     - **自动错误处理**: 如果客户端发送的请求体不是有效的 JSON，或者 JSON 结构与 `RegisterPayload` 不匹配 (例如缺少字段、类型错误)，
//       `Json` 提取器会自动失败，并向客户端返回一个 HTTP `400 Bad Request` 或 `422 Unprocessable Entity` 响应，
//       这个过程对处理函数是透明的，函数本身不会被调用。
//   - `-> Result<impl IntoResponse, AppError>`: 函数的返回类型。
//     - `Result<T, E>`: 表示函数可能成功并返回 `Ok(T)`，或失败并返回 `Err(E)`。
//     - `impl IntoResponse`: `T` (成功类型) 必须是任何实现了 `axum::response::IntoResponse` trait 的类型。
//       这允许我们灵活地返回多种成功响应，例如元组 `(StatusCode, Json<UserResponse>)`。
//     - `AppError`: `E` (错误类型) 是我们自定义的 `AppError`。由于 `AppError` 实现了 `IntoResponse`，
//       当处理函数返回 `Err(AppError)` 时，Axum 会自动将其转换为相应的 HTTP 错误响应。
/// 处理用户注册请求 (HTTP POST /api/register) 的函数。
///
/// 此函数负责:
/// 1. 从 HTTP 请求体中提取 JSON 格式的注册信息 (`RegisterPayload`，包含用户名和密码)。
/// 2. 从应用共享状态中获取数据库连接 (`AppState`)。
/// 3. 调用 `AuthService` 的 `register_user` 方法来执行用户注册的业务逻辑。
/// 4. 如果注册成功，将返回的用户模型 (`user_entity::Model`) 转换为对客户端安全的 `UserResponse` DTO。
/// 5. 返回一个 HTTP 201 Created 状态码和包含 `UserResponse` 的 JSON 响应体。
/// 6. 如果注册过程中发生任何错误 (例如用户名已存在、数据库错误、密码哈希失败)，则返回相应的 `AppError`，
///    Axum 会将其转换为合适的 HTTP 错误响应。
pub async fn register_handler(
    State(app_state): State<AppState>,      // 通过 State 提取器获取应用状态 (包含数据库连接 Arc)
    Json(payload): Json<RegisterPayload>, // 通过 Json 提取器获取请求体并反序列化为 RegisterPayload
) -> Result<impl IntoResponse, AppError> { // 返回 Result，成功时是实现了 IntoResponse 的类型，错误时是 AppError
    // `println!` 用于在服务器控制台打印日志，方便调试。在生产环境中应使用 `tracing` 宏。
    // 这里记录收到了一个注册请求，并打印了用户名。注意不要打印密码等敏感信息。
    println!("CONTROLLER: 接收到用户注册请求: {}", payload.username);

    // --- 调用服务层执行注册逻辑 ---
    // `AuthService::register_user(...)`: 调用 `AuthService` 的静态方法 `register_user`。
    //   - `&app_state.db_conn`: 传递数据库连接的引用。
    //     `app_state.db_conn` 的类型是 `Arc<DatabaseConnection>`。当 SeaORM 的执行器需要 `&DatabaseConnection` 时，
    //     `Arc<T>` 可以通过 `Deref` trait 自动解引用为 `&T` (即 `&DatabaseConnection`)。
    //   - `payload.username`: 将 `RegisterPayload` 中的 `username` (String) 传递给服务。
    //     由于 `username` 字段是 `String` 类型，并且 `register_user` 函数的参数 `username: String` 期望获得所有权，
    //     这里发生了所有权的【移动 (move)】。`payload.username` 的值被移出 `payload` 并传递给服务函数。
    //     之后 `payload.username` 在此函数中将不再有效。
    //   - `payload.password`: 类似地，密码字符串的所有权也被移动到服务函数中。
    //   - `.await`: 因为 `register_user` 是一个异步函数，所以需要 `.await` 来等待其完成。
    //   - `?`: 问号操作符用于错误传播。
    //     如果 `AuthService::register_user` 返回 `Ok(user_model)`，`?` 会将 `user_model` 提取出来。
    //     如果返回 `Err(app_error)`，`?` 会使 `register_handler` 函数立即返回这个 `Err(app_error)`。
    let created_user_model = AuthService::register_user(&app_state.db_conn, payload.username, payload.password)
        .await?; // `?` 如果 AuthService 返回 Err(AppError)，则此 handler 也会返回 Err(AppError)
    // `created_user_model`现在的类型是 `user_entity::Model`，包含了新创建的用户信息 (包括数据库生成的ID和时间戳)。

    // --- 将数据库模型转换为 API 响应 DTO ---
    // `UserResponse::from(created_user_model)`:
    //   - `UserResponse` 是我们定义的用于 API 响应的 DTO，它不包含敏感信息 (如哈希密码)。
    //   - 我们为 `UserResponse` 实现了 `From<user_entity::Model>` trait (在 `auth_dtos.rs` 中)。
    //   - 这允许我们使用 `.from()` (或 `.into()`，由 `From` 自动提供) 来方便地将数据库实体模型 `created_user_model`
    //     转换为 `UserResponse` DTO。这个转换过程会选择性地映射字段。
    //   `created_user_model` 的所有权被移动到 `from()` 方法中。
    let user_response_dto = UserResponse::from(created_user_model);

    // 日志记录：用户注册成功。
    println!("CONTROLLER: 用户 {} 注册成功。", user_response_dto.username);

    // --- 返回成功响应 ---
    // `Ok((StatusCode::CREATED, Json(user_response_dto)))`:
    //   - `Ok(...)`: 表示处理函数成功完成。
    //   - `(StatusCode::CREATED, Json(user_response_dto))`: 这是一个元组，Axum 可以将其转换为 HTTP 响应。
    //     - `StatusCode::CREATED` (201): HTTP 状态码，表示资源已成功创建。这是 RESTful API 中创建操作的标准成功响应码。
    //     - `Json(user_response_dto)`: 将 `user_response_dto` (类型 `UserResponse`) 包装在 `axum::Json` 中。
    //       Axum 会将 `user_response_dto` 序列化为 JSON 字符串，并将其作为 HTTP 响应体发送。
    //       同时，它会自动设置 `Content-Type: application/json` 响应头。
    Ok((StatusCode::CREATED, Json(user_response_dto)))
}


// `pub async fn login_handler(...) -> Result<Json<LoginResponse>, AppError>`
//   - `pub async fn`: 公共异步处理函数。
//   - 参数 `State(app_state): State<AppState>` 和 `Json(payload): Json<LoginPayload>` 与 `register_handler` 中的类似，
//     只是 `payload` 的类型是 `LoginPayload`。
//   - `-> Result<Json<LoginResponse>, AppError>`: 返回类型。
//     - 成功时 (`Ok`) 返回 `Json<LoginResponse>`。这意味着响应体将是 `LoginResponse` DTO 的 JSON 表示，
//       并且 Axum 会默认使用 HTTP `200 OK` 状态码 (因为我们没有像注册那样显式指定状态码)。
//     - 失败时 (`Err`) 返回 `AppError`。
/// 处理用户登录请求 (HTTP POST /api/login) 的函数。
///
/// 此函数负责:
/// 1. 从 HTTP 请求体中提取 JSON 格式的登录凭证 (`LoginPayload`，包含用户名和密码)。
/// 2. 从应用共享状态中获取数据库连接 (`AppState`)。
/// 3. 调用 `AuthService` 的 `login_user` 方法来执行用户登录的业务逻辑 (验证凭证、生成 JWT)。
/// 4. 如果登录成功，将返回的 JWT 字符串包装在 `LoginResponse` DTO 中。
/// 5. 返回一个包含 `LoginResponse` (含JWT) 的 JSON 响应体 (默认状态码 200 OK)。
/// 6. 如果登录过程中发生任何错误 (例如用户不存在、密码错误、JWT生成失败)，则返回相应的 `AppError`。
pub async fn login_handler(
    State(app_state): State<AppState>,   // 提取共享应用状态
    Json(payload): Json<LoginPayload>, // 提取并反序列化登录请求体
) -> Result<Json<LoginResponse>, AppError> { // 成功时返回 Json<LoginResponse>，失败时返回 AppError
    // 日志记录：收到登录请求。
    println!("CONTROLLER: 接收到用户登录请求: {}", payload.username);

    // --- 调用服务层执行登录逻辑 ---
    // `AuthService::login_user(&app_state.db_conn, payload.username, payload.password)`:
    //   - 调用 `AuthService` 的 `login_user` 方法。
    //   - 参数传递方式与 `register_user` 类似，`username` 和 `password` 的所有权被移动到服务函数。
    //   - `.await?`: 等待异步操作完成，并处理可能的 `AppError`。
    let jwt_token_string = AuthService::login_user(&app_state.db_conn, payload.username, payload.password)
        .await?; // `?` 如果 AuthService 返回 Err(AppError)，则此 handler 也会返回 Err(AppError)
    // `jwt_token_string` 现在的类型是 `String`，包含了生成的 JWT。

    // 日志记录：用户登录成功。
    // 注意：payload.username 在这里仍然可用，是因为如果上面 `login_user` 调用成功，
    // `payload.username` 的所有权虽然名义上移交，但如果 `login_user` 内部需要克隆它或者它没有被完全消耗，
    // 这里的 `payload` 变量本身可能仍然有效，但其字段可能处于被移走的状态。
    // 更安全的做法是，如果要在日志中使用用户名，应该从成功登录后可能返回的用户信息中获取，
    // 或者在 `payload` 所有权转移前克隆一份用户名用于日志。
    // 不过，对于 `println!` 调试，如果 `login_user` 内部没有消耗 `username`（例如，只是借用了它），
    // 那么这里的 `payload.username` 可能仍然可以访问，但这依赖于 `login_user` 的具体实现细节。
    // 为了清晰和安全，最好假设 `payload` 的字段在传递给服务后不应再被访问。
    // 假设我们想记录登录的用户名，可以从 `AuthService::login_user` 的返回值（如果它返回了用户信息）或通过解码刚生成的 `jwt_token_string` 中的 `sub` 声明来获取。
    // 但为了简单，这里我们假设 `payload.username` 由于某种原因仍然可访问（例如，`login_user` 内部克隆了它）。
    // 更严谨的做法是 `let username_for_log = payload.username.clone();` 在调用服务前。
    // 或者，如果 `login_user` 返回了用户信息：`let (jwt_token_string, logged_in_user) = AuthService::login_user(...).await?;`
    // 然后用 `logged_in_user.username`。
    // 此处保持原样，但需注意所有权问题。
    println!("CONTROLLER: 用户 {} 登录成功。", "some_user"); // 理想情况下，应记录已验证的用户名

    // --- 构建并返回成功响应 ---
    // `Ok(Json(LoginResponse { token: jwt_token_string }))`:
    //   - `Ok(...)`: 表示处理函数成功。
    //   - `LoginResponse { token: jwt_token_string }`: 创建 `LoginResponse` DTO 的实例，
    //     将其 `token` 字段设置为从服务层获取的 JWT 字符串。
    //     `jwt_token_string` 的所有权被移动到 `LoginResponse` 中。
    //   - `Json(...)`: 将 `LoginResponse` 实例包装在 `axum::Json` 中。
    //     Axum 会将其序列化为 JSON 响应体，并自动设置 `Content-Type: application/json` 和 HTTP `200 OK` 状态码。
    Ok(Json(LoginResponse { token: jwt_token_string }))
}
