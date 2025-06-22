// 集成测试：验证 Axum 0.8.4 升级后的 API 端点功能
// 重点测试路径参数解析和认证中间件功能

use axum_tutorial::{
    app::model::{ task::{ CreateTaskPayload, UpdateTaskPayload } },
    config::AppConfig,
    startup::init_app,
};
use reqwest::{ Client, StatusCode };
use serde::Serialize;
use serde_json::Value;
use tokio::net::TcpListener;
use uuid::Uuid;

// 为测试添加 Serialize 支持
#[derive(Serialize)]
struct TestRegisterRequest {
    username: String,
    password: String,
    #[serde(rename = "confirmPassword")]
    confirm_password: String,
}

#[derive(Serialize)]
struct TestLoginRequest {
    username: String,
    password: String,
}

/// 测试服务器结构体
struct TestServer {
    address: String,
    client: Client,
}

impl TestServer {
    /// 启动测试服务器
    async fn new() -> Self {
        // 创建应用配置
        let config = AppConfig {
            http_addr: "127.0.0.1:0".parse().unwrap(), // 使用随机端口
            database_url: "sqlite::memory:".to_string(), // 使用内存数据库
            jwt_secret: "test-secret-key-for-integration-tests".to_string(),
        };

        // 创建应用
        let (app, _db) = init_app(config.clone()).await.expect("Failed to create app");

        // 绑定到随机端口
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());

        // 启动服务器
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // 等待服务器启动
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Self {
            address,
            client: Client::new(),
        }
    }

    /// 用户注册
    async fn register_user(
        &self,
        username: &str,
        password: &str
    ) -> reqwest::Result<reqwest::Response> {
        let payload = TestRegisterRequest {
            username: username.to_string(),
            password: password.to_string(),
            confirm_password: password.to_string(),
        };

        self.client.post(&format!("{}/api/auth/register", self.address)).json(&payload).send().await
    }

    /// 用户登录
    async fn login_user(
        &self,
        username: &str,
        password: &str
    ) -> reqwest::Result<reqwest::Response> {
        let payload = TestLoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };

        self.client.post(&format!("{}/api/auth/login", self.address)).json(&payload).send().await
    }

    /// 从登录响应中提取 JWT 令牌
    async fn extract_token(&self, response: reqwest::Response) -> String {
        let json: Value = response.json().await.unwrap();
        json["access_token"].as_str().unwrap().to_string()
    }

    /// 创建任务
    async fn create_task(&self, token: &str, title: &str) -> reqwest::Result<reqwest::Response> {
        let payload = CreateTaskPayload {
            title: title.to_string(),
            description: Some("Integration test task".to_string()),
            completed: false,
        };

        self.client
            .post(&format!("{}/api/tasks", self.address))
            .header("Authorization", format!("Bearer {}", token))
            .json(&payload)
            .send().await
    }

    /// 获取所有任务
    async fn get_tasks(&self, token: &str) -> reqwest::Result<reqwest::Response> {
        self.client
            .get(&format!("{}/api/tasks", self.address))
            .header("Authorization", format!("Bearer {}", token))
            .send().await
    }

    /// 根据 ID 获取任务（测试路径参数）
    async fn get_task_by_id(
        &self,
        token: &str,
        task_id: &str
    ) -> reqwest::Result<reqwest::Response> {
        self.client
            .get(&format!("{}/api/tasks/{}", self.address, task_id))
            .header("Authorization", format!("Bearer {}", token))
            .send().await
    }

    /// 更新任务（测试路径参数）
    async fn update_task(&self, token: &str, task_id: &str) -> reqwest::Result<reqwest::Response> {
        let payload = UpdateTaskPayload {
            title: Some("Updated Task".to_string()),
            description: Some(Some("Updated description".to_string())),
            completed: Some(true),
        };

        self.client
            .put(&format!("{}/api/tasks/{}", self.address, task_id))
            .header("Authorization", format!("Bearer {}", token))
            .json(&payload)
            .send().await
    }

    /// 删除任务（测试路径参数）
    async fn delete_task(&self, token: &str, task_id: &str) -> reqwest::Result<reqwest::Response> {
        self.client
            .delete(&format!("{}/api/tasks/{}", self.address, task_id))
            .header("Authorization", format!("Bearer {}", token))
            .send().await
    }

    /// 测试静态文件服务
    async fn get_static_file(&self) -> reqwest::Result<reqwest::Response> {
        self.client.get(&format!("{}/", self.address)).send().await
    }
}

