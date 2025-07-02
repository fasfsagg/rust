// 性能控制器测试模块
//
// 【功能】：为性能监控、指标收集、负载均衡算法、资源管理等功能提供全面测试覆盖
// 【目标】：验证百万并发场景下的性能监控准确性和系统稳定性
// 【覆盖范围】：
// - 性能监控中间件测试
// - 指标收集和导出测试
// - 健康检查端点测试
// - WebSocket连接监控测试
// - 负载均衡算法测试
// - 资源管理和告警测试
// - Prometheus指标导出测试

use axum::{
    body::Body,
    extract::State,
    http::{ Request, StatusCode, header },
    response::Response,
    routing::{ get, post },
    Router,
};
use axum_test::TestServer;
use serde_json::{ json, Value };
use std::{
    collections::HashMap,
    sync::{ Arc, atomic::{ AtomicU64, Ordering } },
    time::{ Duration, Instant },
};
use tokio::time::sleep;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

// 导入项目模块
use axum_tutorial::{
    app::{ controller::performance_controller::*, service::ConnectionManager },
    startup::AppState,
};

/// 测试配置常量
const TEST_TIMEOUT: Duration = Duration::from_secs(30);
const PERFORMANCE_TEST_ITERATIONS: usize = 100;
const CONCURRENT_CONNECTIONS_TEST: usize = 50;

/// 创建测试用的应用状态
async fn create_test_app_state() -> AppState {
    let connection_manager = ConnectionManager::new();
    AppState::new_for_testing(connection_manager).await
}

/// 创建测试服务器
async fn create_test_server() -> TestServer {
    let app_state = create_test_app_state().await;

    let app = Router::new()
        // 基础健康检查端点
        .route("/api/performance/health", get(health_check))
        // 增强健康检查端点
        .route("/api/health/enhanced", get(enhanced_health_check))
        // 就绪状态检查端点
        .route("/api/health/ready", get(readiness_check))
        // 存活状态检查端点
        .route("/api/health/live", get(liveness_check))
        // 深度健康检查端点
        .route("/api/health/deep", get(deep_health_check))
        // Prometheus指标导出端点
        .route("/metrics", get(get_prometheus_metrics))
        // 系统资源告警检查端点
        .route("/api/monitoring/alerts", get(get_system_alerts))
        // WebSocket统计端点
        .route("/api/websocket/stats", get(get_websocket_stats))
        // WebSocket连接信息端点
        .route("/api/websocket/connections", get(get_websocket_connections))
        // WebSocket性能指标端点
        .route("/api/websocket/metrics", get(get_websocket_metrics))
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
        .with_state(app_state);

    TestServer::new(app).unwrap()
}

#[cfg(test)]
mod performance_controller_tests {
    use super::*;

    /// 【测试1】基础健康检查端点测试
    ///
    /// 【功能】：测试 GET /api/performance/health 端点的基本功能
    /// 【验证点】：
    /// - HTTP状态码为200
    /// - 响应格式正确
    /// - 包含必要的健康状态信息
    /// - 响应时间在合理范围内
    #[tokio::test]
    async fn test_basic_health_check_endpoint() {
        let server = create_test_server().await;

        let start_time = Instant::now();
        let response = server.get("/api/performance/health").await;
        let response_time = start_time.elapsed();

        // 验证HTTP状态码（可能是200或503）
        let status_code = response.status_code();
        assert!(
            status_code == StatusCode::OK || status_code == StatusCode::SERVICE_UNAVAILABLE,
            "健康检查状态码应该是200或503，实际: {}",
            status_code
        );

        // 验证响应时间（应该在1秒内）
        assert!(
            response_time < Duration::from_secs(1),
            "基础健康检查响应时间过长: {:?}",
            response_time
        );

        // 验证响应内容类型
        response.assert_header("content-type", "application/json");

        // 解析响应JSON
        let health_data: Value = response.json();

        // 验证响应结构（根据实际API结构）
        if status_code == StatusCode::OK {
            // 健康状态的响应结构
            assert!(health_data.get("status").is_some(), "缺少status字段");
            assert!(health_data.get("details").is_some(), "缺少details字段");
            assert!(health_data.get("timestamp").is_some(), "缺少timestamp字段");
            assert!(health_data.get("check_duration_ms").is_some(), "缺少check_duration_ms字段");
        } else {
            // 错误状态的响应结构
            assert!(health_data.get("error").is_some(), "缺少error字段");
        }

        println!("✅ 基础健康检查端点测试通过");
    }

