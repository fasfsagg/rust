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
    Json,
    extract::{
        Extension, Path, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode, Uri},
    response::IntoResponse,
};
// 导入标准库的 ControlFlow，用于优雅地控制循环。
use std::ops::ControlFlow;
// 导入 futures_util 用于 WebSocket 流分割
use futures_util::{SinkExt, StreamExt};
// 导入 tokio 同步原语
use tokio::sync::mpsc;
// 导入 UUID 生成
use uuid::Uuid;

// 导入模型层定义的载荷结构体。
// 注意：`Task` DTO 已被移除，因为它在控制器层未被直接使用。
use crate::app::model::task::{CreateTaskPayload, UpdateTaskPayload};
// 导入服务层模块。
use crate::app::service;

// 导入自定义错误类型和 Result 别名。
use crate::error::{InstrumentResult, Result};
use crate::startup::AppState;
// 导入认证中间件的用户信息结构体
use crate::app::middleware::auth_middleware::AuthenticatedUser;
// 导入工具函数
use crate::app::utils::{parse_user_id, parse_uuid_string};

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
    Json(payload): Json<CreateTaskPayload>,
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
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<impl IntoResponse> {
    println!(
        "CONTROLLER: Received get all tasks request from user: {} (ID: {})",
        user.username, user.user_id
    );

    // 解析用户ID为UUID
    let user_uuid = parse_user_id(&user.user_id)?;

    let tasks = service::get_all_tasks(state.task_repo.clone(), user_uuid).await?;
    println!(
        "CONTROLLER: Retrieved {} tasks for user: {}",
        tasks.len(),
        user.username
    );
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
    Path(id_str): Path<String>,
) -> Result<impl IntoResponse> {
    println!(
        "CONTROLLER: Received get task by ID request for '{}' from user: {} (ID: {})",
        id_str, user.username, user.user_id
    );
    let id = parse_uuid_string(&id_str)?;
    tracing::debug!(task_id = %id, "任务ID解析成功");

    // 解析用户ID为UUID
    let user_uuid = parse_user_id(&user.user_id)?;

    let task = service::get_task_by_id(state.task_repo.clone(), id, user_uuid).await?;
    println!(
        "CONTROLLER: Task found for ID: {} for user: {}",
        id, user.username
    );
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
    Json(payload): Json<UpdateTaskPayload>,
) -> Result<impl IntoResponse> {
    println!(
        "CONTROLLER: Received update task request for '{}' from user: {} (ID: {})",
        id_str, user.username, user.user_id
    );
    let id = parse_uuid_string(&id_str)?;
    tracing::debug!(task_id = %id, "任务ID解析成功");

    // 解析用户ID为UUID
    let user_uuid = parse_user_id(&user.user_id)?;

    let task = service::update_task(state.task_repo.clone(), id, payload, user_uuid).await?;
    println!(
        "CONTROLLER: Task updated successfully for ID: {} for user: {}",
        id, user.username
    );
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
    Path(id_str): Path<String>,
) -> Result<StatusCode> {
    println!(
        "CONTROLLER: Received delete task request for '{}' from user: {} (ID: {})",
        id_str, user.username, user.user_id
    );
    let id = parse_uuid_string(&id_str)?;
    tracing::debug!(task_id = %id, "任务ID解析成功");

    // 解析用户ID为UUID
    let user_uuid = parse_user_id(&user.user_id)?;

    service::delete_task(state.task_repo.clone(), id, user_uuid).await?;
    println!(
        "CONTROLLER: Task deleted successfully for ID: {} for user: {}",
        id, user.username
    );
    Ok(StatusCode::NO_CONTENT)
}

