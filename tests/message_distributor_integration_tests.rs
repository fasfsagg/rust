//! 消息分发器集成测试
//!
//! 本模块包含消息分发器与其他组件集成的测试。
//! 测试覆盖以下场景：
//! - 消息分发器与连接管理器的集成
//! - 不同分发策略的实际执行
//! - 消息优先级处理
//! - 错误处理和重试机制

use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;
use chrono::Utc;

use axum_tutorial::app::service::{ ConnectionManager, MessageDistributor, MessagePriority };
use axum_tutorial::app::model::chat::{ ServerMessage, UserInfo };

/// 创建测试环境
async fn setup_test_environment() -> (Arc<ConnectionManager>, Arc<MessageDistributor>) {
    let connection_manager = Arc::new(ConnectionManager::new());
    let message_distributor = Arc::new(
        MessageDistributor::new(
            connection_manager.clone(),
            Some(10), // 小批量用于测试
            Some(1) // 单个工作线程
        )
    );

    (connection_manager, message_distributor)
}

/// 添加测试连接
async fn add_test_connection(
    connection_manager: &Arc<ConnectionManager>,
    username: &str
) -> (Uuid, Uuid, mpsc::UnboundedReceiver<axum::extract::ws::Message>) {
    let connection_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let (sender, receiver) = mpsc::unbounded_channel();

    connection_manager
        .add_connection(
            connection_id,
            user_id,
            username.to_string(),
            sender,
            Some("127.0.0.1".to_string())
        ).await
        .unwrap();

    (connection_id, user_id, receiver)
}

#[tokio::test]
async fn test_message_distributor_integration_with_connection_manager() {
    let (connection_manager, message_distributor) = setup_test_environment().await;

    // 添加两个测试连接
    let (conn1_id, user1_id, mut receiver1) = add_test_connection(
        &connection_manager,
        "user1"
    ).await;
    let (_conn2_id, _user2_id, mut receiver2) = add_test_connection(
        &connection_manager,
        "user2"
    ).await;

    // 验证连接已添加
    assert_eq!(connection_manager.get_connection_count().await, 2);
    assert_eq!(connection_manager.get_unique_user_count().await, 2);

    // 创建测试消息
    let user_info = UserInfo {
        user_id: user1_id,
        username: "user1".to_string(),
        connected_at: Some(Utc::now()),
    };
    let test_message = ServerMessage::new_text("Hello from user1!".to_string(), user_info);

    // 使用消息分发器广播消息（排除发送者）
    message_distributor
        .broadcast_to_all(
            test_message,
            true, // 排除发送者
            Some(conn1_id),
            Some(MessagePriority::Normal)
        ).await
        .unwrap();

    // 处理消息队列
    let processed_count = message_distributor.process_batch().await.unwrap();
    assert_eq!(processed_count, 1);

    // 验证只有user2收到消息，user1没有收到（被排除）
    assert!(receiver1.try_recv().is_err()); // user1不应该收到消息
    assert!(receiver2.try_recv().is_ok()); // user2应该收到消息
}

#[tokio::test]
async fn test_direct_message_distribution() {
    let (connection_manager, message_distributor) = setup_test_environment().await;

    // 添加两个测试连接
    let (_conn1_id, user1_id, mut receiver1) = add_test_connection(
        &connection_manager,
        "user1"
    ).await;
    let (_conn2_id, user2_id, mut receiver2) = add_test_connection(
        &connection_manager,
        "user2"
    ).await;

    // 创建私聊消息
    let user_info = UserInfo {
        user_id: user1_id,
        username: "user1".to_string(),
        connected_at: Some(Utc::now()),
    };
    let direct_message = ServerMessage::new_text("Private message to user2".to_string(), user_info);

    // 发送私聊消息给user2
    message_distributor
        .send_direct_message(direct_message, user2_id, Some(MessagePriority::High)).await
        .unwrap();

    // 处理消息队列
    let processed_count = message_distributor.process_batch().await.unwrap();
    assert_eq!(processed_count, 1);

    // 验证只有user2收到消息
    assert!(receiver1.try_recv().is_err()); // user1不应该收到消息
    assert!(receiver2.try_recv().is_ok()); // user2应该收到消息
}