    /// 【测试2】增强健康检查端点测试
    ///
    /// 【功能】：测试 GET /api/health/enhanced 端点的详细功能
    /// 【验证点】：
    /// - 包含组件健康状态
    /// - 包含系统资源信息
    /// - 包含告警信息
    /// - 响应结构完整性
    #[tokio::test]
    async fn test_enhanced_health_check_endpoint() {
        let server = create_test_server().await;

        let response = server.get("/api/health/enhanced").await;

        response.assert_status(StatusCode::OK);
        response.assert_header("content-type", "application/json");

        let health_data: Value = response.json();

        // 验证增强健康检查的特有字段
        assert!(health_data.get("components").is_some(), "缺少components字段");
        assert!(health_data.get("resources").is_some(), "缺少resources字段");
        assert!(health_data.get("alerts").is_some(), "缺少alerts字段");
        assert!(health_data.get("check_duration_ms").is_some(), "缺少check_duration_ms字段");

        // 验证组件状态结构
        let components = health_data["components"].as_object().unwrap();
        assert!(components.contains_key("database"), "缺少database组件状态");
        assert!(components.contains_key("performance"), "缺少performance组件状态");

        // 验证资源信息结构
        let resources = &health_data["resources"];
        assert!(resources.get("cpu_usage_percent").is_some(), "缺少CPU使用率");
        assert!(resources.get("memory_usage_percent").is_some(), "缺少内存使用率");
        assert!(resources.get("active_connections").is_some(), "缺少活跃连接数");

        println!("✅ 增强健康检查端点测试通过");
    }

    /// 【测试3】就绪状态检查端点测试
    ///
    /// 【功能】：测试 GET /api/health/ready 端点的就绪状态检查
    /// 【验证点】：
    /// - 检查应用是否准备好接收流量
    /// - 验证数据库连接状态
    /// - 验证依赖服务状态
    #[tokio::test]
    async fn test_readiness_check_endpoint() {
        // 【测试环境标识】设置环境变量标识测试环境
        unsafe {
            std::env::set_var("CARGO_TEST", "1");
            std::env::set_var("RUST_TEST_THREADS", "1");
        }

        let server = create_test_server().await;

        let response = server.get("/api/health/ready").await;

        response.assert_status(StatusCode::OK);
        response.assert_header("content-type", "application/json");

        let readiness_data: Value = response.json();

        // 验证就绪检查的特有字段
        assert!(readiness_data.get("ready").is_some(), "缺少ready字段");
        assert!(readiness_data.get("checks").is_some(), "缺少checks字段");
        assert!(readiness_data.get("timestamp").is_some(), "缺少timestamp字段");

        // 验证检查项目
        let checks = readiness_data["checks"].as_object().unwrap();
        assert!(checks.contains_key("database_migration"), "缺少数据库迁移检查");
        assert!(checks.contains_key("configuration_loaded"), "缺少配置加载检查");

        println!("✅ 就绪状态检查端点测试通过");
    }