/// Handler: 获取在线用户列表 (GET /online-users)
///
/// 【功能】: 处理获取当前在线用户列表的 HTTP GET 请求。
/// 【路由】: 绑定到 `GET /online-users`。
/// 【认证】: 需要有效的JWT令牌。
/// 【任务7实现】: 实现在线用户列表管理功能的核心API端点。
///
/// # 【参数】
/// * `State(state): State<AppState>`: 注入共享状态以访问连接管理器。
/// * `Extension(user): Extension<AuthenticatedUser>`: 从请求扩展中提取认证用户信息。
///
/// # 【返回值】
/// * `-> Result<impl IntoResponse>`: 返回 `Result`。
///    - 【成功路径】: `Ok((StatusCode::OK, Json<OnlineUsersResponse>))` - 返回 200 OK 和在线用户列表 JSON。
///    - 【失败路径】: 处理潜在的连接管理器错误。
///
/// # 【响应格式】
/// ```json
/// {
///   "online_users": [
///     {
///       "user_id": "uuid",
///       "username": "string",
///       "connected_at": "timestamp",
///       "connection_count": number
///     }
///   ],
///   "total_users": number,
///   "total_connections": number,
///   "timestamp": "timestamp"
/// }
/// ```
pub async fn get_online_users(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<impl IntoResponse> {
    println!(
        "CONTROLLER: Received get online users request from user: {} (ID: {})",
        user.username, user.user_id
    );

    // 从连接管理器获取在线用户列表
    let online_users = state.connection_manager.get_online_users().await;
    let total_connections = state.connection_manager.get_connection_count().await;

    // 将OnlineUser转换为UserInfo
    let users: Vec<crate::app::model::UserInfo> = online_users
        .into_iter()
        .map(|online_user| online_user.into())
        .collect();

    // 构造响应数据
    let response = crate::app::model::OnlineUsersResponse {
        users,
        total_count: total_connections,
        timestamp: chrono::Utc::now(),
    };

    println!(
        "CONTROLLER: Retrieved {} users with {} total connections for user: {}",
        response.users.len(),
        response.total_count,
        user.username
    );

    Ok((StatusCode::OK, Json(response)))
}

/// Handler: WebSocket 处理器 (GET /ws) - 带身份验证
///
/// 【功能】: 处理客户端发起的 WebSocket 握手请求，包含 JWT 身份验证。
/// 【路由】: 通常绑定到 `GET /ws` 或类似路径。
/// 【安全】: 要求客户端提供有效的 JWT token（通过查询参数或协议头）。
///
/// # 【参数】
/// * `ws: WebSocketUpgrade`: [[Axum Extractor: WebSocketUpgrade]]
///    - 用于检测 WebSocket 升级请求，并提供 `.on_upgrade()` 方法。
/// * `State(state): State<AppState>`: 注入共享状态，以便后续的 `handle_socket` 可以访问。
/// * `headers: HeaderMap`: 请求头，用于提取 JWT token
/// * `uri: Uri`: 请求 URI，用于从查询参数提取 JWT token
///
/// # 【返回值】
/// * `-> Result<impl IntoResponse, StatusCode>`:
///   - 成功时返回 WebSocket 升级响应
///   - 失败时返回 401 Unauthorized
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    headers: HeaderMap,
    uri: Uri,
) -> impl IntoResponse {
    use crate::app::utils::AuthService;

    println!("CONTROLLER: Received WebSocket upgrade request");

    // 使用统一的 AuthService 进行 WebSocket 认证
    let auth_service = AuthService::new(state.jwt_secret.clone());
    match auth_service.authenticate_websocket_request(&uri, &headers) {
        Ok(claims) => {
            println!(
                "CONTROLLER: WebSocket 连接已授权：用户 {} (ID: {})",
                claims.username, claims.sub
            );
            // 升级到 WebSocket 连接，传递用户信息
            ws.on_upgrade(move |socket| handle_socket(socket, state, claims))
                .into_response()
        }
        Err(err) => {
            println!("CONTROLLER: WebSocket 连接被拒绝：JWT 验证失败 {:?}", err);
            StatusCode::UNAUTHORIZED.into_response()
        }
    }
}

