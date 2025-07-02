//! WebSocket连接池管理器测试
//!
//! 【任务14.10】WebSocket连接池和内存管理测试（阶段4）
//!
//! 本模块包含WebSocket连接池管理器的全面测试，包括：
//! 1. 内存泄漏检测测试
//! 2. 连接池大小限制和清理机制测试
//! 3. 性能监控模块测试
//! 4. 百万连接场景下的资源管理效率测试

use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;
use axum::extract::ws::Message;

// 导入项目模块
use axum_tutorial::config::WebSocketPoolConfig;
use axum_tutorial::app::utils::websocket_pool_manager::{
    WebSocketPoolManager,
    LoadBalancingStrategy,
    MemoryLeakReport,
    ConnectionPoolStats,
};

/// 创建测试用的WebSocket连接池配置
fn create_test_config() -> WebSocketPoolConfig {
    WebSocketPoolConfig {
        max_connections: 1000,
        pool_size: 100,
        heartbeat_interval: Duration::from_secs(30),
        connection_timeout: Duration::from_secs(30),
        max_reconnect_attempts: 5,
        reconnect_interval: Duration::from_secs(2),
        enable_load_balancing: true,
        enable_failover: true,
    }
}

/// 创建测试用的消息发送通道
fn create_test_sender() -> (mpsc::UnboundedSender<Message>, mpsc::UnboundedReceiver<Message>) {
    mpsc::unbounded_channel()
}

