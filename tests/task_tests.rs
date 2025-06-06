// tests/task_tests.rs

use axum_tutorial::{
    app::model::task::{self as task_model, CreateTaskPayload, UpdateTaskPayload},
    // error::AppError, // For deserializing custom error responses if needed
};
// use hyper_util::client::legacy::Client; // Not needed if using router.oneshot from common utils
// use hyper_util::rt::TokioExecutor;     // Not needed
use serde_json::json;


// Assuming hyper_utils are moved to common or accessible
// For now, let's copy or import them if they are in auth_tests.rs as an inner module
// For a real project, hyper_utils should be in tests/common/mod.rs or similar.
mod common; // Import common test setup
use common::{request_post, request_get, request_put, request_delete, TestResponse, ErrorResponse};


#[tokio::test]
async fn test_create_task_ok() {
    let ctx = common::setup_test_app().await;

    let payload = CreateTaskPayload {
        title: "Test Task Create OK".to_string(),
        description: Some("A description for the task".to_string()),
        completed: false,
    };

    let res = request_post( // Using common::request_post
        "/api/tasks",
        &payload,
        None,
        ctx.router.clone(),
    )
    .await;

    assert_eq!(res.status(), common::StatusCode::CREATED, "Expected 201 CREATED for new task");

    let task_res: task_model::Model = res.json_body().await;
    assert_eq!(task_res.title, "Test Task Create OK");
    assert_eq!(task_res.description.unwrap(), "A description for the task");
    assert!(!task_res.completed);
    assert!(task_res.id > 0); // Should have a positive ID
}

#[tokio::test]
async fn test_get_all_tasks() {
    let ctx = common::setup_test_app().await;

    // Create a few tasks first
    let p1 = CreateTaskPayload { title: "Task 1".to_string(), description: None, completed: false };
    let p2 = CreateTaskPayload { title: "Task 2".to_string(), description: Some("Desc 2".into()), completed: true };
    request_post("/api/tasks", &p1, None, ctx.router.clone()).await;
    request_post("/api/tasks", &p2, None, ctx.router.clone()).await;

    let res = request_get("/api/tasks", None, ctx.router.clone()).await;
    assert_eq!(res.status(), common::StatusCode::OK, "Expected OK for get all tasks");

    let tasks: Vec<task_model::Model> = res.json_body().await;
    assert_eq!(tasks.len(), 2, "Expected two tasks in the list");
    assert!(tasks.iter().any(|t| t.title == "Task 1"));
    assert!(tasks.iter().any(|t| t.title == "Task 2" && t.completed));
}

#[tokio::test]
async fn test_get_task_by_id_ok() {
    let ctx = common::setup_test_app().await;
    let p = CreateTaskPayload { title: "Task For Get By ID".to_string(), description: None, completed: false };
    let created_task_res = request_post("/api/tasks", &p, None, ctx.router.clone()).await;
    let created_task: task_model::Model = created_task_res.json_body().await;

    let res = request_get(&format!("/api/tasks/{}", created_task.id), None, ctx.router.clone()).await;
    assert_eq!(res.status(), common::StatusCode::OK, "Expected OK for get task by ID");

    let fetched_task: task_model::Model = res.json_body().await;
    assert_eq!(fetched_task.id, created_task.id);
    assert_eq!(fetched_task.title, "Task For Get By ID");
}

#[tokio::test]
async fn test_get_task_by_id_not_found() {
    let ctx = common::setup_test_app().await;
    let non_existent_id = 99999;

    let res = request_get(&format!("/api/tasks/{}", non_existent_id), None, ctx.router.clone()).await;
    assert_eq!(res.status(), common::StatusCode::NOT_FOUND, "Expected NOT_FOUND for non-existent task ID");

    let error_res: ErrorResponse = res.json_body().await;
    assert!(error_res.error.message.contains("not found"));
}

