// tests/auth_tests.rs

use axum_tutorial::{
    app::model::user::{Claims, LoginUserPayload, RegisterUserPayload, UserResponse, LoginResponse},
    error::AppError, // For deserializing custom error responses if needed
};
// Removed: use hyper_util::client::legacy::Client;
// Removed: use hyper_util::rt::TokioExecutor;
// Removed: use hyper_utils::{request_post, request_get_with_bearer_token, TestResponse};
use serde_json::json;

mod common; // Import common test setup
// Use request helpers and common error response from common module
use common::{request_post, request_get, TestResponse, ErrorResponse};


#[tokio::test]
async fn test_user_registration_ok() {
    let ctx = common::setup_test_app().await;
    // let client = Client::builder(TokioExecutor::new()).build_http(); // Not needed

    let payload = RegisterUserPayload {
        username: "testuser_reg_ok".to_string(),
        password: "password123".to_string(),
    };

    let res = request_post(
        // Remove client
        "/api/register", // Use path
        &payload,
        None, // No token needed
        ctx.router.clone(), // Use the router from the test context
    )
    .await;

    assert_eq!(res.status(), 200, "Expected status OK for registration"); // Or 201 if you use CREATED

    let user_res: UserResponse = res.json_body().await;
    assert_eq!(user_res.username, "testuser_reg_ok");
}

#[tokio::test]
async fn test_user_registration_conflict() {
    let ctx = common::setup_test_app().await;
    // let client = Client::builder(TokioExecutor::new()).build_http();

    let username = "testuser_conflict".to_string();
    let payload = RegisterUserPayload {
        username: username.clone(),
        password: "password123".to_string(),
    };

    // First registration
    let res1 = request_post("/api/register", &payload, None, ctx.router.clone()).await;
    assert_eq!(res1.status(), 200, "Expected OK for first registration");

    // Attempt to register the same user again
    let res2 = request_post("/api/register", &payload, None, ctx.router.clone()).await;
    assert_eq!(res2.status(), 409, "Expected CONFLICT for duplicate registration");

    let error_res: ErrorResponse = res2.json_body().await;
    assert!(error_res.error.message.contains("already exists"));
}


#[tokio::test]
async fn test_login_ok() {
    let ctx = common::setup_test_app().await;
    // let client = Client::builder(TokioExecutor::new()).build_http();

    let reg_payload = RegisterUserPayload {
        username: "testuser_login_ok".to_string(),
        password: "password123".to_string(),
    };
    request_post("/api/register", &reg_payload, None, ctx.router.clone()).await;
    // Ignore result of registration, assume OK for this test's focus

    let login_payload = LoginUserPayload {
        username: "testuser_login_ok".to_string(),
        password: "password123".to_string(),
    };
    let res = request_post("/api/login", &login_payload, None, ctx.router.clone()).await;
    assert_eq!(res.status(), 200, "Expected OK for successful login");

    let login_res: LoginResponse = res.json_body().await;
    assert!(!login_res.token.is_empty(), "Token should not be empty");
}

#[tokio::test]
async fn test_login_wrong_password() {
    let ctx = common::setup_test_app().await;
    // let client = Client::builder(TokioExecutor::new()).build_http();

    let reg_payload = RegisterUserPayload {
        username: "testuser_wrong_pass".to_string(),
        password: "password123".to_string(),
    };
    request_post("/api/register", &reg_payload, None, ctx.router.clone()).await;

    let login_payload = LoginUserPayload {
        username: "testuser_wrong_pass".to_string(),
        password: "wrongpassword".to_string(),
    };
    let res = request_post("/api/login", &login_payload, None, ctx.router.clone()).await;
    assert_eq!(res.status(), 401, "Expected UNAUTHORIZED for wrong password");

    let error_res: ErrorResponse = res.json_body().await;
    assert!(error_res.error.message.to_lowercase().contains("invalid password"));
}

#[tokio::test]
async fn test_login_user_not_found() {
    let ctx = common::setup_test_app().await;
    // let client = Client::builder(TokioExecutor::new()).build_http();

    let login_payload = LoginUserPayload {
        username: "nonexistentuser_login".to_string(),
        password: "password123".to_string(),
    };
    let res = request_post("/api/login", &login_payload, None, ctx.router.clone()).await;
    assert_eq!(res.status(), 401, "Expected UNAUTHORIZED for user not found"); // UserNotFound maps to 401

    let error_res: ErrorResponse = res.json_body().await;
    assert!(error_res.error.message.to_lowercase().contains("not found"));
}


#[tokio::test]
async fn test_protected_route_no_token() {
    let ctx = common::setup_test_app().await;
    // let client = Client::builder(TokioExecutor::new()).build_http();

    let res = request_get("/api/protected_data", None, ctx.router.clone()).await;
    assert_eq!(res.status(), 401, "Expected UNAUTHORIZED for no token");

    let error_res: ErrorResponse = res.json_body().await;
    assert!(error_res.error.message.contains("Missing or malformed Bearer token"));
}

#[tokio::test]
async fn test_protected_route_with_valid_token() {
    let ctx = common::setup_test_app().await;
    // let client = Client::builder(TokioExecutor::new()).build_http();

    // Register and login to get a token
    let username = "testuser_protected_valid".to_string();
    let password = "password123".to_string();
    let reg_payload = RegisterUserPayload { username: username.clone(), password: password.clone() };
    request_post("/api/register", &reg_payload, None, ctx.router.clone()).await;

    let login_payload = LoginUserPayload { username: username.clone(), password };
    let login_res_wrapper = request_post("/api/login", &login_payload, None, ctx.router.clone()).await;
    let login_res: LoginResponse = login_res_wrapper.json_body().await;
    let token = login_res.token;

    // Access protected route with the token
    let res = request_get("/api/protected_data", Some(&token), ctx.router.clone()).await;
    assert_eq!(res.status(), 200, "Expected OK for protected route with valid token");

    let body_json: serde_json::Value = res.json_body().await;
    assert_eq!(body_json["message"], "This is protected data. You are authenticated.");
    assert_eq!(body_json["username"], username);
}

#[tokio::test]
async fn test_protected_route_with_invalid_token() {
    let ctx = common::setup_test_app().await;
    // let client = Client::builder(TokioExecutor::new()).build_http();
    let invalid_token = "this.is.not.a.valid.jwt";

    let res = request_get("/api/protected_data", Some(invalid_token), ctx.router.clone()).await;
    assert_eq!(res.status(), 401, "Expected UNAUTHORIZED for invalid token");

    let error_res: ErrorResponse = res.json_body().await;
    assert!(error_res.error.message.to_lowercase().contains("invalid token"));
}

// Note: Testing token expiration requires either:
// 1. Setting a very short expiration time in AppConfig for the test, then waiting.
// 2. Mocking time, which is complex in async Rust.
// For this subtask, an explicit expiration test is omitted but would be valuable.

// Removed hyper_utils module from here, it's now in common/mod.rs
[end of tests/auth_tests.rs]
