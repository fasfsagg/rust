// /-----------------------------------------------------------------------------\
// |                              【模块功能图示】                             |
// |-----------------------------------------------------------------------------|
// | HTTP 请求 (例如 GET /tasks/:id)                                           |
// |      |                                                                      |
// |      V                                                                      |
// | +-----------------------------+      +------------------------------------+ |
// | | routes.rs                   | ---> | task_controller.rs (本模块)        | |
// | | (路由定义, 映射路径到 Handler) |      | (处理函数 Handlers)                | |
// | +-----------------------------+      |                                    | |
// |                                      | 依赖项 (Dependencies):             | |
// |                                      |  - axum::{extract::{...}, ...}     | |
// |                                      |  - model::{...载荷, Task} (模型)   | |
// |                                      |  - service::{..._svc} (服务函数)    | |
// |                                      |  - db::Db (数据库类型)              | |
// |                                      |  - error::{Result, AppError} (错误) | |
// |                                      |                                    | |
// |                                      | +--------------------------------+ | |
// |                                      | | 处理函数 (例如 get_task_by_id)   | | |
// |                                      | | - 输入: 提取器 (Extractors)      | | |
// |                                      | |   - State<应用状态>              | | |
// |                                      | |   - Path<字符串> (路径参数)      | | |
// |                                      | |   - Json<载荷> (请求体)          | | |
// |                                      | |   - WebSocket升级                | | |
// |                                      | | - 逻辑:                          | | |
// |                                      | |   1. 解析输入 (例如 UUID)      | | |
// |                                      | |   2. 调用服务函数                | | |
// |                                      | |   3. 处理结果 (Ok/Err)         | | |
// |                                      | | - 输出: impl IntoResponse        | | |
// |                                      | |   - (状态码, Json<任务>)         | | |
// |                                      | |   - 状态码                     | | |
// |                                      | |   - AppError (通过 IntoResponse) | | |
// |                                      | |   - WebSocket 响应             | | |
// |                                      | +--------------------------------+ | |
// |                                      |        | 调用服务层 (Call Service) | |
// |                                      |        V                           | |
// |                                      |   服务层 (service/*.rs)          | |
// |                                      +------------------------------------+ |
// |                                              |                              |
// |                                              V                              |
// | HTTP 响应 (例如 200 OK + JSON Body)                                     |
// \-----------------------------------------------------------------------------/
//
// 文件路径: src/app/controller/task_controller.rs
//
// 【模块核心职责】
// 这个模块是应用程序的【控制器层 (Controller Layer)】的一部分，专门负责处理与"任务"资源相关的 HTTP 请求。
// 它是 Web 框架 (Axum) 与应用程序业务逻辑 (Service Layer) 之间的【接口】。
//
// 【主要职责】
// 1. **接收 HTTP 请求**: 由 Axum 路由层 (`routes.rs`) 将匹配的请求分发到这里的处理函数 (Handler)。
// 2. **解析请求数据**: 使用 Axum 提供的【提取器 (Extractors)】（如 `State`, `Path`, `Json`）从请求中提取所需信息。
// 3. **调用服务层**: 将解析后的数据传递给服务层 (`service::task_service`) 的相应函数来执行核心业务逻辑。
// 4. **处理服务层结果**: 获取服务层返回的 `Result`，并根据是 `Ok` 还是 `Err` 来决定如何响应。
// 5. **构造 HTTP 响应**: 将业务逻辑的处理结果转换为标准的 HTTP 响应，通常使用 Axum 的 `IntoResponse` 特性。
// 6. **WebSocket 处理**: 处理 WebSocket 握手请求和后续通信。
//
// 【控制器层的特点 - "轻薄"】: 控制器不应包含复杂业务逻辑，主要负责数据传递和 HTTP 交互。
//
// 【Axum 框架关键概念】: Handler, Extractor (State, Path, Json, WebSocketUpgrade), IntoResponse, AppState。
//
// 【面向初学者提示】: 控制器像餐厅服务员，接收请求、传递给后厨（服务层）、返回结果给顾客。

