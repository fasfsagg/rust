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
// |                                      |  - model::{...载荷} (模型)         | |
// |                                      |  - service::{...} (服务函数)       | |
// |                                      |  - error::{Result, AppError} (错误) | |
// |                                      |                                    | |
// |                                      | +--------------------------------+ | |
// |                                      | | ... Handlers ...               | | |
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
    extract::{ ws::{ Message, WebSocket, WebSocketUpgrade }, Path, State, Extension },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
// 导入标准库的 ControlFlow，用于优雅地控制循环。
use std::ops::ControlFlow;

// 导入模型层定义的载荷结构体。
// 注意：`Task` DTO 已被移除，因为它在控制器层未被直接使用。
use crate::app::model::task::{ CreateTaskPayload, UpdateTaskPayload };
// 导入服务层模块。
use crate::app::service;
// 导入自定义错误类型和 Result 别名。
use crate::error::Result;
use crate::startup::AppState;
// 导入认证中间件的用户信息结构体
use crate::app::middleware::auth_middleware::AuthenticatedUser;
// 导入工具函数
use crate::app::utils::{ parse_uuid_string, parse_user_id };

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
/// 【认证】: 需要有效的JWT令牌，用户信息从请求扩展中提取。
///
/// # 【参数 (Axum Extractors)】
/// * `State(state): State<AppState>`: [[Axum Extractor: State]]
///    - 从应用程序状态中提取 `AppState` 的【克隆】。
///    - `State(...)` 语法是解构模式。
/// * `Extension(user): Extension<AuthenticatedUser>`: [[Axum Extractor: Extension]]
///    - 从请求扩展中提取认证用户信息，由JWT中间件注入。
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
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<CreateTaskPayload>
) -> Result<impl IntoResponse> {
    tracing::info!(
        username = %user.username,
        user_id = %user.user_id,
        "收到创建任务请求"
    );

    // 解析用户ID为UUID
    let user_uuid = parse_user_id(&user.user_id)?;

    let task = service::create_task(state.task_repo.clone(), payload, user_uuid).await?;
    tracing::info!(task_id = %task.id, username = %user.username, "任务创建成功");
    Ok((StatusCode::CREATED, Json(task)))
}

/// Handler: 获取所有任务 (GET /tasks)
///
/// 【功能】: 处理获取所有任务列表的 HTTP GET 请求。
/// 【路由】: 绑定到 `GET /tasks`。
/// 【认证】: 需要有效的JWT令牌，只返回当前用户的任务。
///
/// # 【参数】
/// * `State(state): State<AppState>`: 注入共享状态以访问数据库。
/// * `Extension(user): Extension<AuthenticatedUser>`: 从请求扩展中提取认证用户信息。
///
/// # 【返回值】
/// * `-> Result<impl IntoResponse>`: 返回 Result 以处理潜在的数据库错误。
///    - 【成功路径】: `Ok((StatusCode::OK, Json<Vec<Task>>))`
///    - 【失败路径】: 由 `?` 操作符处理。
pub async fn get_all_tasks(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>
) -> Result<impl IntoResponse> {
    println!(
        "CONTROLLER: Received get all tasks request from user: {} (ID: {})",
        user.username,
        user.user_id
    );

    // 解析用户ID为UUID
    let user_uuid = parse_user_id(&user.user_id)?;

    let tasks = service::get_all_tasks(state.task_repo.clone(), user_uuid).await?;
    println!("CONTROLLER: Retrieved {} tasks for user: {}", tasks.len(), user.username);
    Ok((StatusCode::OK, Json(tasks)))
}

/// Handler: 获取单个任务 (GET /tasks/:id)
///
/// 【功能】: 处理根据 ID 获取单个任务的 HTTP GET 请求。
/// 【路由】: 绑定到 `GET /tasks/:id`，其中 `:id` 是路径参数。
/// 【认证】: 需要有效的JWT令牌，只能获取属于当前用户的任务。
///
/// # 【参数】
/// * `State(state): State<AppState>`: 注入共享状态。
/// * `Extension(user): Extension<AuthenticatedUser>`: 从请求扩展中提取认证用户信息。
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
    Extension(user): Extension<AuthenticatedUser>,
    Path(id_str): Path<String>
) -> Result<impl IntoResponse> {
    println!(
        "CONTROLLER: Received get task by ID request for '{}' from user: {} (ID: {})",
        id_str,
        user.username,
        user.user_id
    );
    let id = parse_uuid_string(&id_str)?;
    tracing::debug!(task_id = %id, "任务ID解析成功");

    // 解析用户ID为UUID
    let user_uuid = parse_user_id(&user.user_id)?;

    let task = service::get_task_by_id(state.task_repo.clone(), id, user_uuid).await?;
    println!("CONTROLLER: Task found for ID: {} for user: {}", id, user.username);
    Ok((StatusCode::OK, Json(task)))
}