#[tokio::test]
async fn test_websocket_pool_manager_creation() {
    // 测试WebSocket连接池管理器的创建
    let config = create_test_config();
    let manager = WebSocketPoolManager::new(config);

    let metrics = manager.get_metrics();
    assert_eq!(metrics.total_connections.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert_eq!(metrics.active_connections.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert!(metrics.is_healthy());
}

#[tokio::test]
async fn test_add_and_remove_connections() {
    // 测试添加和移除连接
    let config = create_test_config();
    let manager = WebSocketPoolManager::new(config);

    let connection_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let (sender, _receiver) = create_test_sender();

    // 添加连接
    let result = manager.add_connection(connection_id, user_id, sender).await;
    assert!(result.is_ok());

    let metrics = manager.get_metrics();
    assert_eq!(metrics.active_connections.load(std::sync::atomic::Ordering::Relaxed), 1);

    // 移除连接
    let removed = manager.remove_connection(&connection_id).await;
    assert!(removed.is_some());

    let metrics = manager.get_metrics();
    assert_eq!(metrics.active_connections.load(std::sync::atomic::Ordering::Relaxed), 0);
}

#[tokio::test]
async fn test_connection_limit() {
    // 测试连接数限制
    let mut config = create_test_config();
    config.max_connections = 2; // 设置最大连接数为2

    let manager = WebSocketPoolManager::new(config);

    // 添加第一个连接
    let connection_id1 = Uuid::new_v4();
    let user_id1 = Uuid::new_v4();
    let (sender1, _receiver1) = create_test_sender();
    let result1 = manager.add_connection(connection_id1, user_id1, sender1).await;
    assert!(result1.is_ok());

    // 添加第二个连接
    let connection_id2 = Uuid::new_v4();
    let user_id2 = Uuid::new_v4();
    let (sender2, _receiver2) = create_test_sender();
    let result2 = manager.add_connection(connection_id2, user_id2, sender2).await;
    assert!(result2.is_ok());

    // 尝试添加第三个连接（应该失败）
    let connection_id3 = Uuid::new_v4();
    let user_id3 = Uuid::new_v4();
    let (sender3, _receiver3) = create_test_sender();
    let result3 = manager.add_connection(connection_id3, user_id3, sender3).await;
    assert!(result3.is_err());
    assert!(result3.unwrap_err().contains("连接池已满"));
}

#[tokio::test]
async fn test_memory_leak_detection() {
    // 测试内存泄漏检测
    let config = create_test_config();
    let manager = WebSocketPoolManager::new(config);

    // 添加一些连接
    for _i in 0..5 {
        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let (sender, _receiver) = create_test_sender();
        let _ = manager.add_connection(connection_id, user_id, sender).await;
    }

    // 执行内存泄漏检测
    let report = manager.detect_memory_leaks().await;
    assert!(report.is_ok());

    let leak_report = report.unwrap();
    assert_eq!(leak_report.total_connections, 5);
    assert_eq!(leak_report.active_connections, 5);
    assert_eq!(leak_report.stale_connections, 0);
    assert_eq!(leak_report.zombie_connections, 0);
    assert!(!leak_report.has_potential_leak);
    assert!(leak_report.memory_efficiency > 90.0);
}

#[tokio::test]
async fn test_force_cleanup_all() {
    // 测试强制清理所有连接
    let config = create_test_config();
    let manager = WebSocketPoolManager::new(config);

    // 添加一些连接
    for _i in 0..3 {
        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let (sender, _receiver) = create_test_sender();
        let _ = manager.add_connection(connection_id, user_id, sender).await;
    }

    let metrics_before = manager.get_metrics();
    assert_eq!(metrics_before.active_connections.load(std::sync::atomic::Ordering::Relaxed), 3);

    // 强制清理所有连接
    let removed_count = manager.force_cleanup_all().await;
    assert_eq!(removed_count, 3);

    let metrics_after = manager.get_metrics();
    assert_eq!(metrics_after.active_connections.load(std::sync::atomic::Ordering::Relaxed), 0);
}

#[tokio::test]
async fn test_detailed_stats() {
    // 测试获取详细统计信息
    let config = create_test_config();
    let manager = WebSocketPoolManager::new(config);

    // 添加一些连接
    let mut connection_ids = Vec::new();
    for _i in 0..3 {
        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let (sender, _receiver) = create_test_sender();
        let _ = manager.add_connection(connection_id, user_id, sender).await;
        connection_ids.push(connection_id);
    }

    // 获取详细统计
    let stats = manager.get_detailed_stats().await;
    assert_eq!(stats.total_connections, 3);
    assert_eq!(stats.active_connections, 3);
    assert_eq!(stats.idle_connections, 0);
    assert_eq!(stats.total_users, 3);
    assert!(stats.is_healthy);
}

#[tokio::test]
async fn test_health_check() {
    // 测试健康检查
    let config = create_test_config();
    let manager = WebSocketPoolManager::new(config);

    // 空连接池的健康检查
    let health_result = manager.health_check().await;
    assert!(health_result.is_ok());
    assert!(health_result.unwrap());

    // 添加连接后的健康检查
    let connection_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let (sender, _receiver) = create_test_sender();
    let _ = manager.add_connection(connection_id, user_id, sender).await;

    let health_result = manager.health_check().await;
    assert!(health_result.is_ok());
    assert!(health_result.unwrap());
}

#[tokio::test]
async fn test_user_connections() {
    // 测试用户连接管理
    let config = create_test_config();
    let manager = WebSocketPoolManager::new(config);

    let user_id = Uuid::new_v4();

    // 为同一用户添加多个连接
    let connection_id1 = Uuid::new_v4();
    let (sender1, _receiver1) = create_test_sender();
    let _ = manager.add_connection(connection_id1, user_id, sender1).await;

    let connection_id2 = Uuid::new_v4();
    let (sender2, _receiver2) = create_test_sender();
    let _ = manager.add_connection(connection_id2, user_id, sender2).await;

    // 获取用户的所有连接
    let user_connections = manager.get_user_connections(&user_id).await;
    assert_eq!(user_connections.len(), 2);
    assert!(user_connections.contains(&connection_id1));
    assert!(user_connections.contains(&connection_id2));

    // 移除一个连接
    let _ = manager.remove_connection(&connection_id1).await;
    let user_connections = manager.get_user_connections(&user_id).await;
    assert_eq!(user_connections.len(), 1);
    assert!(user_connections.contains(&connection_id2));
}

#[tokio::test]
async fn test_send_to_connection() {
    // 测试向指定连接发送消息
    let config = create_test_config();
    let manager = WebSocketPoolManager::new(config);

    let connection_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let (sender, _receiver) = create_test_sender();

    // 添加连接
    let _ = manager.add_connection(connection_id, user_id, sender).await;

    // 发送消息
    let message = Message::Text("Hello, WebSocket!".into());
    let result = manager.send_to_connection(&connection_id, message).await;
    assert!(result.is_ok());

    // 尝试向不存在的连接发送消息
    let non_existent_id = Uuid::new_v4();
    let message = Message::Text("Hello, WebSocket!".into());
    let result = manager.send_to_connection(&non_existent_id, message).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("不存在"));
}

#[tokio::test]
async fn test_broadcast_to_user() {
    // 测试向用户的所有连接广播消息
    let config = create_test_config();
    let manager = WebSocketPoolManager::new(config);

    let user_id = Uuid::new_v4();

    // 为用户添加多个连接
    let connection_id1 = Uuid::new_v4();
    let (sender1, _receiver1) = create_test_sender();
    let _ = manager.add_connection(connection_id1, user_id, sender1).await;

    let connection_id2 = Uuid::new_v4();
    let (sender2, _receiver2) = create_test_sender();
    let _ = manager.add_connection(connection_id2, user_id, sender2).await;

    // 广播消息
    let message = Message::Text("Broadcast message".into());
    let result = manager.broadcast_to_user(&user_id, message).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 2); // 成功发送到2个连接

    // 向不存在的用户广播
    let non_existent_user = Uuid::new_v4();
    let message = Message::Text("Broadcast message".into());
    let result = manager.broadcast_to_user(&non_existent_user, message).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0); // 没有连接
}

