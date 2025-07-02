//! 连接池管理器功能验证示例
//!
//! 【任务13.4】连接池优化 - 功能验证
//!
//! 本示例演示和验证数据库连接池管理器和WebSocket连接池管理器的功能

use axum::extract::ws::Message;
use axum_tutorial::app::utils::{DatabasePoolManager, WebSocketPoolManager};
use axum_tutorial::config::{AppConfig, DatabasePoolConfig, WebSocketPoolConfig};
use tokio::sync::mpsc;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    println!("🚀 开始验证连接池管理器功能...");

    // 创建测试配置
    let config = AppConfig {
        http_addr: "127.0.0.1:3000".parse().unwrap(),
        database_url: "sqlite::memory:".to_string(),
        jwt_secret: "test_secret".to_string(),
        database_pool: DatabasePoolConfig::development(),
        websocket_pool: WebSocketPoolConfig::development(),
    };

    // 测试数据库连接池管理器
    println!("\n📊 测试数据库连接池管理器...");
    test_database_pool_manager(&config).await?;

    // 测试WebSocket连接池管理器
    println!("\n🔌 测试WebSocket连接池管理器...");
    test_websocket_pool_manager(&config).await?;

    println!("\n✅ 所有连接池管理器功能验证完成！");
    Ok(())
}

/// 测试数据库连接池管理器
async fn test_database_pool_manager(config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("  创建数据库连接池管理器...");
    let pool_manager = DatabasePoolManager::new(config).await?;

    println!("  验证连接池初始状态...");
    let metrics = pool_manager.get_metrics();
    println!(
        "    - 总连接数: {}",
        metrics
            .total_connections
            .load(std::sync::atomic::Ordering::Relaxed)
    );
    println!(
        "    - 活跃连接数: {}",
        metrics
            .active_connections
            .load(std::sync::atomic::Ordering::Relaxed)
    );
    println!("    - 健康状态: {}", metrics.is_healthy());

    println!("  测试连接获取...");
    let connection = pool_manager.get_connection();

    println!("  验证数据库连接可用性...");
    match connection.ping().await {
        Ok(_) => println!("    ✅ 数据库连接可用"),
        Err(e) => println!("    ❌ 数据库连接失败: {}", e),
    }

    println!("  执行健康检查...");
    match pool_manager.health_check().await {
        Ok(true) => println!("    ✅ 连接池健康检查通过"),
        Ok(false) => println!("    ⚠️ 连接池健康检查失败"),
        Err(e) => println!("    ❌ 健康检查错误: {}", e),
    }

    println!("  验证指标更新...");
    let updated_metrics = pool_manager.get_metrics();
    println!(
        "    - 总获取次数: {}",
        updated_metrics
            .total_acquires
            .load(std::sync::atomic::Ordering::Relaxed)
    );
    println!(
        "    - 成功获取次数: {}",
        updated_metrics
            .successful_acquires
            .load(std::sync::atomic::Ordering::Relaxed)
    );
    println!("    - 成功率: {:.1}%", updated_metrics.get_success_rate());

    Ok(())
}

/// 测试WebSocket连接池管理器
async fn test_websocket_pool_manager(config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("  创建WebSocket连接池管理器...");
    let pool_manager = WebSocketPoolManager::new(config.websocket_pool.clone());

    println!("  验证连接池初始状态...");
    let metrics = pool_manager.get_metrics();
    println!(
        "    - 最大连接数: {}",
        config.websocket_pool.max_connections
    );
    println!(
        "    - 当前活跃连接数: {}",
        metrics
            .active_connections
            .load(std::sync::atomic::Ordering::Relaxed)
    );
    println!("    - 健康状态: {}", metrics.is_healthy());

    println!("  测试连接添加...");
    let connection_id1 = Uuid::new_v4();
    let connection_id2 = Uuid::new_v4();
    let user_id1 = Uuid::new_v4();
    let user_id2 = Uuid::new_v4();

    let (sender1, mut receiver1) = mpsc::unbounded_channel();
    let (sender2, mut receiver2) = mpsc::unbounded_channel();

    // 添加第一个连接
    match pool_manager
        .add_connection(connection_id1, user_id1, sender1)
        .await
    {
        Ok(_) => println!("    ✅ 连接1添加成功"),
        Err(e) => println!("    ❌ 连接1添加失败: {}", e),
    }

    // 添加第二个连接
    match pool_manager
        .add_connection(connection_id2, user_id2, sender2)
        .await
    {
        Ok(_) => println!("    ✅ 连接2添加成功"),
        Err(e) => println!("    ❌ 连接2添加失败: {}", e),
    }

    println!("  验证连接池状态更新...");
    let updated_metrics = pool_manager.get_metrics();
    println!(
        "    - 活跃连接数: {}",
        updated_metrics
            .active_connections
            .load(std::sync::atomic::Ordering::Relaxed)
    );
    println!(
        "    - 连接池利用率: {:.1}%",
        updated_metrics.get_utilization_percentage()
    );

    println!("  测试消息发送...");
    let test_message = Message::Text("Hello from connection pool test!".to_string().into());

    match pool_manager
        .send_to_connection(&connection_id1, test_message.clone())
        .await
    {
        Ok(_) => {
            println!("    ✅ 消息发送到连接1成功");
            // 验证消息接收
            if let Ok(received) = receiver1.try_recv() {
                println!("    ✅ 连接1成功接收消息");
            }
        }
        Err(e) => println!("    ❌ 消息发送失败: {}", e),
    }

    println!("  测试用户广播...");
    // 为同一用户添加第二个连接（模拟多设备）
    let connection_id3 = Uuid::new_v4();
    let (sender3, mut receiver3) = mpsc::unbounded_channel();

    match pool_manager
        .add_connection(connection_id3, user_id1, sender3)
        .await
    {
        Ok(_) => println!("    ✅ 用户多设备连接添加成功"),
        Err(e) => println!("    ❌ 多设备连接添加失败: {}", e),
    }

    let broadcast_message = Message::Text("Broadcast to all user devices!".to_string().into());
    match pool_manager
        .broadcast_to_user(&user_id1, broadcast_message)
        .await
    {
        Ok(count) => {
            println!("    ✅ 广播成功，发送到{}个连接", count);
            // 验证广播接收
            if let Ok(_) = receiver3.try_recv() {
                println!("    ✅ 多设备连接成功接收广播消息");
            }
        }
        Err(e) => println!("    ❌ 广播失败: {}", e),
    }

    println!("  测试连接移除...");
    match pool_manager.remove_connection(&connection_id1).await {
        Some(_) => println!("    ✅ 连接1移除成功"),
        None => println!("    ❌ 连接1移除失败"),
    }

    println!("  执行健康检查...");
    match pool_manager.health_check().await {
        Ok(true) => println!("    ✅ WebSocket连接池健康检查通过"),
        Ok(false) => println!("    ⚠️ WebSocket连接池健康检查失败"),
        Err(e) => println!("    ❌ 健康检查错误: {}", e),
    }

    println!("  验证最终指标...");
    let final_metrics = pool_manager.get_metrics();
    println!(
        "    - 最终活跃连接数: {}",
        final_metrics
            .active_connections
            .load(std::sync::atomic::Ordering::Relaxed)
    );
    println!(
        "    - 总消息发送数: {}",
        final_metrics
            .total_messages_sent
            .load(std::sync::atomic::Ordering::Relaxed)
    );
    println!(
        "    - 连接失败次数: {}",
        final_metrics
            .connection_failures
            .load(std::sync::atomic::Ordering::Relaxed)
    );

    Ok(())
}