/// 处理单个 WebSocket 连接（已认证）- 使用连接管理器
async fn handle_socket(socket: WebSocket, state: AppState, claims: crate::app::utils::Claims) {
    println!(
        "WS: 已认证用户 {} (ID: {}) 建立 WebSocket 连接",
        claims.username, claims.sub
    );

    // 生成唯一的连接ID
    let connection_id = Uuid::new_v4();

    // 分割 WebSocket 为发送器和接收器
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // 创建消息通道用于向此连接发送消息
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // 解析用户ID为UUID
    let user_uuid = match parse_user_id(&claims.sub) {
        Ok(uuid) => uuid,
        Err(e) => {
            println!("WS: 解析用户ID失败: {:?}", e);
            return;
        }
    };

    // 将连接添加到连接管理器
    if let Err(e) = state
        .connection_manager
        .add_connection(
            connection_id,
            user_uuid,
            claims.username.clone(),
            tx,
            None, // IP地址暂时为空，可以从请求头中提取
        )
        .await
    {
        println!("WS: 添加连接到管理器失败: {}", e);
        return;
    }

    // 使用消息分发器发送欢迎消息（结构化消息）
    let welcome_content = format!("欢迎, {}! 您已成功连接到聊天大厅。", claims.username);
    let welcome_msg = crate::app::model::ServerMessage::new_system(welcome_content);
    if let Err(e) = state
        .message_distributor
        .send_direct_message(
            welcome_msg,
            user_uuid,
            Some(crate::app::service::MessagePriority::High),
        )
        .await
    {
        println!("WS: 发送欢迎消息失败: {}", e);
    }

    // 使用通知服务发送用户加入通知
    let user_info = crate::app::model::UserInfo {
        user_id: user_uuid,
        username: claims.username.clone(),
        connected_at: Some(chrono::Utc::now()),
    };
    let join_event =
        crate::app::service::notification_service::NotificationEvent::new_user_joined(user_info);
    if let Err(e) = state
        .notification_service
        .handle_notification_event(join_event)
        .await
    {
        println!("WS: 处理用户加入通知事件失败: {}", e);
    }

    // 【任务10实现】使用状态同步服务更新用户在线状态
    if let Err(e) = state
        .status_sync_service
        .update_user_status(
            user_uuid,
            claims.username.clone(),
            crate::app::service::UserOnlineStatus::Online,
        )
        .await
    {
        println!("WS: 更新用户在线状态失败: {}", e);
    }

    // 启动发送任务：从通道接收消息并发送到 WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if ws_sender.send(message).await.is_err() {
                break;
            }
        }
    });

    // 启动接收任务：从 WebSocket 接收消息并处理
    let connection_manager = state.connection_manager.clone();
    let user_claims = claims.clone();
    let state_for_recv = state.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(msg_result) = ws_receiver.next().await {
            match msg_result {
                Ok(msg) => {
                    // 更新最后活跃时间
                    if let Err(e) = connection_manager
                        .update_last_activity(&connection_id)
                        .await
                    {
                        println!("WS: 更新活跃时间失败: {}", e);
                    }

                    // 处理消息
                    if let ControlFlow::Break(()) = process_message_with_broadcast(
                        msg,
                        &user_claims,
                        &state_for_recv,
                        &connection_id,
                    )
                    .await
                    {
                        break;
                    }
                }
                Err(_) => {
                    println!("WS: 用户 {} 连接出错", user_claims.username);
                    break;
                }
            }
        }
    });

    // 等待任一任务完成（连接断开）
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    // 清理：从连接管理器中移除连接
    if let Some(removed_connection) = state
        .connection_manager
        .remove_connection(&connection_id)
        .await
    {
        println!("WS: 用户 {} 连接已清理", removed_connection.username);

        // 使用通知服务发送用户离开通知
        let user_info = crate::app::model::UserInfo {
            user_id: removed_connection.user_id,
            username: removed_connection.username.clone(),
            connected_at: Some(removed_connection.connected_at),
        };
        let leave_event =
            crate::app::service::notification_service::NotificationEvent::new_user_left(user_info);
        if let Err(e) = state
            .notification_service
            .handle_notification_event(leave_event)
            .await
        {
            println!("WS: 处理用户离开通知事件失败: {}", e);
        }

        // 【任务10实现】检查用户是否还有其他连接，如果没有则更新为离线状态
        let remaining_connections = state
            .connection_manager
            .get_online_users()
            .await
            .iter()
            .find(|u| u.user_id == removed_connection.user_id)
            .map(|u| u.connection_count)
            .unwrap_or(0);

        if remaining_connections == 0 {
            // 用户完全离线，更新状态
            if let Err(e) = state
                .status_sync_service
                .update_user_status(
                    removed_connection.user_id,
                    removed_connection.username.clone(),
                    crate::app::service::UserOnlineStatus::Offline,
                )
                .await
            {
                println!("WS: 更新用户离线状态失败: {}", e);
            }

            // 清理用户状态信息
            if let Err(e) = state
                .status_sync_service
                .cleanup_user_status(&removed_connection.user_id)
                .await
            {
                println!("WS: 清理用户状态失败: {}", e);
            }
        }
    }
}

