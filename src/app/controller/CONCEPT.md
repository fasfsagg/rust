# 控制器层 (Controller) 核心概念

## 1. 职责与定位

控制器层 (`src/app/controller/`) 是 Axum Web 应用程序中处理 HTTP 请求的【直接入口点】。它位于路由层 (`routes.rs`) 和服务层 (`service`) 之间，充当着 HTTP 世界与应用程序内部业务逻辑之间的【翻译官】和【协调者】。

主要职责：

- **接收和解析请求**: 接收由路由匹配到的 HTTP 请求。
- **提取数据**: 使用 Axum 的【提取器 (Extractors)】从请求的各个部分（路径、查询参数、请求头、请求体 JSON/表单）提取所需数据。
- **调用服务层**: 将提取并（可能）转换后的数据传递给服务层的相应函数，触发业务逻辑执行。
- **处理结果**: 接收服务层返回的 `Result`（包含成功数据或错误）。
- **构造响应**: 将服务层的结果转换为标准的 HTTP 响应（状态码、响应体、响应头），利用 Axum 的 `IntoResponse` 机制。
- **WebSocket 处理**: (如需) 处理 WebSocket 升级请求，并将建立的连接交给专门的处理逻辑。

**关键点**: 控制器应保持"轻薄"，避免包含复杂的业务逻辑。主要关注点是 HTTP 交互和数据流转。

## 2. Axum 核心要素

控制器层大量使用了 Axum 的核心特性：

- **处理函数 (Handler)**:
    - 形式: `async fn handler_name(extractor1: Type1, extractor2: Type2, ...) -> ReturnType`
    - 异步 (`async fn`): 必须是异步函数，以便在等待 I/O (如服务层调用) 时不阻塞线程。
    - 参数: 必须是实现了 `FromRequestParts` 或 `FromRequest` 的提取器类型。
    - 返回值: 必须是实现了 `IntoResponse` 的类型。
- **提取器 (Extractor)**:
    - **`State<AppState>`**: 从应用程序共享状态中提取数据。`AppState` (定义在 `src/app/state.rs`) 现在包含 `db: DatabaseConnection` (SeaORM 数据库连接池) 和 `config: AppConfig` (应用配置，如 JWT 密钥)。
    - **`Path<T>`**: 从 URL 路径参数中提取数据。例如，`/tasks/:id` 中的 `id` 现在通过 `Path<i32>` 提取为整数。
    - **`Json<T>`**: 从请求体中反序列化 JSON 数据。`T` 必须实现 `serde::Deserialize`。用于接收如 `RegisterUserPayload`, `CreateTaskPayload` 等。
    - **`Claims` (自定义提取器)**: 对于受保护的路由，处理器参数中直接使用 `claims: Claims`。这是通过为 `Claims` 类型实现 `FromRequestParts<AppState>` 来实现的（在 `src/app/middleware/auth_middleware.rs` 中）。该提取器负责从 `Authorization` 头部解析 Bearer Token，验证 JWT，并在成功时提供 `Claims` 数据。如果验证失败，它会直接返回错误响应 (如 401 Unauthorized)，阻止处理器执行。
    - **`WebSocketUpgrade`**: 用于处理 WebSocket 协议升级请求。
- **响应 (`IntoResponse`)**:
    - Handler 的返回值必须能转换为 `axum::response::Response`。
    - 常用的有 `(StatusCode, Json<T>)`，`StatusCode` 单独返回，或 `Result<T, AppError>` (其中 `T` 和 `AppError` 都实现 `IntoResponse`)。

## 3. 共享状态 (`AppState`)

- **定义**: 位于 `src/app/state.rs`。
- **内容**: 封装了 `db: DatabaseConnection` (SeaORM 的数据库连接池) 和 `config: AppConfig` (应用配置)。
- **实现 `Clone`**: Axum 要求状态必须是 `Clone` 的。`DatabaseConnection` 和 `AppConfig` 都支持克隆。
- **注入与访问**: 在 `startup.rs` 中创建并通过 `.with_state()` 注入到 `Router`。Handler 和自定义提取器 (如 `Claims::from_request_parts`) 通过 `State<AppState>` 访问。

## 4. 请求处理生命周期 (简化，包含认证)