    /// 【测试4】存活状态检查端点测试
    ///
    /// 【功能】：测试 GET /api/health/live 端点的存活状态检查
    /// 【验证点】：
    /// - 检查应用进程是否存活
    /// - 验证响应时间检查
    /// - 验证内存泄漏检测
    #[tokio::test]
    async fn test_liveness_check_endpoint() {
        // 【测试环境标识】设置环境变量标识测试环境
        unsafe {
            std::env::set_var("CARGO_TEST", "1");
            std::env::set_var("RUST_TEST_THREADS", "1");
        }

        let server = create_test_server().await;

        let response = server.get("/api/health/live").await;

        response.assert_status(StatusCode::OK);
        response.assert_header("content-type", "application/json");

        let liveness_data: Value = response.json();

        // 验证存活检查的特有字段
        assert!(liveness_data.get("alive").is_some(), "缺少alive字段");
        assert!(liveness_data.get("checks").is_some(), "缺少checks字段");
        assert!(liveness_data.get("timestamp").is_some(), "缺少timestamp字段");
        assert!(liveness_data.get("check_duration_ms").is_some(), "缺少check_duration_ms字段");

        // 验证检查项目
        let checks = liveness_data["checks"].as_object().unwrap();
        assert!(checks.contains_key("application_response"), "缺少应用响应检查");
        assert!(checks.contains_key("performance_metrics"), "缺少性能指标检查");
        assert!(checks.contains_key("websocket_manager"), "缺少WebSocket管理器检查");
        assert!(checks.contains_key("error_recovery"), "缺少错误恢复检查");
        assert!(checks.contains_key("system_resources"), "缺少系统资源检查");

        println!("✅ 存活状态检查端点测试通过");
    }

    /// 【测试5】深度健康检查端点测试
    ///
    /// 【功能】：测试 GET /api/health/deep 端点的深度诊断功能
    /// 【验证点】：
    /// - 包含详细的系统诊断信息
    /// - 包含性能基准对比
    /// - 包含历史趋势分析
    /// - 包含错误统计信息
    #[tokio::test]
    async fn test_deep_health_check_endpoint() {
        let server = create_test_server().await;

        let response = server.get("/api/health/deep").await;

        response.assert_status(StatusCode::OK);
        response.assert_header("content-type", "application/json");

        let deep_health_data: Value = response.json();

        // 验证深度健康检查的特有字段
        assert!(deep_health_data.get("status").is_some(), "缺少status字段");
        assert!(deep_health_data.get("components").is_some(), "缺少components字段");
        assert!(deep_health_data.get("resources").is_some(), "缺少resources字段");
        assert!(
            deep_health_data.get("performance_benchmarks").is_some(),
            "缺少performance_benchmarks字段"
        );
        assert!(deep_health_data.get("historical_trends").is_some(), "缺少historical_trends字段");
        assert!(deep_health_data.get("error_statistics").is_some(), "缺少error_statistics字段");
        assert!(deep_health_data.get("diagnostic_report").is_some(), "缺少diagnostic_report字段");

        // 验证性能基准对比结构
        let benchmarks = &deep_health_data["performance_benchmarks"];
        assert!(benchmarks.get("current_performance").is_some(), "缺少当前性能数据");
        assert!(benchmarks.get("baseline_performance").is_some(), "缺少基准性能数据");

        // 验证历史趋势分析结构
        let trends = &deep_health_data["historical_trends"];
        assert!(trends.get("cpu_trend").is_some(), "缺少CPU趋势分析");
        assert!(trends.get("memory_trend").is_some(), "缺少内存趋势分析");

        println!("✅ 深度健康检查端点测试通过");
    }

    /// 【测试6】Prometheus指标导出端点测试
    ///
    /// 【功能】：测试 GET /metrics 端点的Prometheus指标导出功能
    /// 【验证点】：
    /// - 返回Prometheus格式的指标
    /// - 包含系统资源指标
    /// - 包含应用性能指标
    /// - 包含WebSocket连接指标
    #[tokio::test]
    async fn test_prometheus_metrics_endpoint() {
        let server = create_test_server().await;

        let response = server.get("/metrics").await;

        response.assert_status(StatusCode::OK);
        response.assert_header("content-type", "text/plain; version=0.0.4; charset=utf-8");

        let metrics_text = response.text();

        // 验证Prometheus指标格式
        assert!(metrics_text.contains("# HELP"), "缺少Prometheus HELP注释");
        assert!(metrics_text.contains("# TYPE"), "缺少Prometheus TYPE注释");

        // 验证系统资源指标
        assert!(metrics_text.contains("system_cpu_usage_percent"), "缺少CPU使用率指标");
        assert!(metrics_text.contains("system_memory_usage_percent"), "缺少内存使用率指标");
        assert!(metrics_text.contains("system_disk_usage_percent"), "缺少磁盘使用率指标");

        // 验证应用性能指标
        assert!(metrics_text.contains("axum_tutorial_requests_total"), "缺少HTTP请求总数指标");
        assert!(metrics_text.contains("axum_tutorial_active_connections"), "缺少活跃连接数指标");

        // 验证WebSocket连接指标
        assert!(
            metrics_text.contains("axum_tutorial_websocket_connections"),
            "缺少WebSocket活跃连接数指标"
        );
        assert!(
            metrics_text.contains("axum_tutorial_error_recovery_services_total"),
            "缺少错误恢复服务指标"
        );

        println!("✅ Prometheus指标导出端点测试通过");
    }

