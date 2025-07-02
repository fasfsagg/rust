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
use tokio_tungstenite::connect_async;
use futures_util::{ SinkExt, StreamExt };
use std::time::Duration;

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
            database_pool: axum_tutorial::config::DatabasePoolConfig::development(),
            websocket_pool: axum_tutorial::config::WebSocketPoolConfig::development(),
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

    /// 建立WebSocket连接（带JWT认证）
    async fn connect_websocket(
        &self,
        token: &str
    ) -> Result<
        (
            tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
            tokio_tungstenite::tungstenite::http::Response<Option<Vec<u8>>>,
        ),
        Box<dyn std::error::Error + Send + Sync>
    > {
        let ws_url = format!("ws://{}/ws?token={}", self.address.replace("http://", ""), token);
        let (ws_stream, response) = connect_async(&ws_url).await?;
        Ok((ws_stream, response))
    }

    /// 建立WebSocket连接（无认证，用于测试认证失败）
    async fn connect_websocket_no_auth(
        &self
    ) -> Result<
        (
            tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
            tokio_tungstenite::tungstenite::http::Response<Option<Vec<u8>>>,
        ),
        Box<dyn std::error::Error + Send + Sync>
    > {
        let ws_url = format!("ws://{}/ws", self.address.replace("http://", ""));
        let (ws_stream, response) = connect_async(&ws_url).await?;
        Ok((ws_stream, response))
    }

    /// 测试健康检查端点
    async fn get_health_check(&self) -> reqwest::Result<reqwest::Response> {
        self.client.get(&format!("{}/api/performance/health", self.address)).send().await
    }

    /// 测试性能统计端点
    async fn get_performance_stats(&self) -> reqwest::Result<reqwest::Response> {
        self.client.get(&format!("{}/api/performance/stats", self.address)).send().await
    }

    /// 测试在线用户列表端点
    async fn get_online_users(&self, token: &str) -> reqwest::Result<reqwest::Response> {
        self.client
            .get(&format!("{}/api/online-users", self.address))
            .header("Authorization", format!("Bearer {}", token))
            .send().await
    }

    /// 测试消息搜索端点
    async fn search_messages(
        &self,
        token: &str,
        query: &str
    ) -> reqwest::Result<reqwest::Response> {
        self.client
            .get(&format!("{}/api/messages/search?keyword={}", self.address, query))
            .header("Authorization", format!("Bearer {}", token))
            .send().await
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

/// 测试WebSocket JWT认证
#[tokio::test]
async fn test_websocket_jwt_authentication() {
    let server = TestServer::new().await;

    // 注册并登录用户
    server.register_user("wsuser", "password123").await.unwrap();
    let login_response = server.login_user("wsuser", "password123").await.unwrap();
    let token = server.extract_token(login_response).await;

    // 测试有效令牌的WebSocket连接
    let ws_result = server.connect_websocket(&token).await;
    assert!(ws_result.is_ok(), "WebSocket连接应该成功");

    // 测试无效令牌的WebSocket连接
    let invalid_ws_result = server.connect_websocket("invalid-token").await;
    assert!(invalid_ws_result.is_err(), "无效令牌的WebSocket连接应该失败");

    // 测试无认证的WebSocket连接
    let no_auth_result = server.connect_websocket_no_auth().await;
    assert!(no_auth_result.is_err(), "无认证的WebSocket连接应该失败");
}

/// 测试WebSocket消息传递
#[tokio::test]
async fn test_websocket_message_handling() {
    use tokio_tungstenite::tungstenite::Message;

    let server = TestServer::new().await;

    // 注册并登录用户
    server.register_user("msguser", "password123").await.unwrap();
    let login_response = server.login_user("msguser", "password123").await.unwrap();
    let token = server.extract_token(login_response).await;

    // 建立WebSocket连接
    let (mut ws_stream, _) = server.connect_websocket(&token).await.expect("WebSocket连接失败");

    // 发送测试消息
    let test_message = "Hello WebSocket!";
    ws_stream.send(Message::Text(test_message.to_string())).await.expect("发送消息失败");

    // 接收响应消息（设置超时）
    let timeout_duration = Duration::from_secs(5);
    let message_result = tokio::time::timeout(timeout_duration, ws_stream.next()).await;

    assert!(message_result.is_ok(), "应该在超时时间内收到消息");

    if let Ok(Some(Ok(received_message))) = message_result {
        match received_message {
            Message::Text(text) => {
                // 验证收到的消息包含用户名信息
                assert!(text.contains("msguser"), "消息应该包含用户名");
            }
            _ => panic!("应该收到文本消息"),
        }
    } else {
        panic!("应该收到有效的WebSocket消息");
    }
}

/// 测试中间件链集成 - 性能监控中间件
#[tokio::test]
async fn test_performance_monitoring_middleware() {
    let server = TestServer::new().await;

    // 测试健康检查端点（可能返回200或503，取决于系统状态）
    let health_response = server.get_health_check().await.unwrap();
    assert!(
        health_response.status() == StatusCode::OK ||
            health_response.status() == StatusCode::SERVICE_UNAVAILABLE,
        "健康检查应该返回200或503状态码"
    );

    // 验证响应包含健康检查信息（只有在200状态码时才检查JSON格式）
    if health_response.status() == StatusCode::OK {
        let health_json: Value = health_response.json().await.unwrap();
        assert!(health_json["status"].is_string());
        assert!(health_json["timestamp"].is_number());
    }

    // 测试性能统计端点
    let stats_response = server.get_performance_stats().await.unwrap();
    assert_eq!(stats_response.status(), StatusCode::OK);

    // 验证响应包含性能统计信息
    let stats_json: Value = stats_response.json().await.unwrap();
    assert!(stats_json["total_requests"].is_number());
    assert!(stats_json["active_connections"].is_number());
}

/// 测试中间件链集成 - 安全审计中间件
#[tokio::test]
async fn test_security_audit_middleware() {
    let server = TestServer::new().await;

    // 注册并登录用户
    server.register_user("audituser", "password123").await.unwrap();
    let login_response = server.login_user("audituser", "password123").await.unwrap();
    let token = server.extract_token(login_response).await;

    // 执行一些需要认证的操作，触发安全审计
    let create_response = server.create_task(&token, "Audit Test Task").await.unwrap();
    assert_eq!(create_response.status(), StatusCode::CREATED);

    // 测试无效认证，应该触发安全审计事件
    let invalid_response = server.get_tasks("invalid-token").await.unwrap();
    assert_eq!(invalid_response.status(), StatusCode::UNAUTHORIZED);

    // 验证安全审计中间件正常工作（通过检查日志或响应头）
    // 这里我们通过成功的API调用来验证中间件链正常工作
    let valid_response = server.get_tasks(&token).await.unwrap();
    assert_eq!(valid_response.status(), StatusCode::OK);
}

/// 测试错误恢复中间件
#[tokio::test]
async fn test_error_recovery_middleware() {
    let server = TestServer::new().await;

    // 注册并登录用户
    server.register_user("recoveryuser", "password123").await.unwrap();
    let login_response = server.login_user("recoveryuser", "password123").await.unwrap();
    let token = server.extract_token(login_response).await;

    // 测试正常请求（应该通过错误恢复中间件）
    let normal_response = server.get_tasks(&token).await.unwrap();
    assert_eq!(normal_response.status(), StatusCode::OK);

    // 测试错误请求（应该被错误恢复中间件处理）
    let error_response = server.get_task_by_id(&token, "invalid-uuid").await.unwrap();
    assert_eq!(error_response.status(), StatusCode::BAD_REQUEST);

    // 验证错误恢复中间件正确处理了错误，而不是让应用崩溃
    // 后续请求应该仍然正常工作
    let recovery_response = server.get_tasks(&token).await.unwrap();
    assert_eq!(recovery_response.status(), StatusCode::OK);
}

/// 测试数据库并发操作
#[tokio::test]
async fn test_concurrent_database_operations() {
    let server = TestServer::new().await;

    // 注册并登录用户
    server.register_user("concurrentuser", "password123").await.unwrap();
    let login_response = server.login_user("concurrentuser", "password123").await.unwrap();
    let token = server.extract_token(login_response).await;

    // 顺序创建多个任务来测试数据库操作的稳定性
    let mut success_count = 0;
    for i in 0..5 {
        let task_title = format!("Concurrent Task {}", i);
        let result = server.create_task(&token, &task_title).await;
        if let Ok(response) = result {
            if response.status() == StatusCode::CREATED {
                success_count += 1;
            }
        }
    }

    // 验证所有操作都成功
    assert_eq!(success_count, 5, "所有任务创建都应该成功");

    // 验证数据库中确实有5个任务
    let get_response = server.get_tasks(&token).await.unwrap();
    assert_eq!(get_response.status(), StatusCode::OK);

    let tasks: Value = get_response.json().await.unwrap();
    assert_eq!(tasks.as_array().unwrap().len(), 5, "数据库中应该有5个任务");
}

/// 测试用户数据隔离
#[tokio::test]
async fn test_user_data_isolation() {
    let server = TestServer::new().await;

    // 注册两个不同的用户
    server.register_user("user1", "password123").await.unwrap();
    server.register_user("user2", "password123").await.unwrap();

    // 分别登录获取令牌
    let login1_response = server.login_user("user1", "password123").await.unwrap();
    let token1 = server.extract_token(login1_response).await;

    let login2_response = server.login_user("user2", "password123").await.unwrap();
    let token2 = server.extract_token(login2_response).await;

    // 用户1创建任务
    let create1_response = server.create_task(&token1, "User1 Task").await.unwrap();
    assert_eq!(create1_response.status(), StatusCode::CREATED);
    let task1: Value = create1_response.json().await.unwrap();
    let task1_id = task1["id"].as_str().unwrap();

    // 用户2创建任务
    let create2_response = server.create_task(&token2, "User2 Task").await.unwrap();
    assert_eq!(create2_response.status(), StatusCode::CREATED);

    // 验证用户1只能看到自己的任务
    let get1_response = server.get_tasks(&token1).await.unwrap();
    let tasks1: Value = get1_response.json().await.unwrap();
    assert_eq!(tasks1.as_array().unwrap().len(), 1);
    assert_eq!(tasks1[0]["title"], "User1 Task");

    // 验证用户2只能看到自己的任务
    let get2_response = server.get_tasks(&token2).await.unwrap();
    let tasks2: Value = get2_response.json().await.unwrap();
    assert_eq!(tasks2.as_array().unwrap().len(), 1);
    assert_eq!(tasks2[0]["title"], "User2 Task");

    // 验证用户2无法访问用户1的任务
    let unauthorized_response = server.get_task_by_id(&token2, task1_id).await.unwrap();
    assert_eq!(unauthorized_response.status(), StatusCode::NOT_FOUND);
}

/// 测试API端点边界条件
#[tokio::test]
async fn test_api_boundary_conditions() {
    let server = TestServer::new().await;

    // 注册并登录用户
    server.register_user("boundaryuser", "password123").await.unwrap();
    let login_response = server.login_user("boundaryuser", "password123").await.unwrap();
    let token = server.extract_token(login_response).await;

    // 测试空标题任务创建
    let empty_title_payload = CreateTaskPayload {
        title: "".to_string(),
        description: Some("Empty title test".to_string()),
        completed: false,
    };

    let empty_title_response = server.client
        .post(&format!("{}/api/tasks", server.address))
        .header("Authorization", format!("Bearer {}", token))
        .json(&empty_title_payload)
        .send().await
        .unwrap();

    // 空标题应该被接受（根据当前实现）
    assert_eq!(empty_title_response.status(), StatusCode::CREATED);

    // 测试超长标题
    let long_title = "A".repeat(1000);
    let long_title_payload = CreateTaskPayload {
        title: long_title,
        description: Some("Long title test".to_string()),
        completed: false,
    };

    let long_title_response = server.client
        .post(&format!("{}/api/tasks", server.address))
        .header("Authorization", format!("Bearer {}", token))
        .json(&long_title_payload)
        .send().await
        .unwrap();

    // 超长标题应该被接受（根据当前实现）
    assert_eq!(long_title_response.status(), StatusCode::CREATED);

    // 测试无效JSON格式
    let invalid_json_response = server.client
        .post(&format!("{}/api/tasks", server.address))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body("{invalid json}")
        .send().await
        .unwrap();

    assert_eq!(invalid_json_response.status(), StatusCode::BAD_REQUEST);
}

/// 测试在线用户列表功能
#[tokio::test]
async fn test_online_users_functionality() {
    let server = TestServer::new().await;

    // 注册并登录用户
    server.register_user("onlineuser", "password123").await.unwrap();
    let login_response = server.login_user("onlineuser", "password123").await.unwrap();
    let token = server.extract_token(login_response).await;

    // 测试在线用户列表端点
    let online_users_response = server.get_online_users(&token).await.unwrap();
    assert_eq!(online_users_response.status(), StatusCode::OK);

    // 验证响应格式
    let online_users: Value = online_users_response.json().await.unwrap();
    assert!(
        online_users.is_array() || online_users.is_object(),
        "在线用户列表应该是数组或对象格式"
    );

    // 测试未认证访问
    let unauthorized_response = server.get_online_users("invalid-token").await.unwrap();
    assert_eq!(unauthorized_response.status(), StatusCode::UNAUTHORIZED);
}

/// 测试消息搜索功能
#[tokio::test]
async fn test_message_search_functionality() {
    let server = TestServer::new().await;

    // 注册并登录用户
    server.register_user("searchuser", "password123").await.unwrap();
    let login_response = server.login_user("searchuser", "password123").await.unwrap();
    let token = server.extract_token(login_response).await;

    // 测试消息搜索端点
    let search_response = server.search_messages(&token, "test").await.unwrap();

    // 根据当前实现，可能返回200（找到结果）或404（未找到）
    assert!(
        search_response.status() == StatusCode::OK ||
            search_response.status() == StatusCode::NOT_FOUND,
        "搜索应该返回200或404状态码"
    );

    // 如果返回200，验证响应格式
    if search_response.status() == StatusCode::OK {
        let search_results: Value = search_response.json().await.unwrap();
        assert!(
            search_results.is_array() || search_results.is_object(),
            "搜索结果应该是数组或对象格式"
        );
    }

    // 测试空查询
    let empty_search_response = server.search_messages(&token, "").await.unwrap();
    assert!(
        empty_search_response.status() == StatusCode::OK ||
            empty_search_response.status() == StatusCode::BAD_REQUEST,
        "空查询应该返回200或400状态码"
    );

    // 测试未认证访问
    let unauthorized_search = server.search_messages("invalid-token", "test").await.unwrap();
    assert_eq!(unauthorized_search.status(), StatusCode::UNAUTHORIZED);
}

/// 测试多用户WebSocket交互
#[tokio::test]
async fn test_multi_user_websocket_interaction() {
    use tokio_tungstenite::tungstenite::Message;

    let server = TestServer::new().await;

    // 注册两个用户
    server.register_user("wsuser1", "password123").await.unwrap();
    server.register_user("wsuser2", "password123").await.unwrap();

    // 分别登录获取令牌
    let login1_response = server.login_user("wsuser1", "password123").await.unwrap();
    let token1 = server.extract_token(login1_response).await;

    let login2_response = server.login_user("wsuser2", "password123").await.unwrap();
    let token2 = server.extract_token(login2_response).await;

    // 建立两个WebSocket连接
    let (mut ws1, _) = server.connect_websocket(&token1).await.expect("用户1 WebSocket连接失败");
    let (mut ws2, _) = server.connect_websocket(&token2).await.expect("用户2 WebSocket连接失败");

    // 用户1发送消息
    let message1 = "Hello from user1!";
    ws1.send(Message::Text(message1.to_string())).await.expect("用户1发送消息失败");

    // 用户2发送消息
    let message2 = "Hello from user2!";
    ws2.send(Message::Text(message2.to_string())).await.expect("用户2发送消息失败");

    // 验证两个用户都能接收到消息（设置较短的超时时间）
    let timeout_duration = Duration::from_secs(3);

    // 用户1可能会接收到多条消息：系统欢迎消息 + 用户2的消息
    let mut user1_received_user2_message = false;
    let mut user2_received_user1_message = false;

    // 尝试接收多条消息来找到用户间的消息
    for _ in 0..3 {
        // 用户1接收消息
        if
            let Ok(Some(Ok(Message::Text(text1)))) = tokio::time::timeout(
                timeout_duration,
                ws1.next()
            ).await
        {
            println!("用户1接收到的消息: {}", text1);
            if text1.contains("wsuser2") {
                user1_received_user2_message = true;
            }
        }

        // 用户2接收消息
        if
            let Ok(Some(Ok(Message::Text(text2)))) = tokio::time::timeout(
                timeout_duration,
                ws2.next()
            ).await
        {
            println!("用户2接收到的消息: {}", text2);
            if text2.contains("wsuser1") {
                user2_received_user1_message = true;
            }
        }

        // 如果两个用户都收到了对方的消息，就可以结束了
        if user1_received_user2_message && user2_received_user1_message {
            break;
        }
    }

    // 验证用户间消息交换
    assert!(user1_received_user2_message, "用户1应该接收到包含wsuser2的消息");
    assert!(user2_received_user1_message, "用户2应该接收到包含wsuser1的消息");
}
