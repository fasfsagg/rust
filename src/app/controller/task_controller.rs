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
// 导入标准库的 ControlFlow，用于优雅地控制循环。
use std::ops::ControlFlow;
// 导入 UUID 类型。
use uuid::Uuid;
// 导入模型层定义的载荷结构体和 Task DTO。
// 注意：现在我们使用的是 `Task` DTO，而不是数据库实体。
use crate::app::model::task::{ CreateTaskPayload, Task, UpdateTaskPayload };
// 导入服务层模块。
use crate::app::service;
// 导入自定义错误类型和 Result 别名。
use crate::error::{ self, Result }; // 显式导入 AppError
use crate::startup::AppState; // <--- 使用在 startup.rs 中定义的新 AppState

// --- 应用程序共享状态 ---

// 【说明】: AppState 的定义已移至 `src/startup.rs` 作为单一来源。
//          这里直接使用 `use crate::startup::AppState;` 导入。
//          这解决了 E0255 (重复定义) 和 E0308 (类型不匹配) 的错误。

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
    // `state.db_connection`: 从注入的 AppState 中获取数据库连接的引用。
    // `payload`: 将从 JSON 解析得到的载荷传递给服务层。
    // `.await`: 因为 `service::create_task` 是 `async fn`。
    // `?`: 如果服务层返回 `Err(app_error)`，则立即将 `Err(app_error)` 作为此函数的返回值。
    let task = service::create_task(&state.db_connection, payload).await?;
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
/// * `-> Result<impl IntoResponse>`: 返回 Result 以处理潜在的数据库错误。
///    - 【成功路径】: `Ok((StatusCode::OK, Json<Vec<Task>>))`
///    - 【失败路径】: 由 `?` 操作符处理。
pub async fn get_all_tasks(State(state): State<AppState>) -> Result<impl IntoResponse> {
    println!("CONTROLLER: Received get all tasks request");
    // --- 调用服务层 ---
    // `.await`: 因为 `service::get_all_tasks` 是 `async fn`。
    // `?`: 如果服务层返回错误，则提前返回。
    let tasks = service::get_all_tasks(&state.db_connection).await?;
    println!("CONTROLLER: Retrieved {} tasks", tasks.len());

    // --- 构造成功响应 ---
    Ok((StatusCode::OK, Json(tasks)))
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
    let task = service::get_task_by_id(&state.db_connection, id).await?;
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
    let task = service::update_task(&state.db_connection, id, payload).await?;
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
) -> Result<StatusCode> {
    println!("CONTROLLER: Received delete task request for '{}'", id_str);
    let id = parse_uuid(&id_str)?;
    println!("CONTROLLER: Parsed UUID: {}", id);

    // 调用服务层
    service::delete_task(&state.db_connection, id).await?;
    println!("CONTROLLER: Task deleted successfully for ID: {}", id);

    // --- 构造成功响应 ---
    // 对于成功的 DELETE 操作，返回 204 No Content 是最佳实践。
    // 这告诉客户端操作已成功执行，但响应体中没有内容。
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
    // 成功时，返回一个特殊的响应，指示服务器愿意将连接升级到 WebSocket 协议。
    // `ws.on_upgrade(...)` 接收一个回调函数，该函数在 WebSocket 连接建立后执行。
    println!("CONTROLLER: Received WebSocket upgrade request");
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Helper: 处理单个 WebSocket 连接
///
/// 【功能】: 这个函数是 WebSocket 连接建立后的【主循环】。
///
/// # 【参数】
/// * `socket: WebSocket`: 表示一个独立的 WebSocket 连接。
/// * `state: AppState`: 传入应用状态，以便在需要时访问（例如，数据库）。
async fn handle_socket(mut socket: WebSocket, _state: AppState) {
    // 循环等待来自客户端的消息。
    // `socket.recv().await` 是一个异步操作，会等待直到接收到消息。
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            // 打印收到的消息。
            println!("WS: Received message: {:?}", msg);

            // 处理消息并获取响应
            match msg {
                Message::Text(text) => {
                    println!("WS: Received text message: {}", text);
                    // 发送回复消息
                    if
                        let Err(e) = socket.send(
                            Message::Text(format!("服务器收到: {}", text))
                        ).await
                    {
                        println!("WS: Error sending response: {:?}", e);
                        break;
                    }
                }
                Message::Close(c) => {
                    if let Some(cf) = c {
                        println!(
                            "WS: Received close with code {} and reason '{}'",
                            cf.code,
                            cf.reason
                        );
                    } else {
                        println!("WS: Received close message without details");
                    }
                    break;
                }
                _ => {
                    println!("WS: Received other type of message: {:?}", msg);
                }
            }
        } else {
            // 客户端断开连接。
            println!("WS: Client disconnected.");
            break;
        }
    }
    // 注意: `state` 在这里被丢弃，但由于它是 `Arc` 的克隆，所以不会影响其他部分。
}

/// 辅助函数：处理单个 WebSocket 消息
///
/// 【功能】: 根据收到的消息类型执行不同操作。
/// 【返回值】: `ControlFlow` - 用于指示调用者是否应该中断循环。
///    - `ControlFlow::Continue(())`: 继续处理后续消息。
///    - `ControlFlow::Break(())`: 关闭连接并跳出循环。
fn process_message(msg: Message) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(t) => {
            println!("WS: Received text message: {}", t);
        }
        Message::Binary(b) => {
            println!("WS: Received binary message: {:?}", b);
        }
        Message::Ping(p) => {
            println!("WS: Received ping: {:?}", p);
        }
        Message::Pong(p) => {
            println!("WS: Received pong: {:?}", p);
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                println!("WS: Received close with code {} and reason '{}'", cf.code, cf.reason);
            } else {
                println!("WS: Received close message without details");
            }
            return ControlFlow::Break(()); // 收到关闭帧，中断循环
        }
    }
    ControlFlow::Continue(()) // 其他消息，继续循环
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