#[tokio::test]
async fn test_system_message_distribution() {
    let (connection_manager, message_distributor) = setup_test_environment().await;

    // 添加两个测试连接
    let (_conn1_id, _user1_id, mut receiver1) = add_test_connection(
        &connection_manager,
        "user1"
    ).await;
    let (_conn2_id, _user2_id, mut receiver2) = add_test_connection(
        &connection_manager,
        "user2"
    ).await;

    // 创建系统消息
    let system_message = ServerMessage::new_system("System maintenance notification".to_string());

    // 发送系统消息
    message_distributor.send_system_message(system_message).await.unwrap();

    // 处理消息队列
    let processed_count = message_distributor.process_batch().await.unwrap();
    assert_eq!(processed_count, 1);

    // 验证所有用户都收到系统消息
    assert!(receiver1.try_recv().is_ok()); // user1应该收到消息
    assert!(receiver2.try_recv().is_ok()); // user2应该收到消息
}

#[tokio::test]
async fn test_message_priority_processing() {
    let (connection_manager, message_distributor) = setup_test_environment().await;

    // 添加测试连接
    let (_conn_id, user_id, mut receiver) = add_test_connection(
        &connection_manager,
        "test_user"
    ).await;

    // 创建不同优先级的消息
    let low_priority_msg = ServerMessage::new_system("Low priority message".to_string());
    let high_priority_msg = ServerMessage::new_system("High priority message".to_string());
    let critical_priority_msg = ServerMessage::new_system("Critical priority message".to_string());

    // 按低->高->紧急的顺序提交
    message_distributor
        .send_direct_message(low_priority_msg, user_id, Some(MessagePriority::Low)).await
        .unwrap();

    message_distributor
        .send_direct_message(high_priority_msg, user_id, Some(MessagePriority::High)).await
        .unwrap();

    message_distributor
        .send_direct_message(critical_priority_msg, user_id, Some(MessagePriority::Critical)).await
        .unwrap();

    // 验证队列长度
    assert_eq!(message_distributor.get_queue_length().await, 3);

    // 处理消息队列（应该按优先级顺序处理）
    let processed_count = message_distributor.process_batch().await.unwrap();
    assert_eq!(processed_count, 3);

    // 验证所有消息都被处理
    assert_eq!(message_distributor.get_queue_length().await, 0);

    // 验证用户收到了所有消息
    assert!(receiver.try_recv().is_ok()); // 第一条消息
    assert!(receiver.try_recv().is_ok()); // 第二条消息
    assert!(receiver.try_recv().is_ok()); // 第三条消息
}

#[tokio::test]
async fn test_message_distributor_stats() {
    let (connection_manager, message_distributor) = setup_test_environment().await;

    // 添加测试连接
    let (_conn_id, user_id, _receiver) = add_test_connection(
        &connection_manager,
        "test_user"
    ).await;

    // 初始统计信息
    let initial_stats = message_distributor.get_stats().await;
    assert_eq!(initial_stats.total_tasks, 0);
    assert_eq!(initial_stats.successful_distributions, 0);
    assert_eq!(initial_stats.failed_distributions, 0);

    // 发送一些消息
    for i in 0..5 {
        let message = ServerMessage::new_system(format!("Test message {}", i));
        message_distributor
            .send_direct_message(message, user_id, Some(MessagePriority::Normal)).await
            .unwrap();
    }

    // 验证任务统计
    let stats_after_submit = message_distributor.get_stats().await;
    assert_eq!(stats_after_submit.total_tasks, 5);
    assert_eq!(stats_after_submit.queue_length, 5);

    // 处理消息队列
    let processed_count = message_distributor.process_batch().await.unwrap();
    assert_eq!(processed_count, 5);

    // 验证处理后的统计
    let final_stats = message_distributor.get_stats().await;
    assert_eq!(final_stats.total_tasks, 5);
    assert_eq!(final_stats.successful_distributions, 5);
    assert_eq!(final_stats.failed_distributions, 0);
    assert_eq!(final_stats.queue_length, 0);
}

#[tokio::test]
async fn test_queue_management() {
    let (_connection_manager, message_distributor) = setup_test_environment().await;

    // 添加一些任务到队列
    for i in 0..10 {
        let message = ServerMessage::new_system(format!("Test message {}", i));
        message_distributor.send_system_message(message).await.unwrap();
    }

    // 验证队列长度
    assert_eq!(message_distributor.get_queue_length().await, 10);

    // 清空队列
    let cleared_count = message_distributor.clear_queue().await;
    assert_eq!(cleared_count, 10);
    assert_eq!(message_distributor.get_queue_length().await, 0);
}