#[tokio::test]
async fn test_memory_monitoring_start_stop() {
    // 测试内存监控的启动和停止
    let config = create_test_config();
    let mut manager = WebSocketPoolManager::new(config);

    // 启动内存监控
    let result = manager.start_memory_monitoring().await;
    assert!(result.is_ok());

    // 尝试重复启动（应该失败）
    let result = manager.start_memory_monitoring().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("已在运行"));

    // 停止内存监控
    manager.stop_memory_monitoring().await;

    // 再次启动应该成功
    let result = manager.start_memory_monitoring().await;
    assert!(result.is_ok());

    // 清理
    manager.stop_memory_monitoring().await;
}

#[tokio::test]
async fn test_high_concurrency_connections() {
    // 测试高并发连接场景
    let mut config = create_test_config();
    config.max_connections = 1000; // 设置较高的连接数限制

    let manager = std::sync::Arc::new(WebSocketPoolManager::new(config));

    // 并发添加多个连接
    let mut handles = Vec::new();
    for _i in 0..100 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            let connection_id = Uuid::new_v4();
            let user_id = Uuid::new_v4();
            let (sender, _receiver) = create_test_sender();
            manager_clone.add_connection(connection_id, user_id, sender).await
        });
        handles.push(handle);
    }

    // 等待所有连接完成
    let mut success_count = 0;
    for handle in handles {
        let result = handle.await.unwrap();
        if result.is_ok() {
            success_count += 1;
        }
    }

    assert_eq!(success_count, 100);

    let metrics = manager.get_metrics();
    assert_eq!(metrics.active_connections.load(std::sync::atomic::Ordering::Relaxed), 100);
}

#[tokio::test]
async fn test_memory_efficiency_calculation() {
    // 测试内存效率计算
    let config = create_test_config();
    let manager = WebSocketPoolManager::new(config);

    // 添加一些活跃连接
    for _i in 0..8 {
        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let (sender, _receiver) = create_test_sender();
        let _ = manager.add_connection(connection_id, user_id, sender).await;
    }

    // 执行内存泄漏检测
    let report = manager.detect_memory_leaks().await.unwrap();

    // 验证内存效率计算
    assert_eq!(report.total_connections, 8);
    assert_eq!(report.active_connections, 8);
    assert_eq!(report.memory_efficiency, 100.0); // 所有连接都活跃
    assert!(!report.has_potential_leak);
}

#[tokio::test]
async fn test_connection_pool_utilization() {
    // 测试连接池利用率计算
    let config = create_test_config();
    let manager = WebSocketPoolManager::new(config);

    let metrics = manager.get_metrics();

    // 初始利用率应该为0
    assert_eq!(metrics.get_utilization_percentage(), 0.0);

    // 添加一些连接
    for _i in 0..5 {
        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let (sender, _receiver) = create_test_sender();
        let _ = manager.add_connection(connection_id, user_id, sender).await;
    }

    // 检查利用率
    let utilization = metrics.get_utilization_percentage();
    assert!(utilization > 0.0);
    assert!(utilization <= 100.0);
}

#[tokio::test]
async fn test_connection_metrics_tracking() {
    // 测试连接指标跟踪
    let config = create_test_config();
    let manager = WebSocketPoolManager::new(config);

    let metrics = manager.get_metrics();

    // 初始指标
    assert_eq!(metrics.total_connections.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert_eq!(metrics.active_connections.load(std::sync::atomic::Ordering::Relaxed), 0);

    // 添加连接
    let connection_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let (sender, _receiver) = create_test_sender();
    let _ = manager.add_connection(connection_id, user_id, sender).await;

    // 验证指标更新
    assert_eq!(metrics.total_connections.load(std::sync::atomic::Ordering::Relaxed), 1);
    assert_eq!(metrics.active_connections.load(std::sync::atomic::Ordering::Relaxed), 1);

    // 移除连接
    let _ = manager.remove_connection(&connection_id).await;

    // 验证指标更新
    assert_eq!(metrics.active_connections.load(std::sync::atomic::Ordering::Relaxed), 0);
}