1.  HTTP 请求到达。
2.  Axum 中间件 (Middleware) 执行 (如日志中间件)。
3.  Axum 路由层 (`Router` in `routes.rs`) 根据请求路径和方法匹配到对应的 Handler。
4.  Axum 尝试从请求中运行 Handler 参数所需的所有【提取器】:
    *   `State<AppState>`: 提取共享状态。
    *   `Json<Payload>`: 解析请求体。
    *   `Path<ParamType>`: 解析路径参数。
    *   `Claims`: (如果处理器参数中有此项) `Claims::from_request_parts` 执行：
        *   读取 `Authorization` 头部。
        *   验证 Bearer Token (签名、有效期等)，使用 `AppState` 中的 JWT 配置。
        *   如果Token无效或缺失，提取器返回 `AppError::Unauthorized`，请求处理终止，直接发送 401 响应。Handler **不执行**。
        *   如果Token有效，提取器返回 `Ok(Claims)`。
    *   如果任何非认证相关的提取器失败（如 JSON 解析错误），Axum 直接返回相应错误响应，Handler **不执行**。
5.  如果所有提取器成功，Axum 调用 Handler 函数，并将提取到的数据作为参数传入。
6.  Handler 函数执行：
    *   调用服务层函数 (`service::*`)，这些服务现在使用 SeaORM 与数据库交互，并 `.await` 其结果。
    *   处理服务层返回的 `Result`。
    *   构造一个实现了 `IntoResponse` 的返回值。
7.  Axum 将 Handler 返回的值转换为 `Response` 对象。
8.  Axum 中间件再次执行 (可以修改响应)。
9.  Axum 将最终的 `Response` 发送回客户端。

## 5. 具体控制器模块

### 5.1. `auth_controller.rs`
*   **职责**: 处理用户注册 (`/api/register`) 和登录 (`/api/login`) 的 HTTP 请求。
*   **`register_handler(State<AppState>, Json<RegisterUserPayload>)`**:
    *   调用 `AuthService::register_user`，传递数据库连接 (`app_state.db`) 和注册信息。
    *   返回包含新用户信息 (`UserResponse`) 的 JSON 响应。
*   **`login_handler(State<AppState>, Json<LoginUserPayload>)`**:
    *   调用 `AuthService::login_user`，传递数据库连接 (`app_state.db`)、JWT 配置 (`app_state.config`) 和登录凭据。
    *   成功则返回包含 JWT (`LoginResponse`) 的 JSON 响应。

### 5.2. `task_controller.rs`
*   **职责**: 处理任务相关的 CRUD 操作和 WebSocket 连接。
*   **CRUD 操作**:
    *   例如 `create_task(State<AppState>, Json<CreateTaskPayload>)`。
    *   调用 `task_service` 中相应的方法，这些方法现在使用 SeaORM 与数据库交互。
    *   路径参数 `:id` 现在通过 `Path<i32>` 提取。
*   **受保护路由示例 (`protected_data_handler`)**:
    *   处理器函数签名包含 `claims: Claims`。
    *   这会自动触发 `Claims::from_request_parts` 提取器进行 JWT 认证。
    *   如果认证成功，处理器可以安全地使用 `claims` 中的用户信息。
*   **WebSocket 处理 (`ws_handler(WebSocketUpgrade, State<AppState>)`)**:
    *   基本保持不变，但现在通过 `AppState` 可以访问到配置好的数据库连接和应用配置，如果需要在 WebSocket 逻辑中使用的话。

## 6. WebSocket 处理流程 (基本不变)

1.  客户端发起 HTTP GET 请求到 `/ws`，包含升级头。
2.  Axum 路由匹配到 `ws_handler`。
3.  `WebSocketUpgrade` 提取器成功。
4.  `ws_handler` 调用 `ws.on_upgrade(handle_socket)`，发送 101 Switching Protocols 响应。
5.  连接升级为 WebSocket。
6.  Axum 调用 `handle_socket(socket, state)`，传入 `WebSocket` 连接和 `AppState`。
7.  `handle_socket` 处理消息收发。

## 7. 与其他层的关系

- **被路由层 (`routes.rs`) 调用**: 路由层将 HTTP 路径映射到控制器中的 Handler 函数。
- **调用服务层 (`service`)**: 控制器委托服务层执行业务逻辑。服务层现在使用 SeaORM。
- **使用模型层 (`model`)**:
    *   控制器接收 `*Payload` DTOs (通过 `Json` 提取器)。
    *   对于受保护路由，控制器通过 `Claims` 提取器获取 JWT 声明。
    *   控制器将服务层返回的 SeaORM `Model` 对象 (如 `task::Model`) 序列化为 JSON 响应。
- **使用错误处理层 (`error`)**: 控制器函数的返回值通常是 `Result<T, AppError>`。认证失败时，`Claims` 提取器直接返回 `AppError`。