/// 辅助函数：处理单个 WebSocket 消息（已认证）- 带消息分发器支持
async fn process_message_with_broadcast(
    msg: Message,
    claims: &crate::app::utils::Claims,
    state: &AppState,
    connection_id: &Uuid,
) -> ControlFlow<(), ()> {
    use crate::app::model::{ChatMessage, ErrorResponse, ServerMessage, UserInfo};

    match msg {
        Message::Text(text) => {
            println!("WS: 用户 {} 发送文本消息: {}", claims.username, text);

            // 尝试解析为结构化的聊天消息
            match serde_json::from_str::<ChatMessage>(&text) {
                Ok(chat_msg) => {
                    // 处理结构化消息
                    if let Err(e) =
                        handle_structured_message(chat_msg, claims, state, connection_id).await
                    {
                        println!("WS: 处理结构化消息失败: {}", e);

                        // 发送错误响应给客户端
                        let error_response = ErrorResponse {
                            error_code: "MESSAGE_PROCESSING_ERROR".to_string(),
                            error_message: "消息处理失败".to_string(),
                            details: Some(e.clone()),
                            timestamp: chrono::Utc::now(),
                        };

                        let error_msg = ServerMessage::new_error(error_response);
                        // 使用消息分发器发送错误消息
                        let _ = state
                            .message_distributor
                            .send_direct_message(
                                error_msg,
                                parse_user_id(&claims.sub).unwrap_or_else(|_| Uuid::new_v4()),
                                Some(crate::app::service::MessagePriority::High),
                            )
                            .await;
                    }
                }
                Err(_) => {
                    // 如果不是JSON格式，当作普通文本消息处理
                    let user_info = UserInfo {
                        user_id: parse_user_id(&claims.sub).unwrap_or_else(|_| Uuid::new_v4()),
                        username: claims.username.clone(),
                        connected_at: Some(chrono::Utc::now()),
                    };

                    let server_msg = ServerMessage::new_text(text.to_string(), user_info);

                    // 使用消息分发器广播消息给所有其他用户（排除发送者）
                    if let Err(e) = state
                        .message_distributor
                        .broadcast_to_all(
                            server_msg,
                            true, // 排除发送者
                            Some(*connection_id),
                            Some(crate::app::service::MessagePriority::Normal),
                        )
                        .await
                    {
                        println!("WS: 广播消息失败: {}", e);
                    }
                }
            }
        }
        Message::Binary(b) => {
            println!("WS: 用户 {} 发送二进制消息: {:?}", claims.username, b);
            // 二进制消息暂不广播
        }
        Message::Ping(p) => {
            println!("WS: 用户 {} 发送 ping: {:?}", claims.username, p);
            // Ping 消息不需要广播
        }
        Message::Pong(p) => {
            println!("WS: 用户 {} 发送 pong: {:?}", claims.username, p);
            // Pong 消息不需要广播
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    "WS: 用户 {} 关闭连接，代码: {} 原因: '{}'",
                    claims.username, cf.code, cf.reason
                );
            } else {
                println!("WS: 用户 {} 关闭连接", claims.username);
            }
            return ControlFlow::Break(());
        }
    }
    ControlFlow::Continue(())
}

