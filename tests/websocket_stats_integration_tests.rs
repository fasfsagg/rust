// ================================================================================================
// 【任务12.7集成测试】WebSocket连接监控和统计集成测试
// ================================================================================================

use axum_tutorial::startup::AppState;
use axum_tutorial::app::service::connection_manager::ConnectionManager;
use axum_tutorial::app::controller::performance_controller::{
    get_websocket_stats,
    get_websocket_connections,
    get_websocket_metrics,
};
use axum::{ extract::State, response::IntoResponse };
use serde_json::Value;
use tokio::sync::mpsc;
use uuid::Uuid;

/// 【任务12.7集成测试】测试WebSocket统计API端点
///
/// 【功能】: 测试 GET /api/websocket/stats 端点的功能
/// 【验证】:
/// - API响应格式正确
/// - 统计数据准确性
/// - 响应状态码为200
#[tokio::test]
async fn test_websocket_stats_api() {
    // 创建连接管理器和应用状态
    let connection_manager = ConnectionManager::new();
    let app_state = AppState::new_for_testing(connection_manager.clone()).await;

    // 添加一些测试连接
    let (sender1, _receiver1) = mpsc::unbounded_channel();
    let (sender2, _receiver2) = mpsc::unbounded_channel();

    let connection_id1 = Uuid::new_v4();
    let connection_id2 = Uuid::new_v4();
    let user_id1 = Uuid::new_v4();
    let user_id2 = Uuid::new_v4();

    app_state.connection_manager
        .add_connection(
            connection_id1,
            user_id1,
            "test_user1".to_string(),
            sender1,
            Some("192.168.1.1".to_string())
        ).await
        .unwrap();

    app_state.connection_manager
        .add_connection(
            connection_id2,
            user_id2,
            "test_user2".to_string(),
            sender2,
            Some("192.168.1.2".to_string())
        ).await
        .unwrap();

    // 记录一些消息统计
    app_state.connection_manager.record_message_sent(&connection_id1, 100).await.unwrap();
    app_state.connection_manager.record_message_received(&connection_id1, 150).await.unwrap();
    app_state.connection_manager.record_message_sent(&connection_id2, 200).await.unwrap();

    // 调用API处理函数
    let response = get_websocket_stats(State(app_state)).await;

    // 将响应转换为字节
    let response_bytes = axum::body
        ::to_bytes(response.into_response().into_body(), usize::MAX).await
        .unwrap();
    let response_text = String::from_utf8(response_bytes.to_vec()).unwrap();

    // 解析JSON响应
    let json_response: Value = serde_json::from_str(&response_text).unwrap();

    // 验证响应结构
    assert!(json_response.get("websocket_stats").is_some());
    assert!(json_response.get("connection_quality").is_some());
    assert!(json_response.get("message_throughput").is_some());
    assert!(json_response.get("timestamp").is_some());
    assert!(json_response.get("server_status").is_some());

    // 验证WebSocket统计数据
    let websocket_stats = &json_response["websocket_stats"];
    assert_eq!(websocket_stats["active_connections"], 2);
    assert_eq!(websocket_stats["total_connections"], 2);
    assert_eq!(websocket_stats["unique_users"], 2);
    assert_eq!(websocket_stats["total_messages_sent"], 2);
    assert_eq!(websocket_stats["total_messages_received"], 1);
    assert_eq!(websocket_stats["connection_success_rate"], 100.0);

    // 验证连接质量数据
    let connection_quality = &json_response["connection_quality"];
    assert!(connection_quality["stability_score"].as_f64().unwrap() >= 0.0);
    assert!(connection_quality["average_response_time"].as_f64().unwrap() > 0.0);
    assert!(connection_quality["error_rate"].as_f64().unwrap() >= 0.0);
    assert!(connection_quality["heartbeat_loss_rate"].as_f64().unwrap() >= 0.0);

    // 验证消息吞吐量数据
    let message_throughput = &json_response["message_throughput"];
    assert!(message_throughput["messages_per_second"].as_f64().unwrap() >= 0.0);
    assert!(message_throughput["messages_per_minute"].as_f64().unwrap() >= 0.0);
    assert_eq!(message_throughput["total_bytes_transferred"], 450); // 100 + 150 + 200
    assert_eq!(message_throughput["average_message_size"], 150.0); // 450 / 3

    // 验证服务器状态
    assert_eq!(json_response["server_status"], "healthy");

    println!("✅ WebSocket统计API测试通过");
}

