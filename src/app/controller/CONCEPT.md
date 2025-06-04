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

`task_controller.rs` 中大量使用了 Axum 的核心特性：

- **处理函数 (Handler)**:
    - 形式: `async fn handler_name(extractor1: Type1, extractor2: Type2, ...) -> ReturnType`
    - 异步 (`async fn`): 必须是异步函数，以便在等待 I/O (如服务层调用) 时不阻塞线程。
    - 参数: 必须是实现了 `FromRequestParts` 或 `FromRequest` 的提取器类型。
    - 返回值: 必须是实现了 `IntoResponse` 的类型。
- **提取器 (Extractor)**:
    - **`State<AppState>`**: 从应用程序共享状态中提取数据 (需要 `AppState` 实现 `Clone`，并通过 `.with_state()` 注入)。这是实现依赖注入和共享资源（如数据库连接池）的关键。
    - **`Path<T>`**: 从 URL 路径参数中提取数据 (例如 `/tasks/:id` 中的 `id`)。`T` 可以是元组或实现了 `serde::Deserialize` 的类型。
    - **`Json<T>`**: 从请求体中反序列化 JSON 数据。`T` 必须实现 `serde::Deserialize`。如果反序列化失败，Axum 会自动返回 4xx 错误响应。
    - **`WebSocketUpgrade`**: 特殊提取器，用于处理 WebSocket 协议升级请求。它不直接提取数据，而是提供 `.on_upgrade()` 方法来设置连接建立后的回调。
- **响应 (`IntoResponse`)**:
    - Handler 的返回值必须能转换为 `axum::response::Response`。
    - **`(StatusCode, Json<T>)`**: 常用组合，返回指定状态码和 JSON 响应体。`T` 必须实现 `serde::Serialize`。
    - **`StatusCode`**: 单独返回状态码，通常用于无响应体的成功（如 204 No Content）或错误情况。
    - **`Result<T, E>`**: Axum 对 `Result` 有特殊处理。如果 `T` 和 `E` 都实现了 `IntoResponse`，则 `Ok(T)` 会变成 `T` 的响应，`Err(E)` 会变成 `E` 的响应。这对于统一处理成功和错误路径非常方便（本项目中 `E` 是 `AppError`）。
    - **`impl IntoResponse`**: 作为返回类型，允许 Handler 根据内部逻辑返回不同类型的响应（只要它们都实现了 `IntoResponse`）。

## 3. 共享状态 (`AppState`)**

- **目的**: 封装需要在多个 Handler 间共享的数据，最常见的是数据库连接池 (`Db` 类型，即 `Arc<RwLock<...>>`)。
- **实现 `Clone`**: Axum 要求状态必须是 `Clone` 的，以便在分发给 Handler 时克隆。
- **注入**: 在创建 `Router` 时使用 `.with_state(app_state)` 方法将 `AppState` 实例注入。
- **访问**: Handler 通过 `State<AppState>` 提取器访问状态的克隆副本。

## 4. 请求处理生命周期 (简化)**

1.  HTTP 请求到达。
2.  Axum 中间件 (Middleware) 执行 (本项目中是日志中间件，后续会添加)。
3.  Axum 路由层 (`Router` in `routes.rs`) 根据请求路径和方法匹配到对应的 Handler (如 `controller::create_task`)。
4.  Axum 尝试从请求中运行 Handler 参数所需的所有【提取器】 (`State`, `Json`, `Path` 等)。
    - 如果任何提取器失败（如 JSON 解析错误），Axum 直接返回错误响应，Handler **不执行**。
5.  如果所有提取器成功，Axum 调用 Handler 函数。
6.  Handler 函数执行：
    - (可选) 解析/验证提取的数据。
    - 调用服务层函数 (`service::*`)，并 `.await` 其结果。
    - 处理服务层返回的 `Result`。
    - 构造一个实现了 `IntoResponse` 的返回值。
7.  Axum 将 Handler 返回的值转换为 `Response` 对象。
8.  Axum 中间件再次执行 (可以修改响应)。
9.  Axum 将最终的 `Response` 发送回客户端。

## 5. WebSocket 处理流程**

1.  客户端发起 HTTP GET 请求到 `/ws` (或其他配置的路径)，并包含特定的升级头 (e.g., `Upgrade: websocket`, `Connection: Upgrade`)。
2.  Axum 路由匹配到 `ws_handler`。
3.  `WebSocketUpgrade` 提取器成功提取。
4.  `ws_handler` 调用 `ws.on_upgrade(handle_socket)`。这会向客户端发送一个 101 Switching Protocols 响应，表示同意升级。
5.  HTTP 连接升级为 WebSocket 连接。
6.  Axum (在后台任务中) 调用 `handle_socket(socket, state)` 函数，并将建立的 `WebSocket` 连接 (`socket`) 和之前传递的状态 (`state`) 作为参数。
7.  `handle_socket` 函数进入循环，使用 `socket.recv().await` 等待接收消息，使用 `socket.send().await` 发送消息。
8.  当 `recv()` 返回 `None` (连接关闭) 或发生错误，或收到 `Close` 消息时，循环结束，`handle_socket` 函数返回，WebSocket 连接关闭。

## 6. 与其他层的关系**

- **被路由层 (`routes.rs`) 调用**: 路由层将 HTTP 路径映射到控制器中的 Handler 函数。
- **调用服务层 (`service`)**: 控制器委托服务层执行业务逻辑。
- **使用模型层 (`model`)**: 控制器接收 `*Payload` DTOs (通过 `Json` 提取器)，并将 `Task` 等模型对象序列化为响应 (通过 `Json` 响应体)。
- **使用错误处理层 (`error`)**: 控制器函数的返回值通常是 `Result<T, AppError>`，依赖 `AppError` 实现 `IntoResponse` 来自动将错误转换为 HTTP 响应。 