/// Handler: 更新任务 (PUT /tasks/:id)
///
/// 【功能】: 处理更新现有任务的 HTTP PUT 请求。
/// 【路由】: 绑定到 `PUT /tasks/:id`。
/// 【认证】: 需要有效的JWT令牌，只能更新属于当前用户的任务。
///
/// # 【参数】
/// * `State(state): State<AppState>`: 注入共享状态。
/// * `Extension(user): Extension<AuthenticatedUser>`: 从请求扩展中提取认证用户信息。
/// * `Path(id_str): Path<String>`: 提取路径参数 ID。
/// * `Json(payload): Json<UpdateTaskPayload>`: 从请求体解析 JSON 更新数据。
///
/// # 【返回值】
/// * `-> Result<impl IntoResponse>`: 返回 `Result`。
///    - 【成功路径】: `Ok((StatusCode::OK, Json(task)))` - 返回 200 OK 和更新后的任务 JSON。
///    - 【失败路径】: 处理 `InvalidUuid` 和 `TaskNotFound` 等错误。
pub async fn update_task(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id_str): Path<String>,
    Json(payload): Json<UpdateTaskPayload>
) -> Result<impl IntoResponse> {
    println!(
        "CONTROLLER: Received update task request for '{}' from user: {} (ID: {})",
        id_str,
        user.username,
        user.user_id
    );
    let id = parse_uuid_string(&id_str)?;
    tracing::debug!(task_id = %id, "任务ID解析成功");

    // 解析用户ID为UUID
    let user_uuid = parse_user_id(&user.user_id)?;

    let task = service::update_task(state.task_repo.clone(), id, payload, user_uuid).await?;
    println!("CONTROLLER: Task updated successfully for ID: {} for user: {}", id, user.username);
    Ok((StatusCode::OK, Json(task)))
}

/// Handler: 删除任务 (DELETE /tasks/:id)
///
/// 【功能】: 处理删除任务的 HTTP DELETE 请求。
/// 【路由】: 绑定到 `DELETE /tasks/:id`。
/// 【认证】: 需要有效的JWT令牌，只能删除属于当前用户的任务。
///
/// # 【参数】
/// * `State(state): State<AppState>`: 注入共享状态。
/// * `Extension(user): Extension<AuthenticatedUser>`: 从请求扩展中提取认证用户信息。
/// * `Path(id_str): Path<String>`: 提取路径参数 ID。
///
/// # 【返回值】
/// * `-> Result<impl IntoResponse>`: 返回 `Result`。
///    - 【成功路径】: `Ok(StatusCode::NO_CONTENT)` - 返回 204 No Content 状态码。[[HTTP 状态码: 204 No Content]]
///      - 204 响应通常【没有】响应体。
///    - 【失败路径】: 处理 `InvalidUuid` 和 `TaskNotFound` 等错误。
pub async fn delete_task(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id_str): Path<String>
) -> Result<StatusCode> {
    println!(
        "CONTROLLER: Received delete task request for '{}' from user: {} (ID: {})",
        id_str,
        user.username,
        user.user_id
    );
    let id = parse_uuid_string(&id_str)?;
    tracing::debug!(task_id = %id, "任务ID解析成功");

    // 解析用户ID为UUID
    let user_uuid = parse_user_id(&user.user_id)?;

    service::delete_task(state.task_repo.clone(), id, user_uuid).await?;
    println!("CONTROLLER: Task deleted successfully for ID: {} for user: {}", id, user.username);
    Ok(StatusCode::NO_CONTENT)
}

/// Handler: WebSocket 处理器 (GET /ws)
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
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    println!("CONTROLLER: Received WebSocket upgrade request");
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// 处理单个 WebSocket 连接
async fn handle_socket(mut socket: WebSocket, _state: AppState) {
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            if process_message(msg) == ControlFlow::Break(()) {
                break;
            }
        } else {
            println!("WS: Client disconnected.");
            break;
        }
    }
}

/// 辅助函数：处理单个 WebSocket 消息
#[allow(dead_code)]
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
            return ControlFlow::Break(());
        }
    }
    ControlFlow::Continue(())
}