#[tokio::test]
async fn test_update_task_ok() {
    let ctx = common::setup_test_app().await;
    let p = CreateTaskPayload { title: "Task Before Update".to_string(), description: Some("Original Desc".into()), completed: false };
    let created_task_res = request_post("/api/tasks", &p, None, ctx.router.clone()).await;
    let created_task: task_model::Model = created_task_res.json_body().await;

    let update_payload = UpdateTaskPayload {
        title: Some("Task After Update".to_string()),
        description: Some("Updated Desc".to_string()),
        completed: Some(true),
    };

    let res = request_put(&format!("/api/tasks/{}", created_task.id), &update_payload, None, ctx.router.clone()).await;
    assert_eq!(res.status(), common::StatusCode::OK, "Expected OK for task update");

    let updated_task: task_model::Model = res.json_body().await;
    assert_eq!(updated_task.id, created_task.id);
    assert_eq!(updated_task.title, "Task After Update");
    assert_eq!(updated_task.description.unwrap(), "Updated Desc");
    assert!(updated_task.completed);
    assert!(updated_task.updated_at > created_task.created_at, "updated_at should be greater than created_at");
}

#[tokio::test]
async fn test_update_task_clear_description() {
    let ctx = common::setup_test_app().await;
    let p = CreateTaskPayload { title: "Task for desc clear".to_string(), description: Some("Original Desc".into()), completed: false };
    let created_task_res = request_post("/api/tasks", &p, None, ctx.router.clone()).await;
    let created_task: task_model::Model = created_task_res.json_body().await;

    let update_payload = UpdateTaskPayload {
        title: None,
        description: None,
        completed: None,
    };

    let res = request_put(&format!("/api/tasks/{}", created_task.id), &update_payload, None, ctx.router.clone()).await;
    assert_eq!(res.status(), common::StatusCode::OK, "Expected OK for task update (clearing description)");

    let updated_task: task_model::Model = res.json_body().await;
    assert_eq!(updated_task.id, created_task.id);
    assert_eq!(updated_task.title, "Task for desc clear"); // Title should be unchanged
    assert!(updated_task.description.is_none(), "Description should be cleared (None)");
}


#[tokio::test]
async fn test_update_task_not_found() {
    let ctx = common::setup_test_app().await;
    let non_existent_id = 99999;
    let update_payload = UpdateTaskPayload { title: Some("Won't matter".to_string()), ..Default::default() };

    let res = request_put(&format!("/api/tasks/{}", non_existent_id), &update_payload, None, ctx.router.clone()).await;
    assert_eq!(res.status(), common::StatusCode::NOT_FOUND, "Expected NOT_FOUND for updating non-existent task");
}

#[tokio::test]
async fn test_delete_task_ok() {
    let ctx = common::setup_test_app().await;
    let p = CreateTaskPayload { title: "Task To Be Deleted".to_string(), description: None, completed: false };
    let created_task_res = request_post("/api/tasks", &p, None, ctx.router.clone()).await;
    let created_task: task_model::Model = created_task_res.json_body().await;

    let res_delete = request_delete(&format!("/api/tasks/{}", created_task.id), None, ctx.router.clone()).await;
    assert_eq!(res_delete.status(), common::StatusCode::NO_CONTENT, "Expected NO_CONTENT for successful delete");

    // Verify it's gone
    let res_get = request_get(&format!("/api/tasks/{}", created_task.id), None, ctx.router.clone()).await;
    assert_eq!(res_get.status(), common::StatusCode::NOT_FOUND, "Expected NOT_FOUND for deleted task");
}

#[tokio::test]
async fn test_delete_task_not_found() {
    let ctx = common::setup_test_app().await;
    let non_existent_id = 99999;

    let res = request_delete(&format!("/api/tasks/{}", non_existent_id), None, ctx.router.clone()).await;
    assert_eq!(res.status(), common::StatusCode::NOT_FOUND, "Expected NOT_FOUND for deleting non-existent task");
}

[end of tests/task_tests.rs]