/// 辅助函数：处理单个 WebSocket 消息（已认证）- 旧版本，保留用于兼容性
#[allow(dead_code)]
fn process_message(msg: Message, claims: &crate::app::utils::Claims) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(t) => {
            println!("WS: 用户 {} 发送文本消息: {}", claims.username, t);
        }
        Message::Binary(b) => {
            println!("WS: 用户 {} 发送二进制消息: {:?}", claims.username, b);
        }
        Message::Ping(p) => {
            println!("WS: 用户 {} 发送 ping: {:?}", claims.username, p);
        }
        Message::Pong(p) => {
            println!("WS: 用户 {} 发送 pong: {:?}", claims.username, p);
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    "WS: 用户 {} 关闭连接，代码: {} 原因: '{}'",
                    claims.username, cf.code, cf.reason
                );
            } else {
                println!("WS: 用户 {} 关闭连接", claims.username);
            }
            return ControlFlow::Break(());
        }
    }
    ControlFlow::Continue(())
}

/// 处理结构化聊天消息
///
/// 【功能】: 根据消息类型处理不同的聊天消息
/// 【参数】:
/// * `chat_msg` - 解析后的聊天消息
/// * `claims` - 用户认证信息
/// * `state` - 应用状态（包含连接管理器和消息分发器）
/// * `connection_id` - 当前连接ID
///
/// 【返回值】: Result<(), String> - 成功返回 Ok(())，失败返回错误信息
async fn handle_structured_message(
    chat_msg: crate::app::model::ChatMessage,
    claims: &crate::app::utils::Claims,
    state: &AppState,
    connection_id: &Uuid,
) -> std::result::Result<(), String> {
    use crate::app::model::{MessageType, OnlineUsersResponse, ServerMessage, UserInfo};

    let user_info = UserInfo {
        user_id: parse_user_id(&claims.sub).map_err(|e| format!("解析用户ID失败: {:?}", e))?,
        username: claims.username.clone(),
        connected_at: Some(chrono::Utc::now()),
    };

    match chat_msg.message_type {
        MessageType::Text => {
            // 处理文本消息
            let server_msg = ServerMessage::new_text(chat_msg.content, user_info);

            // 使用消息分发器广播消息给所有其他用户（排除发送者）
            state
                .message_distributor
                .broadcast_to_all(
                    server_msg,
                    true, // 排除发送者
                    Some(*connection_id),
                    Some(crate::app::service::MessagePriority::Normal),
                )
                .await
                .map_err(|e| format!("广播文本消息失败: {}", e))?;
        }
        MessageType::GetOnlineUsers => {
            // 处理获取在线用户列表请求
            let online_users = state.connection_manager.get_online_users().await;
            let users_info: Vec<UserInfo> = online_users.into_iter().map(|u| u.into()).collect();

            let response = OnlineUsersResponse {
                users: users_info,
                total_count: state.connection_manager.get_unique_user_count().await,
                timestamp: chrono::Utc::now(),
            };

            let server_msg = ServerMessage::new_online_users_list(response);

            // 使用消息分发器只发送给请求者
            state
                .message_distributor
                .send_direct_message(
                    server_msg,
                    user_info.user_id,
                    Some(crate::app::service::MessagePriority::Normal),
                )
                .await
                .map_err(|e| format!("发送在线用户列表失败: {}", e))?;
        }
        MessageType::Ping => {
            // 处理心跳消息
            let pong_msg = ServerMessage::new_pong();

            // 使用消息分发器只发送给发送者
            state
                .message_distributor
                .send_direct_message(
                    pong_msg,
                    user_info.user_id,
                    Some(crate::app::service::MessagePriority::High), // 心跳响应优先级高
                )
                .await
                .map_err(|e| format!("发送心跳响应失败: {}", e))?;
        }
        MessageType::MessageRead => {
            // 【任务10实现】处理消息已读通知
            let message_id =
                Uuid::parse_str(&chat_msg.content).map_err(|e| format!("解析消息ID失败: {}", e))?;

            // 使用状态同步服务更新消息状态
            if let Err(e) = state
                .status_sync_service
                .update_message_status(
                    message_id,
                    user_info.user_id,
                    crate::app::service::MessageReadStatus::Read,
                )
                .await
            {
                println!("WS: 更新消息已读状态失败: {}", e);
                return Err(format!("更新消息已读状态失败: {}", e));
            }

            println!(
                "WS: 用户 {} 标记消息 {} 为已读",
                user_info.username, message_id
            );
        }
        _ => {
            // 其他消息类型暂不支持
            return Err(format!("不支持的消息类型: {:?}", chat_msg.message_type));
        }
    }

    Ok(())
}

