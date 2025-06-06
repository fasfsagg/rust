// src/app/controller/task_controller.rs

// This module handles HTTP requests related to tasks, acting as an interface
// between the web framework (Axum) and the business logic (service layer).
// It uses extractors to parse request data, calls service functions,
// and constructs HTTP responses.

// --- Imports ---
use axum::{
    extract::{ Path, State, ws::{ WebSocket, Message, WebSocketUpgrade } },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
// 导入模型层定义的载荷结构体 (task::Model will be used for responses)
// Also CreateTaskPayload, UpdateTaskPayload for request bodies.
use crate::app::model::{task, CreateTaskPayload, UpdateTaskPayload};
// 导入服务层模块。
use crate::app::service;
// 导入自定义错误类型和 Result 别名。
use crate::error::{ self, AppError, Result }; // 显式导入 AppError
// 导入新的 AppState
use crate::app::AppState;

// --- 任务相关的 HTTP 处理函数 (Task Handlers) ---

/// Handler for creating a new task.
///
/// Receives task data via `CreateTaskPayload` in the JSON request body.
/// Calls the task service to perform the creation logic using SeaORM.
/// Returns the created task as JSON with a 201 status code.
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

/// Handler for retrieving all tasks.
///
/// Calls the task service to fetch all tasks from the database via SeaORM.
/// Returns a JSON array of tasks with a 200 status code.
/// Handles potential errors from the service layer.
pub async fn get_all_tasks(State(state): State<AppState>) -> impl IntoResponse {
    println!("CONTROLLER: Received get all tasks request");
    // --- 调用服务层 ---
    // `.await`: 因为 `service::get_all_tasks` 是 `async fn`。
    // The service now returns Result<Vec<task::Model>, AppError>
    let tasks_result = service::get_all_tasks(&state.db).await;

    match tasks_result {
        Ok(tasks) => {
            println!("CONTROLLER: Retrieved {} tasks", tasks.len());
            (StatusCode::OK, Json(tasks)).into_response()
        }
        Err(e) => {
            println!("CONTROLLER: Error retrieving tasks: {:?}", e);
            e.into_response()
        }
    }
}

/// Handler for retrieving a single task by its ID.
///
/// Expects an `i32` task ID from the URL path.
/// Calls the task service to fetch the specific task.
/// Returns the task as JSON with a 200 status code if found,
/// or a 404 error if not found.
pub async fn get_task_by_id(
    State(state): State<AppState>,
    Path(id): Path<i32> // ID is now i32
) -> Result<impl IntoResponse> {
    println!("CONTROLLER: Received get task by ID request for '{}'", id);
    // ID is already i32, no parsing needed from string.
    // --- 调用服务层 ---
    let task = service::get_task_by_id(&state.db, id).await?;
    println!("CONTROLLER: Task found for ID: {}", id);

    // --- 构造成功响应 ---
    Ok((StatusCode::OK, Json(task)))
}

/// Handler for updating an existing task.
///
/// Expects an `i32` task ID from the URL path and `UpdateTaskPayload` in the JSON body.
/// Calls the task service to perform the update.
/// Returns the updated task as JSON with a 200 status code,
/// or relevant errors (e.g., 404 if not found).
pub async fn update_task(
    State(state): State<AppState>,
    Path(id): Path<i32>, // ID is now i32
    Json(payload): Json<UpdateTaskPayload> // 从请求体解析更新数据
) -> Result<impl IntoResponse> {
    println!("CONTROLLER: Received update task request for ID: {}", id);
    // ID is already i32.
    // --- 调用服务层 ---
    let task = service::update_task(&state.db, id, payload).await?;
    println!("CONTROLLER: Task updated successfully for ID: {}", id);

    // --- 构造成功响应 ---
    Ok((StatusCode::OK, Json(task)))
}

/// Handler for deleting a task by its ID.
///
/// Expects an `i32` task ID from the URL path.
/// Calls the task service to perform the deletion.
/// Returns a 204 No Content status on successful deletion,
/// or relevant errors (e.g., 404 if not found).
pub async fn delete_task(
    State(state): State<AppState>,
    Path(id): Path<i32> // ID is now i32
) -> Result<impl IntoResponse> {
    println!("CONTROLLER: Received delete task request for ID: {}", id);
    // ID is already i32.
    // --- 调用服务层 ---
    // Service returns the deleted task, but we don't need to use it in the response for DELETE.
    let _deleted_task = service::delete_task(&state.db, id).await?;
    println!("CONTROLLER: Task deleted successfully for ID: {}", id);

    // --- 构造成功响应 ---
    // 直接返回 204 状态码。
    Ok(StatusCode::NO_CONTENT)
}

/// Handler for WebSocket connections.
///
/// Upgrades the HTTP connection to a WebSocket connection and passes it
/// to `handle_socket` for message processing.
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

/// Handles individual WebSocket connections.
///
/// Receives messages from the client and can send messages back.
/// Includes basic echo functionality and connection lifecycle logging.
/// The `state: AppState` argument demonstrates how shared state can be accessed
/// within WebSocket handlers if needed (e.g., for broadcasting or DB interaction).
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
*/

// The parse_uuid function is no longer needed as IDs are i32.
// The error::invalid_uuid helper (in src/error.rs) might still be useful if other string UUIDs
// are parsed elsewhere, but it's not used for task IDs from path anymore.

// --- Protected Data Handler (Example for JWT Auth) ---
// Ensure model::user::Claims is imported if not already covered by model::*
use crate::app::model::user::Claims;
use serde_json::{json, Value};

/// Handler for accessing protected data.
///
/// This handler demonstrates JWT authentication. It requires valid `Claims`
/// to be extracted from the JWT in the request. If authentication fails,
/// the `FromRequestParts` implementation for `Claims` will reject the request
/// before this handler is called.
///
/// Returns a JSON object with a success message and some claims data.
pub async fn protected_data_handler(
    claims: Claims // The Claims struct is automatically extracted and validated by Axum
) -> Result<Json<Value>, AppError> {
    println!(
        "CONTROLLER: Accessing protected data for user_id: {}, username: {}",
        claims.sub, claims.username
    );
    Ok(Json(json!({
        "message": "This is protected data. You are authenticated.",
        "user_id": claims.sub,
        "username": claims.username,
        "company": claims.company,
        "expires_at_timestamp": claims.exp,
        "issued_at_timestamp": claims.iat,
    })))
}
