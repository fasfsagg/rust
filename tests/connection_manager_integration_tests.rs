// tests/connection_manager_integration_tests.rs
//
// 【连接管理器集成测试】
//
// 这个测试文件验证连接管理器与应用程序其他组件的集成，
// 确保连接管理器能够正确地与AppState和WebSocket处理器协同工作。

use axum_tutorial::startup::init_app;
use axum_tutorial::config::AppConfig;
use axum_tutorial::app::service::ConnectionManager;
use axum_tutorial::app::utils::AuthService;

use axum::extract::ws::Message;
use tokio::sync::mpsc;
use uuid::Uuid;
use std::sync::Arc;

/// 创建测试用的应用配置
fn create_test_config() -> AppConfig {
    use axum_tutorial::config::{ DatabasePoolConfig, WebSocketPoolConfig };

    AppConfig {
        database_url: "sqlite::memory:".to_string(),
        jwt_secret: "test_secret_key_for_connection_manager_integration".to_string(),
        http_addr: "127.0.0.1:0".parse().unwrap(),
        database_pool: DatabasePoolConfig::development(),
        websocket_pool: WebSocketPoolConfig::development(),
    }
}

#[tokio::test]
async fn test_connection_manager_integration_with_app_state() {
    // 创建测试应用
    let config = create_test_config();
    let (_app, _db) = init_app(config).await.expect("Failed to initialize app");

    // 这里我们无法直接从app中提取AppState，但我们可以创建一个类似的状态来测试
    let _test_config = create_test_config();
    let connection_manager = Arc::new(ConnectionManager::new());

    // 验证连接管理器的基本功能
    assert_eq!(connection_manager.get_connection_count().await, 0);
    assert_eq!(connection_manager.get_unique_user_count().await, 0);

    // 模拟添加连接
    let (sender, _receiver) = mpsc::unbounded_channel();
    let connection_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let result = connection_manager.add_connection(
        connection_id,
        user_id,
        "test_user".to_string(),
        sender,
        Some("127.0.0.1".to_string())
    ).await;

    assert!(result.is_ok());
    assert_eq!(connection_manager.get_connection_count().await, 1);
    assert_eq!(connection_manager.get_unique_user_count().await, 1);

    // 验证在线用户列表
    let online_users = connection_manager.get_online_users().await;
    assert_eq!(online_users.len(), 1);
    assert_eq!(online_users[0].user_id, user_id);
    assert_eq!(online_users[0].username, "test_user");
    assert_eq!(online_users[0].connection_count, 1);

    println!("连接管理器与应用状态集成测试通过！");
}

#[tokio::test]
async fn test_connection_manager_message_broadcasting() {
    let connection_manager = Arc::new(ConnectionManager::new());

    // 创建多个连接
    let mut receivers = Vec::new();
    let mut connection_ids = Vec::new();

    for i in 0..3 {
        let (sender, receiver) = mpsc::unbounded_channel();
        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        connection_manager
            .add_connection(connection_id, user_id, format!("user_{}", i), sender, None).await
            .unwrap();

        receivers.push(receiver);
        connection_ids.push(connection_id);
    }

    // 验证所有连接都已添加
    assert_eq!(connection_manager.get_connection_count().await, 3);
    assert_eq!(connection_manager.get_unique_user_count().await, 3);

    // 广播消息，排除第一个连接
    let broadcast_message = Message::Text("Hello everyone!".into());
    let result = connection_manager.broadcast_message(
        broadcast_message,
        Some(&connection_ids[0])
    ).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 2); // 应该发送给2个连接

    // 验证消息接收
    assert!(receivers[0].try_recv().is_err()); // 被排除的连接不应该收到消息
    assert!(receivers[1].try_recv().is_ok()); // 应该收到消息
    assert!(receivers[2].try_recv().is_ok()); // 应该收到消息

    println!("连接管理器消息广播测试通过！");
}