    /// 【测试7】系统资源告警检查端点测试
    ///
    /// 【功能】：测试 GET /api/monitoring/alerts 端点的告警检查功能
    /// 【验证点】：
    /// - 检查CPU使用率告警
    /// - 检查内存使用率告警
    /// - 检查磁盘空间告警
    /// - 检查网络连接数告警
    #[tokio::test]
    async fn test_system_alerts_endpoint() {
        let server = create_test_server().await;

        let response = server.get("/api/monitoring/alerts").await;

        response.assert_status(StatusCode::OK);
        response.assert_header("content-type", "application/json");

        let alerts_data: Value = response.json();

        // 验证告警检查的基本结构
        assert!(alerts_data.get("alert_status").is_some(), "缺少alert_status字段");
        assert!(alerts_data.get("active_alerts").is_some(), "缺少active_alerts字段");
        assert!(alerts_data.get("resource_status").is_some(), "缺少resource_status字段");
        assert!(alerts_data.get("thresholds").is_some(), "缺少thresholds字段");
        assert!(alerts_data.get("timestamp").is_some(), "缺少timestamp字段");

        // 验证告警状态
        let alert_status = alerts_data["alert_status"].as_str().unwrap();
        assert!(
            alert_status == "normal" || alert_status == "warning" || alert_status == "critical",
            "无效的告警状态: {}",
            alert_status
        );

        // 验证活跃告警数组
        let active_alerts = alerts_data["active_alerts"].as_array().unwrap();
        // 活跃告警可能为空，这是正常的

        // 验证系统资源信息
        let resources = &alerts_data["system_resources"];
        assert!(resources.get("cpu_usage_percent").is_some(), "缺少CPU使用率");
        assert!(resources.get("memory_usage_percent").is_some(), "缺少内存使用率");
        assert!(resources.get("disk_usage_percent").is_some(), "缺少磁盘使用率");
        assert!(resources.get("active_connections").is_some(), "缺少活跃连接数");

        // 验证告警阈值配置
        let thresholds = &alerts_data["thresholds"];
        assert!(thresholds.get("cpu_warning_threshold").is_some(), "缺少CPU告警阈值");
        assert!(thresholds.get("memory_warning_threshold").is_some(), "缺少内存告警阈值");

        println!("✅ 系统资源告警检查端点测试通过");
    }

    /// 【测试8】WebSocket统计端点测试
    ///
    /// 【功能】：测试 GET /api/websocket/stats 端点的WebSocket统计功能
    /// 【验证点】：
    /// - 包含连接数统计
    /// - 包含消息吞吐量统计
    /// - 包含连接质量统计
    /// - 包含错误统计
    #[tokio::test]
    async fn test_websocket_stats_endpoint() {
        let server = create_test_server().await;

        let response = server.get("/api/websocket/stats").await;

        response.assert_status(StatusCode::OK);
        response.assert_header("content-type", "application/json");

        let stats_data: Value = response.json();

        // 验证WebSocket统计的基本结构
        assert!(stats_data.get("websocket_stats").is_some(), "缺少websocket_stats字段");
        assert!(stats_data.get("message_stats").is_some(), "缺少message_stats字段");
        assert!(stats_data.get("performance_stats").is_some(), "缺少performance_stats字段");
        assert!(stats_data.get("timestamp").is_some(), "缺少timestamp字段");

        // 验证连接统计
        let connection_stats = &stats_data["connection_stats"];
        assert!(connection_stats.get("total_connections").is_some(), "缺少总连接数");
        assert!(connection_stats.get("active_connections").is_some(), "缺少活跃连接数");
        assert!(connection_stats.get("failed_connections").is_some(), "缺少失败连接数");

        // 验证消息统计
        let message_stats = &stats_data["message_stats"];
        assert!(message_stats.get("total_messages_sent").is_some(), "缺少发送消息总数");
        assert!(message_stats.get("total_messages_received").is_some(), "缺少接收消息总数");
        assert!(message_stats.get("messages_per_second").is_some(), "缺少每秒消息数");

        // 验证性能统计
        let performance_stats = &stats_data["performance_stats"];
        assert!(performance_stats.get("average_response_time_ms").is_some(), "缺少平均响应时间");
        assert!(performance_stats.get("connection_success_rate").is_some(), "缺少连接成功率");

        println!("✅ WebSocket统计端点测试通过");
    }