/// 【任务12.7集成测试】测试WebSocket连接详细信息API端点
///
/// 【功能】: 测试 GET /api/websocket/connections 端点的功能
/// 【验证】:
/// - API响应格式正确
/// - 连接详细信息准确性
/// - 在线用户列表正确
#[tokio::test]
async fn test_websocket_connections_api() {
    // 创建连接管理器和应用状态
    let connection_manager = ConnectionManager::new();
    let app_state = AppState::new_for_testing(connection_manager.clone()).await;

    // 添加测试连接
    let (sender, _receiver) = mpsc::unbounded_channel();
    let connection_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    app_state.connection_manager
        .add_connection(
            connection_id,
            user_id,
            "test_user".to_string(),
            sender,
            Some("192.168.1.100".to_string())
        ).await
        .unwrap();

    // 调用API处理函数
    let response = get_websocket_connections(State(app_state)).await;

    // 将响应转换为字节
    let response_bytes = axum::body
        ::to_bytes(response.into_response().into_body(), usize::MAX).await
        .unwrap();
    let response_text = String::from_utf8(response_bytes.to_vec()).unwrap();

    // 解析JSON响应
    let json_response: Value = serde_json::from_str(&response_text).unwrap();

    // 验证响应结构
    assert!(json_response.get("connection_count").is_some());
    assert!(json_response.get("unique_user_count").is_some());
    assert!(json_response.get("online_users").is_some());
    assert!(json_response.get("timestamp").is_some());

    // 验证连接数据
    assert_eq!(json_response["connection_count"], 1);
    assert_eq!(json_response["unique_user_count"], 1);

    // 验证在线用户列表
    let online_users = json_response["online_users"].as_array().unwrap();
    assert_eq!(online_users.len(), 1);
    assert_eq!(online_users[0]["username"], "test_user");
    assert_eq!(online_users[0]["user_id"], user_id.to_string());

    println!("✅ WebSocket连接详细信息API测试通过");
}

/// 【任务12.7集成测试】测试WebSocket性能指标API端点
///
/// 【功能】: 测试 GET /api/websocket/metrics 端点的功能
/// 【验证】:
/// - API响应格式正确
/// - 性能指标数据准确性
/// - Prometheus风格的指标格式
#[tokio::test]
async fn test_websocket_metrics_api() {
    // 创建连接管理器和应用状态
    let connection_manager = ConnectionManager::new();
    let app_state = AppState::new_for_testing(connection_manager.clone()).await;

    // 添加测试连接并记录一些活动
    let (sender, _receiver) = mpsc::unbounded_channel();
    let connection_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    app_state.connection_manager
        .add_connection(connection_id, user_id, "metrics_test_user".to_string(), sender, None).await
        .unwrap();

    // 记录消息和重连
    app_state.connection_manager.record_message_sent(&connection_id, 512).await.unwrap();
    app_state.connection_manager.record_message_received(&connection_id, 256).await.unwrap();
    app_state.connection_manager.record_reconnection(&connection_id).await.unwrap();

    // 调用API处理函数
    let response = get_websocket_metrics(State(app_state)).await;

    // 将响应转换为字节
    let response_bytes = axum::body
        ::to_bytes(response.into_response().into_body(), usize::MAX).await
        .unwrap();
    let response_text = String::from_utf8(response_bytes.to_vec()).unwrap();

    // 解析JSON响应
    let json_response: Value = serde_json::from_str(&response_text).unwrap();

    // 验证Prometheus风格的指标
    assert_eq!(json_response["websocket_active_connections"], 1);
    assert_eq!(json_response["websocket_total_connections"], 1);
    assert_eq!(json_response["websocket_unique_users"], 1);
    assert_eq!(json_response["websocket_messages_sent_total"], 1);
    assert_eq!(json_response["websocket_messages_received_total"], 1);
    assert_eq!(json_response["websocket_reconnections_total"], 1);
    assert_eq!(json_response["websocket_connection_success_rate"], 100.0);
    assert_eq!(json_response["websocket_total_bytes_transferred"], 768); // 512 + 256
    assert_eq!(json_response["websocket_average_message_size_bytes"], 384.0); // 768 / 2

    // 验证稳定性评分（应该因为重连而降低）
    assert_eq!(json_response["websocket_stability_score"], 90.0); // 100 - 1*10

    // 验证时间戳存在
    assert!(json_response.get("timestamp").is_some());

    println!("✅ WebSocket性能指标API测试通过");
}

/// 【任务12.7集成测试】测试空连接状态下的API响应
///
/// 【功能】: 测试在没有WebSocket连接时API的响应
/// 【验证】:
/// - 空状态下的统计数据正确
/// - API不会崩溃或返回错误
#[tokio::test]
async fn test_websocket_stats_empty_state() {
    // 创建空的连接管理器和应用状态
    let connection_manager = ConnectionManager::new();
    let app_state = AppState::new_for_testing(connection_manager).await;

    // 调用API处理函数
    let response = get_websocket_stats(State(app_state)).await;

    // 将响应转换为字节
    let response_bytes = axum::body
        ::to_bytes(response.into_response().into_body(), usize::MAX).await
        .unwrap();
    let response_text = String::from_utf8(response_bytes.to_vec()).unwrap();

    // 解析JSON响应
    let json_response: Value = serde_json::from_str(&response_text).unwrap();

    // 验证空状态下的统计数据
    let websocket_stats = &json_response["websocket_stats"];
    assert_eq!(websocket_stats["active_connections"], 0);
    assert_eq!(websocket_stats["total_connections"], 0);
    assert_eq!(websocket_stats["unique_users"], 0);
    assert_eq!(websocket_stats["total_messages_sent"], 0);
    assert_eq!(websocket_stats["total_messages_received"], 0);
    assert_eq!(websocket_stats["reconnection_count"], 0);
    assert_eq!(websocket_stats["connection_success_rate"], 100.0); // 默认100%
    assert_eq!(websocket_stats["average_connection_duration"], 0.0);
    assert_eq!(websocket_stats["max_connection_duration"], 0.0);

    // 验证服务器状态
    assert_eq!(json_response["server_status"], "healthy");

    println!("✅ WebSocket空状态API测试通过");
}