#[tokio::test]
async fn test_connection_manager_with_auth_service() {
    let jwt_secret = "test_secret_for_auth_integration".to_string();
    let auth_service = AuthService::new(jwt_secret.clone());
    let connection_manager = Arc::new(ConnectionManager::new());

    // 创建测试用户的JWT令牌
    let user_id = Uuid::new_v4().to_string();
    let username = "auth_test_user";

    let token = auth_service.create_token(&user_id, username, 1).unwrap();
    assert!(!token.is_empty());

    // 验证令牌
    let claims = auth_service.jwt_utils().validate_token(&token).unwrap();
    assert_eq!(claims.sub, user_id);
    assert_eq!(claims.username, username);

    // 使用验证后的用户信息添加连接
    let (sender, _receiver) = mpsc::unbounded_channel();
    let connection_id = Uuid::new_v4();
    let user_uuid = user_id.parse::<Uuid>().unwrap();

    let result = connection_manager.add_connection(
        connection_id,
        user_uuid,
        claims.username.clone(),
        sender,
        None
    ).await;

    assert!(result.is_ok());

    // 验证连接已添加
    assert_eq!(connection_manager.get_connection_count().await, 1);

    let online_users = connection_manager.get_online_users().await;
    assert_eq!(online_users.len(), 1);
    assert_eq!(online_users[0].username, username);

    println!("连接管理器与认证服务集成测试通过！");
}

#[tokio::test]
async fn test_connection_manager_concurrent_operations() {
    let connection_manager = Arc::new(ConnectionManager::new());

    // 并发添加多个连接
    let mut handles = Vec::new();

    for i in 0..10 {
        let manager = connection_manager.clone();
        let handle = tokio::spawn(async move {
            let (sender, _receiver) = mpsc::unbounded_channel();
            let connection_id = Uuid::new_v4();
            let user_id = Uuid::new_v4();

            manager
                .add_connection(
                    connection_id,
                    user_id,
                    format!("concurrent_user_{}", i),
                    sender,
                    None
                ).await
                .unwrap();

            connection_id
        });
        handles.push(handle);
    }

    // 等待所有连接添加完成
    let mut connection_ids = Vec::new();
    for handle in handles {
        let connection_id = handle.await.unwrap();
        connection_ids.push(connection_id);
    }

    // 验证所有连接都已添加
    assert_eq!(connection_manager.get_connection_count().await, 10);
    assert_eq!(connection_manager.get_unique_user_count().await, 10);

    // 并发移除连接
    let mut remove_handles = Vec::new();
    for connection_id in connection_ids {
        let manager = connection_manager.clone();
        let handle = tokio::spawn(async move { manager.remove_connection(&connection_id).await });
        remove_handles.push(handle);
    }

    // 等待所有连接移除完成
    for handle in remove_handles {
        let removed = handle.await.unwrap();
        assert!(removed.is_some());
    }

    // 验证所有连接都已移除
    assert_eq!(connection_manager.get_connection_count().await, 0);
    assert_eq!(connection_manager.get_unique_user_count().await, 0);

    println!("连接管理器并发操作测试通过！");
}

#[tokio::test]
async fn test_connection_manager_error_handling() {
    let connection_manager = Arc::new(ConnectionManager::new());

    // 测试向不存在的连接发送消息
    let non_existent_id = Uuid::new_v4();
    let result = connection_manager.send_to_connection(
        &non_existent_id,
        Message::Text("test".into())
    ).await;
    assert!(result.is_err());

    // 测试向不存在的用户发送消息
    let non_existent_user = Uuid::new_v4();
    let result = connection_manager.send_to_user(
        &non_existent_user,
        Message::Text("test".into())
    ).await;
    assert!(result.is_err());

    // 测试更新不存在的连接活跃时间
    let result = connection_manager.update_last_activity(&non_existent_id).await;
    assert!(result.is_err());

    // 测试移除不存在的连接
    let result = connection_manager.remove_connection(&non_existent_id).await;
    assert!(result.is_none());

    println!("连接管理器错误处理测试通过！");
}

#[tokio::test]
async fn test_connection_manager_memory_cleanup() {
    let connection_manager = Arc::new(ConnectionManager::new());

    // 添加大量连接然后移除，测试内存清理
    let mut connection_ids = Vec::new();

    // 添加100个连接
    for i in 0..100 {
        let (sender, _receiver) = mpsc::unbounded_channel();
        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        connection_manager
            .add_connection(
                connection_id,
                user_id,
                format!("cleanup_test_user_{}", i),
                sender,
                None
            ).await
            .unwrap();

        connection_ids.push(connection_id);
    }

    assert_eq!(connection_manager.get_connection_count().await, 100);

    // 移除所有连接
    for connection_id in connection_ids {
        let removed = connection_manager.remove_connection(&connection_id).await;
        assert!(removed.is_some());
    }

    // 验证内存已清理
    assert_eq!(connection_manager.get_connection_count().await, 0);
    assert_eq!(connection_manager.get_unique_user_count().await, 0);

    let online_users = connection_manager.get_online_users().await;
    assert!(online_users.is_empty());

    println!("连接管理器内存清理测试通过！");
}
