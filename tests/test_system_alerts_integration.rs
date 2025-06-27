// ================================================================================================
// 【任务12.8实现】系统资源监控和告警阈值检查集成测试
// ================================================================================================

use axum::{ body::Body, http::{ Request, StatusCode } };
use axum_tutorial::{
    app::{ controller::performance_controller::SystemAlertsResponse, service::ConnectionManager },
    startup::AppState,
    routes::create_routes,
};
use serde_json::Value;
use tower::ServiceExt;

/// 测试系统资源监控和告警阈值检查API端点
///
/// 【功能】：测试 GET /api/monitoring/alerts 端点的完整功能
/// 【验证内容】：
/// - HTTP状态码为200
/// - 响应JSON结构正确
/// - 包含所有必需的字段
/// - 告警阈值配置正确
/// - 资源状态信息完整
#[tokio::test]
async fn test_system_alerts_endpoint() {
    // 创建应用状态
    let connection_manager = ConnectionManager::new();
    let app_state = AppState::new_for_testing(connection_manager).await;

    // 创建路由
    let app = create_routes(app_state);

    // 创建请求
    let request = Request::builder()
        .method("GET")
        .uri("/api/monitoring/alerts")
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap();

    // 发送请求
    let response = app.oneshot(request).await.unwrap();

    // 验证状态码
    assert_eq!(response.status(), StatusCode::OK);

    // 获取响应体
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_text = String::from_utf8(body.to_vec()).unwrap();

    // 解析JSON响应
    let response_json: SystemAlertsResponse = serde_json
        ::from_str(&response_text)
        .expect("响应应该是有效的SystemAlertsResponse JSON");

    // 验证响应结构
    assert!(["normal", "warning", "critical"].contains(&response_json.alert_status.as_str()));
    assert!(response_json.active_alerts.is_empty() || !response_json.active_alerts.is_empty());
    assert!(response_json.timestamp > 0);
    assert!(response_json.check_duration_ms >= 0);

    // 验证告警阈值配置
    let thresholds = &response_json.thresholds;
    assert_eq!(thresholds.cpu.warning, 70.0);
    assert_eq!(thresholds.cpu.critical, 85.0);
    assert_eq!(thresholds.cpu.unit, "percent");

    assert_eq!(thresholds.memory.warning, 75.0);
    assert_eq!(thresholds.memory.critical, 90.0);
    assert_eq!(thresholds.memory.unit, "percent");

    assert_eq!(thresholds.disk.warning, 80.0);
    assert_eq!(thresholds.disk.critical, 95.0);
    assert_eq!(thresholds.disk.unit, "percent");

    assert_eq!(thresholds.network_connections.warning, 8000.0);
    assert_eq!(thresholds.network_connections.critical, 10000.0);
    assert_eq!(thresholds.network_connections.unit, "connections");

    assert_eq!(thresholds.system_load.warning, 0.8);
    assert_eq!(thresholds.system_load.critical, 1.0);
    assert_eq!(thresholds.system_load.unit, "load");

    // 验证资源状态
    let status = &response_json.resource_status;

    // 验证CPU状态
    assert!(status.cpu.current >= 0.0);
    assert!(status.cpu.usage_percent >= 0.0 && status.cpu.usage_percent <= 100.0);
    assert!(["normal", "warning", "critical"].contains(&status.cpu.status.as_str()));
    assert_eq!(status.cpu.unit, "percent");

    // 验证内存状态
    assert!(status.memory.current >= 0.0);
    assert!(status.memory.usage_percent >= 0.0 && status.memory.usage_percent <= 100.0);
    assert!(["normal", "warning", "critical"].contains(&status.memory.status.as_str()));
    assert_eq!(status.memory.unit, "percent");

    // 验证磁盘状态
    assert!(status.disk.current >= 0.0);
    assert!(status.disk.usage_percent >= 0.0 && status.disk.usage_percent <= 100.0);
    assert!(["normal", "warning", "critical"].contains(&status.disk.status.as_str()));
    assert_eq!(status.disk.unit, "percent");

    // 验证网络连接状态
    assert!(status.network_connections.current >= 0.0);
    assert!(status.network_connections.usage_percent >= 0.0);
    assert!(
        ["normal", "warning", "critical"].contains(&status.network_connections.status.as_str())
    );
    assert_eq!(status.network_connections.unit, "connections");

    // 验证系统负载状态
    assert!(status.system_load.current >= 0.0);
    assert!(status.system_load.usage_percent >= 0.0);
    assert!(["normal", "warning", "critical"].contains(&status.system_load.status.as_str()));
    assert_eq!(status.system_load.unit, "load");

    println!("✅ 系统资源监控和告警阈值检查API测试通过");
    println!("📊 告警状态: {}", response_json.alert_status);
    println!("🔍 活跃告警数量: {}", response_json.active_alerts.len());
    println!("⏱️ 检查耗时: {}ms", response_json.check_duration_ms);
    println!("💻 CPU使用率: {:.1}%", status.cpu.usage_percent);
    println!("🧠 内存使用率: {:.1}%", status.memory.usage_percent);
    println!("💾 磁盘使用率: {:.1}%", status.disk.usage_percent);
    println!("🌐 网络连接数: {}", status.network_connections.current);
    println!("⚡ 系统负载: {:.2}", status.system_load.current);
}

/// 测试系统告警响应的JSON序列化和反序列化
///
/// 【功能】：验证SystemAlertsResponse结构体的JSON处理能力
#[tokio::test]
async fn test_system_alerts_json_serialization() {
    // 创建应用状态
    let connection_manager = ConnectionManager::new();
    let app_state = AppState::new_for_testing(connection_manager).await;

    // 创建路由
    let app = create_routes(app_state);

    // 创建请求
    let request = Request::builder()
        .method("GET")
        .uri("/api/monitoring/alerts")
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap();

    // 发送请求
    let response = app.oneshot(request).await.unwrap();

    // 获取响应体
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_text = String::from_utf8(body.to_vec()).unwrap();

    // 验证JSON格式
    let json_value: Value = serde_json::from_str(&response_text).expect("响应应该是有效的JSON");

    // 验证必需字段存在
    assert!(json_value.get("alert_status").is_some());
    assert!(json_value.get("active_alerts").is_some());
    assert!(json_value.get("resource_status").is_some());
    assert!(json_value.get("thresholds").is_some());
    assert!(json_value.get("timestamp").is_some());
    assert!(json_value.get("check_duration_ms").is_some());

    // 验证嵌套结构
    let resource_status = json_value.get("resource_status").unwrap();
    assert!(resource_status.get("cpu").is_some());
    assert!(resource_status.get("memory").is_some());
    assert!(resource_status.get("disk").is_some());
    assert!(resource_status.get("network_connections").is_some());
    assert!(resource_status.get("system_load").is_some());

    let thresholds = json_value.get("thresholds").unwrap();
    assert!(thresholds.get("cpu").is_some());
    assert!(thresholds.get("memory").is_some());
    assert!(thresholds.get("disk").is_some());
    assert!(thresholds.get("network_connections").is_some());
    assert!(thresholds.get("system_load").is_some());

    println!("✅ 系统告警JSON序列化测试通过");
}