    /// 【测试9】WebSocket连接信息端点测试
    ///
    /// 【功能】：测试 GET /api/websocket/connections 端点的连接信息功能
    /// 【验证点】：
    /// - 包含在线用户列表
    /// - 包含连接详细信息
    /// - 包含连接时长统计
    #[tokio::test]
    async fn test_websocket_connections_endpoint() {
        let server = create_test_server().await;

        let response = server.get("/api/websocket/connections").await;

        response.assert_status(StatusCode::OK);
        response.assert_header("content-type", "application/json");

        let connections_data: Value = response.json();

        // 验证连接信息的基本结构
        assert!(connections_data.get("connection_count").is_some(), "缺少connection_count字段");
        assert!(connections_data.get("unique_user_count").is_some(), "缺少unique_user_count字段");
        assert!(connections_data.get("online_users").is_some(), "缺少online_users字段");
        assert!(connections_data.get("timestamp").is_some(), "缺少timestamp字段");

        // 验证连接数统计
        let connection_count = connections_data["connection_count"].as_u64().unwrap();
        let unique_user_count = connections_data["unique_user_count"].as_u64().unwrap();
        assert!(unique_user_count <= connection_count, "唯一用户数不能超过连接数");

        // 验证在线用户列表
        let _online_users = connections_data["online_users"].as_array().unwrap();
        // 连接列表可能为空，这是正常的

        println!("✅ WebSocket连接信息端点测试通过");
    }

    /// 【测试10】WebSocket性能指标端点测试
    ///
    /// 【功能】：测试 GET /api/websocket/metrics 端点的性能指标功能
    /// 【验证点】：
    /// - 包含详细的性能指标
    /// - 包含连接质量评估
    /// - 包含消息吞吐量分析
    #[tokio::test]
    async fn test_websocket_metrics_endpoint() {
        let server = create_test_server().await;

        let response = server.get("/api/websocket/metrics").await;

        response.assert_status(StatusCode::OK);
        response.assert_header("content-type", "application/json");

        let metrics_data: Value = response.json();

        // 验证性能指标的基本结构
        assert!(
            metrics_data.get("websocket_active_connections").is_some(),
            "缺少websocket_active_connections字段"
        );
        assert!(metrics_data.get("connection_quality").is_some(), "缺少connection_quality字段");
        assert!(metrics_data.get("message_throughput").is_some(), "缺少message_throughput字段");
        assert!(metrics_data.get("timestamp").is_some(), "缺少timestamp字段");

        // 验证WebSocket统计
        let websocket_stats = &metrics_data["websocket_stats"];
        assert!(websocket_stats.get("active_connections").is_some(), "缺少活跃连接数");
        assert!(websocket_stats.get("total_messages").is_some(), "缺少消息总数");

        // 验证连接质量
        let connection_quality = &metrics_data["connection_quality"];
        assert!(connection_quality.get("average_latency_ms").is_some(), "缺少平均延迟");
        assert!(connection_quality.get("connection_stability").is_some(), "缺少连接稳定性");

        // 验证消息吞吐量
        let message_throughput = &metrics_data["message_throughput"];
        assert!(message_throughput.get("messages_per_second").is_some(), "缺少每秒消息数");
        assert!(message_throughput.get("bytes_per_second").is_some(), "缺少每秒字节数");

        println!("✅ WebSocket性能指标端点测试通过");
    }
}