// --- 导入依赖 ---
// 导入 Axum 框架的核心组件
use axum::{
    extract::{ Path, State, ws::{ WebSocket, Message, WebSocketUpgrade } },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
// 导入 UUID 类型。
use uuid::Uuid;
// 导入模型层定义的载荷结构体。
use crate::app::model::{ CreateTaskPayload, Task, UpdateTaskPayload };
// 导入服务层模块。
use crate::app::service;
// 导入数据库类型定义，用于 AppState。
use crate::db::Db;
// 导入自定义错误类型和 Result 别名。
use crate::error::{ self, AppError, Result }; // 显式导入 AppError

// --- 应用程序共享状态 ---

/// 应用程序状态结构体 (Application State Struct)
///
/// 【用途】: 封装需要在整个应用程序（特别是不同的请求处理函数之间）共享的数据。
/// 【生命周期】: 通常在应用程序启动时创建一次，并通过 Axum 的 `.with_state()` 方法注入到 Router 中。
/// 【共享机制】: Axum 要求 State 必须实现 `Clone` 特性。
///             当请求到达时，Axum 会【克隆】这个状态并将其传递给处理函数。
///             因此，状态内部的字段通常需要使用 `Arc` 来包裹，以避免深拷贝。
/// 【`#[derive(Clone)]`**: 自动实现 `Clone` 特性。[[关键语法要素: derive 宏]]
#[derive(Clone)]
pub struct AppState {
    /// 数据库实例 (Database Instance)
    /// 【类型】: `Db` (即 `Arc<RwLock<HashMap<Uuid, Task>>>`)
    /// 【共享】: `Db` 是 `Arc` 包裹的，克隆 `AppState` 实际上是克隆 `Arc` 指针，非常高效。
    pub db: Db,
}

// --- 任务相关的 HTTP 处理函数 (Task Handlers) ---

/// Handler: 创建任务 (POST /tasks)
///
/// 【功能】: 处理创建新任务的 HTTP POST 请求。
/// 【路由】: 通常在 `routes.rs` 中被绑定到 `POST /tasks` 路径。
/// 【标记】: `pub async fn` - 公共异步处理函数。
///
/// # 【参数 (Axum Extractors)】
/// * `State(state): State<AppState>`: [[Axum Extractor: State]]
///    - 从应用程序状态中提取 `AppState` 的【克隆】。
///    - `State(...)` 语法是解构模式。
/// * `Json(payload): Json<CreateTaskPayload>`: [[Axum Extractor: Json]]
///    - 尝试从 HTTP 请求体中【反序列化】JSON 数据为 `CreateTaskPayload` 类型。
///    - `Json(...)` 解构模式。
///    - 【错误处理】: 无效 JSON 或结构不匹配会自动返回 4xx 错误。
///
/// # 【返回值】
/// * `-> Result<impl IntoResponse>`: [[Axum 返回值: Result]]
///    - 使用自定义 `Result` (`Result<T, AppError>`)。
///    - `impl IntoResponse`: 成功类型 (`T`) 和失败类型 (`AppError`) 都必须实现 `IntoResponse`。
///    - 【成功路径 (`Ok(...)`)】: `Ok((StatusCode::CREATED, Json(task)))`
///      - 返回元组 `(StatusCode, Json<Task>)`，Axum 内置了其 `IntoResponse` 实现。
///      - `StatusCode::CREATED` (201): 设置状态码。
///      - `Json(task)`: 将 `Task` 序列化为 JSON 响应体，并设置 `Content-Type`。
///    - 【失败路径 (`Err(...)`)】: 函数体中的 `?` 会处理错误。
///      - 如果 `service::create_task(...).await` 返回 `Err(app_error)`，`?` 将其作为当前函数的返回值。
///      - Axum 会调用 `AppError` 的 `into_response()` 方法将错误转为 HTTP 响应。
pub async fn create_task(
    State(state): State<AppState>, // 注入共享状态
    Json(payload): Json<CreateTaskPayload> // 从请求体解析 JSON
) -> Result<impl IntoResponse> {
    // 返回 Result，成功和失败类型都能转为 Response
    println!("CONTROLLER: Received create task request");
    // --- 调用服务层 ---
    // `state.db`: 从注入的 AppState 中获取数据库实例的引用。
    // `payload`: 将从 JSON 解析得到的载荷传递给服务层。
    // `.await`: 因为 `service::create_task` 是 `async fn`。
    // `?`: 如果服务层返回 `Err(app_error)`，则立即将 `Err(app_error)` 作为此函数的返回值。
    let task = service::create_task(&state.db, payload).await?;
    println!("CONTROLLER: Task created successfully (ID: {})", task.id);

    // --- 构造成功响应 ---
    // 成功时，返回一个包含状态码和 JSON 响应体的元组。
    Ok((StatusCode::CREATED, Json(task)))
}

/// Handler: 获取所有任务 (GET /tasks)
///
/// 【功能】: 处理获取所有任务列表的 HTTP GET 请求。
/// 【路由】: 绑定到 `GET /tasks`。
///
/// # 【参数】
/// * `State(state): State<AppState>`: 注入共享状态以访问数据库。
///
/// # 【返回值】
/// * `-> impl IntoResponse`: [[Axum 返回值: impl Trait]]
///    - 这里没有使用 `Result`，假设此操作总能成功（简单示例）。
///    - 直接返回一个元组 `(StatusCode, Json<Vec<Task>>)`。
///    - `StatusCode::OK` (200): 标准成功状态码。
///    - `Json(tasks)`: 将 `Vec<Task>` 序列化为 JSON 数组作为响应体。
pub async fn get_all_tasks(State(state): State<AppState>) -> impl IntoResponse {
    println!("CONTROLLER: Received get all tasks request");
    // --- 调用服务层 ---
    // `.await`: 因为 `service::get_all_tasks` 是 `async fn`。
    let tasks = service::get_all_tasks(&state.db).await;
    println!("CONTROLLER: Retrieved {} tasks", tasks.len());

    // --- 构造成功响应 ---
    (StatusCode::OK, Json(tasks))
}

/// Handler: 获取单个任务 (GET /tasks/:id)
///
/// 【功能】: 处理根据 ID 获取单个任务的 HTTP GET 请求。
/// 【路由】: 绑定到 `GET /tasks/:id`，其中 `:id` 是路径参数。
///
/// # 【参数】
/// * `State(state): State<AppState>`: 注入共享状态。
/// * `Path(id_str): Path<String>`: [[Axum Extractor: Path]]
///    - 从 URL 路径中提取 `:id` 部分作为 `String`。
///    - `Path(...)` 解构。
///
/// # 【返回值】
/// * `-> Result<impl IntoResponse>`: 返回 `Result`，因为任务可能找不到。
///    - 【成功路径】: `Ok((StatusCode::OK, Json(task)))` - 返回 200 OK 和任务 JSON。
///    - 【失败路径】: 处理 `InvalidUuid` 和 `TaskNotFound` 错误，Axum 会自动转换。
pub async fn get_task_by_id(
    State(state): State<AppState>,
    Path(id_str): Path<String> // 从路径提取 ID 字符串
) -> Result<impl IntoResponse> {
    println!("CONTROLLER: Received get task by ID request for '{}'", id_str);
    // --- 解析输入 ---
    // 路径参数提取的是字符串，我们需要将其解析为 Uuid 类型。
    // 调用下面的辅助函数 `parse_uuid`。
    // `?`: 如果解析失败，`parse_uuid` 返回 `Err(AppError::InvalidUuid)`，`?` 将其作为此函数的返回值。
    let id = parse_uuid(&id_str)?;
    println!("CONTROLLER: Parsed UUID: {}", id);

    // --- 调用服务层 ---
    let task = service::get_task_by_id(&state.db, id).await?;
    println!("CONTROLLER: Task found for ID: {}", id);

    // --- 构造成功响应 ---
    Ok((StatusCode::OK, Json(task)))
}

/// Handler: 更新任务 (PUT /tasks/:id)
///
/// 【功能】: 处理更新现有任务的 HTTP PUT 请求。
/// 【路由】: 绑定到 `PUT /tasks/:id`。
///
/// # 【参数】
/// * `State(state): State<AppState>`: 注入共享状态。
/// * `Path(id_str): Path<String>`: 提取路径参数 ID。
/// * `Json(payload): Json<UpdateTaskPayload>`: 从请求体解析 JSON 更新数据。
///
/// # 【返回值】
/// * `-> Result<impl IntoResponse>`: 返回 `Result`。
///    - 【成功路径】: `Ok((StatusCode::OK, Json(task)))` - 返回 200 OK 和更新后的任务 JSON。
///    - 【失败路径】: 处理 `InvalidUuid` 和 `TaskNotFound` 等错误。
pub async fn update_task(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    Json(payload): Json<UpdateTaskPayload> // 从请求体解析更新数据
) -> Result<impl IntoResponse> {
    println!("CONTROLLER: Received update task request for '{}'", id_str);
    // --- 解析输入 ---
    let id = parse_uuid(&id_str)?;
    println!("CONTROLLER: Parsed UUID: {}", id);

    // --- 调用服务层 ---
    let task = service::update_task(&state.db, id, payload).await?;
    println!("CONTROLLER: Task updated successfully for ID: {}", id);

    // --- 构造成功响应 ---
    Ok((StatusCode::OK, Json(task)))
}

/// Handler: 删除任务 (DELETE /tasks/:id)
///
/// 【功能】: 处理删除任务的 HTTP DELETE 请求。
/// 【路由】: 绑定到 `DELETE /tasks/:id`。
///
/// # 【参数】
/// * `State(state): State<AppState>`: 注入共享状态。
/// * `Path(id_str): Path<String>`: 提取路径参数 ID。
///
/// # 【返回值】
/// * `-> Result<impl IntoResponse>`: 返回 `Result`。
///    - 【成功路径】: `Ok(StatusCode::NO_CONTENT)` - 返回 204 No Content 状态码。[[HTTP 状态码: 204 No Content]]
///      - 204 响应通常【没有】响应体。
///    - 【失败路径】: 处理 `InvalidUuid` 和 `TaskNotFound` 等错误。
pub async fn delete_task(
    State(state): State<AppState>,
    Path(id_str): Path<String>
) -> Result<impl IntoResponse> {
    println!("CONTROLLER: Received delete task request for '{}'", id_str);
    // --- 解析输入 ---
    let id = parse_uuid(&id_str)?;
    println!("CONTROLLER: Parsed UUID: {}", id);

    // --- 调用服务层 ---
    // `?` 在成功时提取 `Ok(Task)` 中的 `Task`，但我们忽略它。
    service::delete_task(&state.db, id).await?;
    println!("CONTROLLER: Task deleted successfully for ID: {}", id);

    // --- 构造成功响应 ---
    // 直接返回 204 状态码。
    Ok(StatusCode::NO_CONTENT)
}

/// Handler: WebSocket 连接处理 (GET /ws)
///
/// 【功能】: 处理客户端发起的 WebSocket 握手请求。
/// 【路由】: 通常绑定到 `GET /ws` 或类似路径。
///
/// # 【参数】
/// * `ws: WebSocketUpgrade`: [[Axum Extractor: WebSocketUpgrade]]
///    - 用于检测 WebSocket 升级请求，并提供 `.on_upgrade()` 方法。
/// * `State(state): State<AppState>`: 注入共享状态，以便后续的 `handle_socket` 可以访问。
///
/// # 【返回值】
/// * `-> impl IntoResponse`: 返回由 `ws.on_upgrade()` 生成的特殊响应，告知客户端同意升级协议。
pub async fn ws_handler(
    ws: WebSocketUpgrade, // WebSocket 升级请求提取器
    State(state): State<AppState> // 注入共享状态
) -> impl IntoResponse {
    println!("CONTROLLER: Received WebSocket upgrade request");
    // --- 处理 WebSocket 升级 ---
    // `ws.on_upgrade()`: 接收一个回调函数（这里是 `handle_socket`）。
    // 连接成功升级后，Axum 调用此回调，传入建立好的 `WebSocket` 连接。
    // 使用 `move` 将 state 的所有权移入闭包，传递给 `handle_socket`。
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// WebSocket 实际处理逻辑
///
/// 【功能】: 在 WebSocket 连接建立后，处理消息的接收和发送。
/// 【调用】: 由 `ws.on_upgrade()` 在连接成功升级后异步调用。
///
/// # 【参数】
/// * `socket: WebSocket`: [[Axum 类型: WebSocket]]
///    - 代表已建立的双向 WebSocket 连接。提供 `.send()` 和 `.recv()` 方法。
/// * `state: AppState`: 从 `ws_handler` 传递过来的共享状态。
async fn handle_socket(mut socket: WebSocket, state: AppState) {
    println!("WEBSOCKET: Connection established. Sending welcome message.");
    // --- 发送欢迎消息 ---
    if let Err(e) = socket.send(Message::Text("欢迎连接任务管理 WebSocket 服务！".into())).await {
        println!("WEBSOCKET: Failed to send welcome message: {}", e);
        return; // 发送失败则直接返回
    }

    // --- 消息接收与处理循环 ---
    // `socket.recv()`: 异步等待接收下一条消息。
    // 返回 `Option<Result<Message, axum::Error>>`。
    while let Some(msg_result) = socket.recv().await {
        match msg_result {
            Ok(msg) => {
                // --- 根据消息类型处理 ---
                match msg {
                    // 处理文本消息
                    Message::Text(text) => {
                        println!("WEBSOCKET: Received text message: {}", text);
                        // --- 业务逻辑示例：简单回显 ---
                        let response = Message::Text(format!("Echo: {}", text));
                        if socket.send(response).await.is_err() {
                            println!("WEBSOCKET: Failed to send echo message. Closing connection.");
                            break; // 发送失败，退出循环
                        }
                    }
                    // 处理二进制消息
                    Message::Binary(data) => {
                        println!("WEBSOCKET: Received binary message ({} bytes)", data.len());
                        if socket.send(Message::Binary(data)).await.is_err() {
                            println!("WEBSOCKET: Failed to send binary echo. Closing connection.");
                            break;
                        }
                    }
                    // 处理关闭消息
                    Message::Close(close_frame) => {
                        println!("WEBSOCKET: Received close message: {:?}", close_frame);
                        break; // 收到关闭帧，退出循环
                    }
                    // 处理 Ping/Pong (Axum 通常会自动处理 Pong)
                    Message::Ping(ping_data) => {
                        println!("WEBSOCKET: Received ping");
                        if socket.send(Message::Pong(ping_data)).await.is_err() {
                            println!("WEBSOCKET: Failed to send pong. Closing connection.");
                            break;
                        }
                    }
                    Message::Pong(_) => {
                        println!("WEBSOCKET: Received pong");
                    }
                }
            }
            Err(e) => {
                println!("WEBSOCKET: Error receiving message: {}", e);
                break; // 发生错误，退出循环
            }
        }
    }
    // --- 连接关闭 ---
    println!("WEBSOCKET: Connection closed.");
}

/// 辅助函数：将字符串解析为 UUID (Helper Function: Parse UUID)
///
/// 【功能】: 尝试将传入的字符串 slice (`&str`) 解析为 `Uuid` 类型。
/// 【错误处理】: 如果解析失败，返回自定义错误 `AppError::InvalidUuid`。
///
/// # 【参数】
/// * `id_str: &str` - 要解析的字符串的【不可变引用】。
///
/// # 【返回值】
/// * `-> Result<Uuid>`: `Ok(Uuid)` 或 `Err(AppError::InvalidUuid)`。
fn parse_uuid(id_str: &str) -> Result<Uuid> {
    // `Uuid::parse_str`: 返回 `Result<Uuid, uuid::Error>`。
    // `.map_err(|_| ...)`: 如果是 Err，则转换错误类型。
    //   - `_`: 忽略原始的 `uuid::Error`。
    //   - `error::invalid_uuid(id_str)`: 创建自定义错误。
    Uuid::parse_str(id_str).map_err(|_| error::invalid_uuid(id_str))
}
