//! 消息分发器演示程序
//!
//! 这个演示程序展示了消息分发器的核心功能：
//! - 不同的广播策略
//! - 消息优先级处理
//! - 批量处理和工作线程
//! - 统计信息监控

use chrono::Utc;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use axum_tutorial::app::model::chat::{ServerMessage, UserInfo};
use axum_tutorial::app::service::{ConnectionManager, MessageDistributor, MessagePriority};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 消息分发器演示程序启动");
    println!("{}", "=".repeat(50));

    // 创建连接管理器和消息分发器
    let connection_manager = Arc::new(ConnectionManager::new());
    let message_distributor = Arc::new(MessageDistributor::new(
        connection_manager.clone(),
        Some(5), // 批量处理大小
        Some(2), // 工作线程数量
    ));

    println!("✅ 连接管理器和消息分发器已创建");

    // 启动工作线程
    let _worker_handles = message_distributor.start_workers();
    println!("✅ 消息分发器工作线程已启动");

    // 模拟添加用户连接
    let users = vec![
        ("Alice", "alice@example.com"),
        ("Bob", "bob@example.com"),
        ("Charlie", "charlie@example.com"),
    ];

    let mut user_connections = Vec::new();

    for (username, _email) in users {
        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let (sender, receiver) = mpsc::unbounded_channel();

        connection_manager
            .add_connection(
                connection_id,
                user_id,
                username.to_string(),
                sender,
                Some("127.0.0.1".to_string()),
            )
            .await?;

        user_connections.push((connection_id, user_id, username.to_string(), receiver));
        println!("👤 用户 {} 已连接 (ID: {})", username, user_id);
    }

    println!("\n📊 当前连接状态:");
    println!(
        "   总连接数: {}",
        connection_manager.get_connection_count().await
    );
    println!(
        "   唯一用户数: {}",
        connection_manager.get_unique_user_count().await
    );

    // 演示1: 全员广播消息
    println!("\n🔊 演示1: 全员广播消息");
    println!("{}", "-".repeat(30));

    let alice_user_info = UserInfo {
        user_id: user_connections[0].1,
        username: user_connections[0].2.clone(),
        connected_at: Some(Utc::now()),
    };

    let broadcast_message = ServerMessage::new_text(
        "大家好！这是一条全员广播消息！".to_string(),
        alice_user_info,
    );

    message_distributor
        .broadcast_to_all(
            broadcast_message,
            true,                        // 排除发送者
            Some(user_connections[0].0), // Alice的连接ID
            Some(MessagePriority::Normal),
        )
        .await?;

    // 等待消息处理
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    println!("✅ 全员广播消息已发送");

    // 演示2: 私聊消息
    println!("\n💬 演示2: 私聊消息");
    println!("{}", "-".repeat(30));

    let bob_user_info = UserInfo {
        user_id: user_connections[1].1,
        username: user_connections[1].2.clone(),
        connected_at: Some(Utc::now()),
    };

    let direct_message =
        ServerMessage::new_text("嗨 Charlie，这是一条私聊消息！".to_string(), bob_user_info);

    message_distributor
        .send_direct_message(
            direct_message,
            user_connections[2].1, // Charlie的用户ID
            Some(MessagePriority::High),
        )
        .await?;

    // 等待消息处理
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    println!("✅ 私聊消息已发送给 Charlie");

    // 演示3: 系统消息
    println!("\n📢 演示3: 系统消息");
    println!("{}", "-".repeat(30));

    let system_message =
        ServerMessage::new_system("系统维护通知：服务器将在10分钟后重启".to_string());

    message_distributor
        .send_system_message(system_message)
        .await?;

    // 等待消息处理
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    println!("✅ 系统消息已发送");

    // 演示4: 消息优先级
    println!("\n⚡ 演示4: 消息优先级处理");
    println!("{}", "-".repeat(30));

    // 发送不同优先级的消息
    let priorities = vec![
        (MessagePriority::Low, "低优先级消息"),
        (MessagePriority::Critical, "紧急消息！"),
        (MessagePriority::Normal, "普通消息"),
        (MessagePriority::High, "高优先级消息"),
    ];

    for (priority, content) in priorities {
        let message = ServerMessage::new_system(content.to_string());
        message_distributor
            .send_direct_message(
                message,
                user_connections[0].1, // 发送给Alice
                Some(priority),
            )
            .await?;
        println!("📤 已提交 {:?} 优先级消息: {}", priority, content);
    }

    // 等待消息处理
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    println!("✅ 优先级消息处理完成");

    // 演示5: 统计信息
    println!("\n📈 演示5: 分发统计信息");
    println!("{}", "-".repeat(30));

    let stats = message_distributor.get_stats().await;
    println!("📊 消息分发统计:");
    println!("   总任务数: {}", stats.total_tasks);
    println!("   成功分发: {}", stats.successful_distributions);
    println!("   失败分发: {}", stats.failed_distributions);
    println!("   当前队列长度: {}", stats.queue_length);
    println!("   最后更新时间: {}", stats.last_updated.format("%H:%M:%S"));

    // 演示6: 在线用户列表
    println!("\n👥 演示6: 在线用户列表");
    println!("{}", "-".repeat(30));

    let online_users = connection_manager.get_online_users().await;
    println!("📋 当前在线用户:");
    for user in online_users {
        println!(
            "   - {} (ID: {}, 连接数: {})",
            user.username, user.user_id, user.connection_count
        );
    }

    // 清理演示
    println!("\n🧹 清理资源");
    println!("{}", "-".repeat(30));

    // 移除所有连接
    for (connection_id, _, username, _) in user_connections {
        connection_manager.remove_connection(&connection_id).await;
        println!("👋 用户 {} 已断开连接", username);
    }

    // 清空消息队列
    let cleared_count = message_distributor.clear_queue().await;
    if cleared_count > 0 {
        println!("🗑️  已清空 {} 个待处理消息", cleared_count);
    }

    println!("\n🎉 消息分发器演示完成！");
    println!("{}", "=".repeat(50));

    Ok(())
}