/// 测试用户注册和登录 API
#[tokio::test]
async fn test_user_registration_and_login() {
    let server = TestServer::new().await;

    // 测试用户注册
    let register_response = server.register_user("testuser", "password123").await.unwrap();
    assert_eq!(register_response.status(), StatusCode::CREATED);

    // 测试用户登录
    let login_response = server.login_user("testuser", "password123").await.unwrap();
    assert_eq!(login_response.status(), StatusCode::OK);

    // 验证登录响应包含令牌
    let json: Value = login_response.json().await.unwrap();
    assert!(json["access_token"].is_string());
    assert_eq!(json["token_type"], "Bearer");
}

/// 测试 JWT 认证中间件
#[tokio::test]
async fn test_jwt_authentication_middleware() {
    let server = TestServer::new().await;

    // 测试未认证访问（应该返回 401）
    let response = server.get_tasks("invalid-token").await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 注册并登录用户
    server.register_user("authuser", "password123").await.unwrap();
    let login_response = server.login_user("authuser", "password123").await.unwrap();
    let token = server.extract_token(login_response).await;

    // 测试有效令牌访问（应该成功）
    let response = server.get_tasks(&token).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

/// 测试任务 CRUD API 和路径参数解析
#[tokio::test]
async fn test_task_crud_and_path_parameters() {
    let server = TestServer::new().await;

    // 注册并登录用户
    server.register_user("taskuser", "password123").await.unwrap();
    let login_response = server.login_user("taskuser", "password123").await.unwrap();
    let token = server.extract_token(login_response).await;

    // 1. 创建任务
    let create_response = server.create_task(&token, "Test Task").await.unwrap();
    assert_eq!(create_response.status(), StatusCode::CREATED);

    let created_task: Value = create_response.json().await.unwrap();
    let task_id = created_task["id"].as_str().unwrap();

    // 2. 获取所有任务
    let get_all_response = server.get_tasks(&token).await.unwrap();
    assert_eq!(get_all_response.status(), StatusCode::OK);

    // 3. 根据 ID 获取任务（测试路径参数 {id}）
    let get_by_id_response = server.get_task_by_id(&token, task_id).await.unwrap();
    assert_eq!(get_by_id_response.status(), StatusCode::OK);

    let task: Value = get_by_id_response.json().await.unwrap();
    assert_eq!(task["title"], "Test Task");

    // 4. 更新任务（测试路径参数 {id}）
    let update_response = server.update_task(&token, task_id).await.unwrap();
    assert_eq!(update_response.status(), StatusCode::OK);

    // 5. 删除任务（测试路径参数 {id}）
    let delete_response = server.delete_task(&token, task_id).await.unwrap();
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    // 6. 验证任务已删除
    let get_deleted_response = server.get_task_by_id(&token, task_id).await.unwrap();
    assert_eq!(get_deleted_response.status(), StatusCode::NOT_FOUND);
}

/// 测试静态文件服务
#[tokio::test]
async fn test_static_file_service() {
    let server = TestServer::new().await;

    let response = server.get_static_file().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 验证返回的是 HTML 内容
    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("text/html"));
}

/// 测试路径参数错误处理
#[tokio::test]
async fn test_path_parameter_error_handling() {
    let server = TestServer::new().await;

    // 注册并登录用户
    server.register_user("erroruser", "password123").await.unwrap();
    let login_response = server.login_user("erroruser", "password123").await.unwrap();
    let token = server.extract_token(login_response).await;

    // 测试无效的 UUID 格式
    let response = server.get_task_by_id(&token, "invalid-uuid").await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // 测试不存在的任务 ID
    let fake_uuid = Uuid::new_v4().to_string();
    let response = server.get_task_by_id(&token, &fake_uuid).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
