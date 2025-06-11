//! tests/task_api_tests.rs

use axum_tutorial::app::model::task::{ CreateTaskPayload, Task, UpdateTaskPayload };
use reqwest::{ Client, StatusCode };
use serde::Deserialize;
use uuid::Uuid;

// 引入通用测试辅助模块
mod common;

// 定义用于反序列化错误响应的辅助结构体
// 这个结构必须匹配 `error.rs` 中 `impl IntoResponse for AppError` 构建的 JSON 结构
#[derive(Deserialize, Debug)]
struct ErrorResponse {
    error: ErrorDetails,
}

#[derive(Deserialize, Debug)]
struct ErrorDetails {
    message: String,
    code: u16,
}

/// 辅助函数：创建一个任务，并返回其完整模型
///
/// 这个函数会向 `POST /api/tasks` 端点发送请求来创建一个任务，
/// 然后解析响应体，返回创建好的 `Task` 对象。
/// 这在多个测试中需要创建前置数据的场景下非常有用，可以减少代码重复。
///
/// # 参数
/// * `client`: 一个 `&reqwest::Client` 实例。
/// * `api_base_url`: 测试服务器的基地址，例如 "http://127.0.0.1:1234"。
///
/// # 返回
/// * `Task`: 成功创建的任务。
async fn create_task_helper(client: &Client, api_base_url: &str) -> Task {
    let payload = CreateTaskPayload {
        title: "Test Task via Helper".to_string(),
        description: Some("Helper-created description".to_string()),
        completed: false,
    };

    client
        .post(&format!("{}/api/tasks", api_base_url))
        .json(&payload)
        .send().await
        .expect("Failed to send create request")
        .json::<Task>().await
        .expect("Failed to parse create response")
}

#[tokio::test]
async fn test_create_task_success() {
    // --- 准备 (Arrange) ---
    let app = common::spawn_app().await; // 启动应用
    let client = Client::new(); // 创建 HTTP 客户端
    let api_base_url = format!("http://{}", app.http_addr);

    let payload = CreateTaskPayload {
        title: "My Test Task".to_string(),
        description: Some("This is a detailed description.".to_string()),
        completed: false, // 在创建时显式设置为 false
    };

    // --- 执行 (Act) ---
    let response = client
        .post(&format!("{}/api/tasks", api_base_url))
        .json(&payload)
        .send().await
        .expect("Failed to execute request.");

    // --- 断言 (Assert) ---
    assert_eq!(response.status(), StatusCode::CREATED, "应该返回 201 Created 状态");

    // 解析响应体以进行更详细的验证
    let created_task: Task = response.json().await.expect("Failed to parse JSON body");

    assert_eq!(created_task.title, payload.title);
    assert_eq!(created_task.description, payload.description);
    assert!(!created_task.completed); // 确认 completed 字段为 false
}

#[tokio::test]
async fn test_get_all_tasks_success() {
    // --- 准备 (Arrange) ---
    let app = common::spawn_app().await;
    let client = Client::new();
    let api_base_url = format!("http://{}", app.http_addr);

    // 使用辅助函数创建两个任务作为前置数据
    let task1 = create_task_helper(&client, &api_base_url).await;
    let task2 = create_task_helper(&client, &api_base_url).await;

    // --- 执行 (Act) ---
    let response = client
        .get(&format!("{}/api/tasks", api_base_url))
        .send().await
        .expect("Failed to execute request.");

    // --- 断言 (Assert) ---
    assert_eq!(response.status(), StatusCode::OK);

    let tasks: Vec<Task> = response.json().await.expect("Failed to parse JSON body");

    assert_eq!(tasks.len(), 2, "响应中应该包含两个任务");
    // 检查返回的列表中是否包含我们创建的任务
    assert!(tasks.iter().any(|t| t.id == task1.id));
    assert!(tasks.iter().any(|t| t.id == task2.id));
}

#[tokio::test]
async fn test_get_task_by_id_success() {
    // --- 准备 (Arrange) ---
    let app = common::spawn_app().await;
    let client = Client::new();
    let api_base_url = format!("http://{}", app.http_addr);

    let created_task = create_task_helper(&client, &api_base_url).await;

    // --- 执行 (Act) ---
    let response = client
        .get(&format!("{}/api/tasks/{}", api_base_url, created_task.id))
        .send().await
        .expect("Failed to execute request.");

    // --- 断言 (Assert) ---
    assert_eq!(response.status(), StatusCode::OK);

    let fetched_task: Task = response.json().await.expect("Failed to parse JSON body");

    assert_eq!(fetched_task.id, created_task.id);
    assert_eq!(fetched_task.title, created_task.title);
}

#[tokio::test]
async fn test_get_task_by_id_not_found() {
    // --- 准备 (Arrange) ---
    let app = common::spawn_app().await;
    let client = Client::new();
    let api_base_url = format!("http://{}", app.http_addr);
    let non_existent_id = Uuid::new_v4(); // 一个不存在的 ID

    // --- 执行 (Act) ---
    let response = client
        .get(&format!("{}/api/tasks/{}", api_base_url, non_existent_id))
        .send().await
        .expect("Failed to execute request.");

    // --- 断言 (Assert) ---
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // 验证响应体是否符合我们预期的错误格式
    let error_body: ErrorResponse = response.json().await.expect("Failed to parse error body");
    assert_eq!(error_body.error.code, 404);
    assert!(
        error_body.error.message.contains(&format!("未找到ID为 {} 的任务", non_existent_id)),
        "错误消息不符合预期"
    );
}

#[tokio::test]
async fn test_update_task_success() {
    // --- 准备 (Arrange) ---
    let app = common::spawn_app().await;
    let client = Client::new();
    let api_base_url = format!("http://{}", app.http_addr);

    let created_task = create_task_helper(&client, &api_base_url).await;
    let update_payload = UpdateTaskPayload {
        title: Some("Updated Title".to_string()),
        description: Some(Some("Updated description.".to_string())),
        completed: Some(true),
    };

    // --- 执行 (Act) ---
    let response = client
        .put(&format!("{}/api/tasks/{}", api_base_url, created_task.id))
        .json(&update_payload)
        .send().await
        .expect("Failed to execute request.");

    // --- 断言 (Assert) ---
    assert_eq!(response.status(), StatusCode::OK);

    let updated_task: Task = response.json().await.expect("Failed to parse JSON body");

    assert_eq!(updated_task.id, created_task.id);
    assert_eq!(updated_task.title, "Updated Title");
    assert_eq!(updated_task.description.as_deref(), Some("Updated description."));
    assert!(updated_task.completed);
}

#[tokio::test]
async fn test_delete_task_success() {
    // --- 准备 (Arrange) ---
    let app = common::spawn_app().await;
    let client = Client::new();
    let api_base_url = format!("http://{}", app.http_addr);

    let created_task = create_task_helper(&client, &api_base_url).await;

    // --- 执行 (Act) ---
    // 第一步：删除任务
    let delete_response = client
        .delete(&format!("{}/api/tasks/{}", api_base_url, created_task.id))
        .send().await
        .expect("Failed to execute delete request.");

    // 第二步：尝试获取已删除的任务
    let get_response = client
        .get(&format!("{}/api/tasks/{}", api_base_url, created_task.id))
        .send().await
        .expect("Failed to execute get request.");

    // --- 断言 (Assert) ---
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT, "删除操作应该返回 204 No Content");
    assert_eq!(
        get_response.status(),
        StatusCode::NOT_FOUND,
        "获取已删除的任务应该返回 404 Not Found"
    );
}