/// 示例控制器：展示增强的错误跟踪功能
///
/// 【功能】：演示如何在控制器层使用 tracing-error 进行错误上下文跟踪
/// 【路由】：GET /tasks/:id/enhanced - 获取任务（带增强错误跟踪）
///
/// # 参数
/// * `State(state)` - 应用程序状态
/// * `Extension(user)` - 认证用户信息
/// * `Path(id_str)` - 任务ID字符串
///
/// # 返回值
/// * `Result<impl IntoResponse>` - 包含详细错误跟踪信息的响应
#[tracing::instrument(skip(state), fields(user_id = %user.user_id, task_id = %id_str))]
pub async fn get_task_with_enhanced_tracking(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id_str): Path<String>,
) -> Result<impl IntoResponse> {
    tracing::info!(
        user_id = %user.user_id,
        username = %user.username,
        task_id = %id_str,
        "开始处理增强错误跟踪的任务获取请求"
    );

    // 使用 InstrumentResult trait 自动捕获 span 上下文
    let task_id = Uuid::parse_str(&id_str).in_current_span(StatusCode::BAD_REQUEST)?;
    let user_uuid = Uuid::parse_str(&user.user_id).in_current_span(StatusCode::BAD_REQUEST)?;

    // 调用增强的服务函数
    let task = service::task_service::get_task_with_enhanced_error_tracking(
        state.task_repo.clone(),
        task_id,
        user_uuid,
    )
    .await?;

    tracing::info!(
        user_id = %user.user_id,
        task_id = %task_id,
        task_title = %task.title,
        "成功获取任务（增强错误跟踪）"
    );

    Ok((StatusCode::OK, Json(task)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::middleware::auth_middleware::AuthenticatedUser;
    use crate::app::repository::task_repository::TaskRepository;
    use crate::app::service::connection_manager::ConnectionManager;
    use crate::app::service::message_distributor::MessageDistributor;
    use crate::app::service::notification_service::NotificationService;
    use crate::startup::AppState;
    use axum::{Extension, extract::State};
    use chrono::Utc;
    use std::sync::Arc;
    use tokio::sync::mpsc;
    use uuid::Uuid;

    /// 创建测试用的AppState
    /// 注意：这个函数创建一个简化的测试状态，用于单元测试
    async fn create_test_app_state() -> AppState {
        let connection_manager = Arc::new(ConnectionManager::new());
        let message_distributor = Arc::new(MessageDistributor::new(
            connection_manager.clone(),
            Some(10), // 小批量用于测试
            Some(1),  // 单线程用于测试
        ));
        let notification_service = Arc::new(NotificationService::new(
            connection_manager.clone(),
            message_distributor.clone(),
        ));
        let status_sync_service = Arc::new(crate::app::service::StatusSyncService::new(
            connection_manager.clone(),
            message_distributor.clone(),
        ));

        // 创建一个内存数据库连接（用于测试）
        let db_connection = sea_orm::Database::connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        // 创建性能监控配置
        let performance_config = crate::app::middleware::PerformanceConfig {
            enable_detailed_logging: false,
            enable_system_monitoring: false,
            enable_prometheus_metrics: false,
            ..Default::default()
        };
        let performance_metrics =
            crate::app::middleware::create_performance_monitoring_layer(performance_config);

        // 创建错误恢复状态
        let error_recovery_manager = crate::app::utils::ErrorRecoveryManager::with_default_config();
        let error_recovery_state =
            crate::app::middleware::error_recovery_middleware::ErrorRecoveryState::new(
                error_recovery_manager,
            );

        // 【任务13.2新增】创建异步性能优化器（测试配置）
        let async_perf_config = crate::app::service::AsyncPerformanceConfig {
            worker_threads: Some(1),            // 测试时使用单线程
            performance_monitoring_interval: 1, // 测试时使用短间隔
            max_concurrent_tasks: 10,           // 测试时使用小限制
            ..Default::default()
        };
        let async_performance_optimizer = Arc::new(
            crate::app::service::AsyncPerformanceOptimizer::new(async_perf_config),
        );

        // 【任务13.3新增】创建测试用内存管理器
        let memory_config = crate::app::utils::memory_manager::MemoryManagerConfig {
            l1_cache_max_entries: 50,    // 测试时使用小缓存
            l1_cache_ttl_seconds: 30,    // 30秒TTL
            object_pool_initial_size: 5, // 小对象池
            object_pool_max_size: 20,
            memory_pool_block_size: 512, // 512字节块
            memory_pool_max_blocks: 10,
            cache_eviction_interval_seconds: 15,
            memory_monitoring_interval_seconds: 5, // 测试时使用短间隔
            memory_pressure_threshold_bytes: 512 * 1024, // 512KB阈值
            enable_leak_detection: false,          // 测试时关闭泄漏检测
        };
        let memory_manager = Arc::new(crate::app::utils::memory_manager::MemoryManager::new(
            memory_config,
        ));

        // 【任务13.4新增】创建测试环境的连接池管理器
        let test_config = crate::config::AppConfig {
            http_addr: "127.0.0.1:3000".parse().unwrap(),
            database_url: "sqlite::memory:".to_string(),
            jwt_secret: "test_secret".to_string(),
            database_pool: crate::config::DatabasePoolConfig::development(),
            websocket_pool: crate::config::WebSocketPoolConfig::development(),
        };

        // 创建数据库连接池管理器（测试环境）
        let database_pool_manager = Arc::new(
            crate::app::utils::DatabasePoolManager::new(&test_config)
                .await
                .expect("Failed to create test database pool manager"),
        );

        // 创建WebSocket连接池管理器（测试环境）
        let websocket_pool_manager = Arc::new(crate::app::utils::WebSocketPoolManager::new(
            test_config.websocket_pool,
        ));

        let db_arc = Arc::new(db_connection);
        AppState {
            task_repo: Arc::new(TaskRepository::from_arc(db_arc.clone())),
            db: db_arc,
            jwt_secret: "test_secret".to_string(),
            connection_manager,
            message_distributor,
            notification_service,
            status_sync_service,
            performance_metrics,
            error_recovery_state,
            async_performance_optimizer,
            memory_manager,
            database_pool_manager,  // 【任务13.4新增】
            websocket_pool_manager, // 【任务13.4新增】
        }
    }

    /// 创建测试用的认证用户
    fn create_test_user() -> AuthenticatedUser {
        AuthenticatedUser {
            user_id: Uuid::new_v4().to_string(),
            username: "test_user".to_string(),
        }
    }

    #[tokio::test]
    async fn test_get_online_users_empty_list() {
        // 测试空的在线用户列表
        let app_state = create_test_app_state().await;
        let user = create_test_user();

        let result = get_online_users(State(app_state), Extension(user)).await;

        assert!(result.is_ok());
        // 注意：由于返回类型是 impl IntoResponse，我们无法直接访问字段
        // 这里只验证函数调用成功
    }

    #[tokio::test]
    async fn test_get_online_users_with_connections() {
        // 测试有连接的在线用户列表
        let app_state = create_test_app_state().await;
        let user = create_test_user();

        // 添加一些测试连接
        let (sender1, _receiver1) = mpsc::unbounded_channel();
        let (sender2, _receiver2) = mpsc::unbounded_channel();

        let user_id1 = Uuid::new_v4();
        let user_id2 = Uuid::new_v4();

        app_state
            .connection_manager
            .add_connection(
                Uuid::new_v4(),
                user_id1,
                "user1".to_string(),
                sender1,
                Some("127.0.0.1".to_string()),
            )
            .await
            .unwrap();

        app_state
            .connection_manager
            .add_connection(
                Uuid::new_v4(),
                user_id2,
                "user2".to_string(),
                sender2,
                Some("127.0.0.2".to_string()),
            )
            .await
            .unwrap();

        let result = get_online_users(State(app_state.clone()), Extension(user)).await;

        assert!(result.is_ok());

        // 验证连接数
        assert_eq!(app_state.connection_manager.get_connection_count().await, 2);
        assert_eq!(
            app_state.connection_manager.get_unique_user_count().await,
            2
        );
    }

    #[tokio::test]
    async fn test_handle_structured_message_get_online_users() {
        // 测试WebSocket结构化消息处理 - 获取在线用户列表
        let app_state = create_test_app_state().await;
        let now = Utc::now();
        let claims = crate::app::utils::Claims {
            sub: Uuid::new_v4().to_string(),
            username: "test_user".to_string(),
            exp: (now + chrono::Duration::hours(1)).timestamp(),
            iat: now.timestamp(),
        };
        let connection_id = Uuid::new_v4();

        // 创建获取在线用户的消息
        let chat_msg = crate::app::model::ChatMessage::new_get_online_users();

        let result = handle_structured_message(chat_msg, &claims, &app_state, &connection_id).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_structured_message_ping() {
        // 测试WebSocket结构化消息处理 - 心跳消息
        let app_state = create_test_app_state().await;
        let now = Utc::now();
        let claims = crate::app::utils::Claims {
            sub: Uuid::new_v4().to_string(),
            username: "test_user".to_string(),
            exp: (now + chrono::Duration::hours(1)).timestamp(),
            iat: now.timestamp(),
        };
        let connection_id = Uuid::new_v4();

        // 创建心跳消息
        let chat_msg = crate::app::model::ChatMessage::new_ping();

        let result = handle_structured_message(chat_msg, &claims, &app_state, &connection_id).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_structured_message_text() {
        // 测试WebSocket结构化消息处理 - 文本消息
        let app_state = create_test_app_state().await;
        let now = Utc::now();
        let claims = crate::app::utils::Claims {
            sub: Uuid::new_v4().to_string(),
            username: "test_user".to_string(),
            exp: (now + chrono::Duration::hours(1)).timestamp(),
            iat: now.timestamp(),
        };
        let connection_id = Uuid::new_v4();

        // 创建文本消息
        let chat_msg = crate::app::model::ChatMessage::new_text("Hello, World!".to_string());

        let result = handle_structured_message(chat_msg, &claims, &app_state, &connection_id).await;

        assert!(result.is_ok());
    }
}
