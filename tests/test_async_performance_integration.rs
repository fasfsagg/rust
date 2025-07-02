// tests/test_async_performance_integration.rs
//
// /--------------------------------------------------------------------------------------------\
// |                        【任务13.2集成测试】异步性能优化器API测试                            |
// |--------------------------------------------------------------------------------------------|
// |                                                                                            |
// | 【测试目标】:                                                                               |
// | 1. **API端点测试**: 验证 GET /api/performance/async-stats 端点功能                         |
// | 2. **统计数据验证**: 确保异步性能统计数据的准确性和完整性                                   |
// | 3. **响应格式验证**: 验证API响应的JSON格式和字段完整性                                      |
// | 4. **性能指标验证**: 确保各项性能指标的合理性和一致性                                       |
// |                                                                                            |
// | 【测试覆盖】:                                                                               |
// | - 异步性能优化器启动和运行状态                                                               |
// | - 任务调度器统计信息                                                                        |
// | - I/O批处理统计信息                                                                         |
// | - 背压控制统计信息                                                                          |
// | - API响应格式和状态码                                                                       |
// |                                                                                            |
// \--------------------------------------------------------------------------------------------/

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use axum_tutorial::{
    app::{
        controller::performance_controller::get_async_performance_stats,
        service::{AsyncPerformanceOptimizer, ConnectionManager},
    },
    routes::create_routes,
    startup::AppState,
};
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;
use tower::ServiceExt;

/// 【任务13.2集成测试】测试异步性能统计API端点
///
/// 【功能】: 测试 GET /api/performance/async-stats 端点的功能
/// 【验证】:
/// - API响应格式正确
/// - 统计数据准确性
/// - 响应状态码为200
/// - 各项性能指标的合理性
#[tokio::test]
async fn test_async_performance_stats_api() {
    // 创建连接管理器和应用状态
    let connection_manager = ConnectionManager::new();
    let app_state = AppState::new_for_testing(connection_manager).await;

    // 启动异步性能优化器
    app_state.async_performance_optimizer.start().await.unwrap();

    // 等待一小段时间让优化器初始化
    sleep(Duration::from_millis(100)).await;

    // 创建路由
    let app = create_routes(app_state.clone());

    // 创建请求
    let request = Request::builder()
        .method("GET")
        .uri("/api/performance/async-stats")
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap();

    // 发送请求
    let response = app.oneshot(request).await.unwrap();

    // 验证响应状态码
    assert_eq!(response.status(), StatusCode::OK);

    // 将响应转换为字节
    let response_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response_text = String::from_utf8(response_bytes.to_vec()).unwrap();

    // 解析JSON响应
    let json_response: Value = serde_json::from_str(&response_text).unwrap();

    // 验证响应结构
    assert!(json_response.is_object());

    // 验证必要字段存在
    let required_fields = [
        "total_tasks_processed",
        "successful_tasks",
        "failed_tasks",
        "average_task_duration_ms",
        "p99_latency_ms",
        "throughput_per_second",
        "scheduler_efficiency",
        "queue_lengths",
        "backpressure_activations",
        "rejection_rate",
        "available_permits",
        "io_batch_efficiency",
        "total_batches",
        "average_batch_size",
        "memory_usage_mb",
        "cpu_usage_percent",
        "timestamp",
    ];

    for field in &required_fields {
        assert!(
            json_response.get(field).is_some(),
            "Missing required field: {}",
            field
        );
    }

    // 验证数值字段的合理性
    assert!(json_response["total_tasks_processed"].as_u64().unwrap() >= 0);
    assert!(json_response["successful_tasks"].as_u64().unwrap() >= 0);
    assert!(json_response["failed_tasks"].as_u64().unwrap() >= 0);
    assert!(json_response["average_task_duration_ms"].as_f64().unwrap() >= 0.0);
    assert!(json_response["p99_latency_ms"].as_f64().unwrap() >= 0.0);
    assert!(json_response["throughput_per_second"].as_f64().unwrap() >= 0.0);
    assert!(json_response["scheduler_efficiency"].as_f64().unwrap() >= 0.0);
    assert!(json_response["scheduler_efficiency"].as_f64().unwrap() <= 100.0);
    assert!(json_response["rejection_rate"].as_f64().unwrap() >= 0.0);
    assert!(json_response["rejection_rate"].as_f64().unwrap() <= 100.0);
    assert!(json_response["available_permits"].as_u64().unwrap() >= 0);
    assert!(json_response["io_batch_efficiency"].as_f64().unwrap() >= 0.0);
    assert!(json_response["total_batches"].as_u64().unwrap() >= 0);
    assert!(json_response["average_batch_size"].as_f64().unwrap() >= 0.0);
    assert!(json_response["memory_usage_mb"].as_f64().unwrap() >= 0.0);
    assert!(json_response["cpu_usage_percent"].as_f64().unwrap() >= 0.0);
    assert!(json_response["cpu_usage_percent"].as_f64().unwrap() <= 100.0);
    assert!(json_response["timestamp"].as_u64().unwrap() > 0);

    // 验证队列长度信息
    let queue_lengths = json_response["queue_lengths"].as_object().unwrap();
    assert!(queue_lengths.contains_key("critical"));
    assert!(queue_lengths.contains_key("high"));
    assert!(queue_lengths.contains_key("normal"));
    assert!(queue_lengths.contains_key("low"));

    // 停止异步性能优化器
    app_state.async_performance_optimizer.stop().await.unwrap();

    println!("✅ 异步性能统计API测试通过");
}

/// 【任务13.2集成测试】测试异步性能优化器的基本功能
///
/// 【功能】: 测试异步性能优化器的启动、运行和停止
/// 【验证】:
/// - 优化器能够正常启动和停止
/// - 统计信息能够正确更新
/// - 各个组件能够协同工作
#[tokio::test]
async fn test_async_performance_optimizer_functionality() {
    // 创建连接管理器和应用状态
    let connection_manager = ConnectionManager::new();
    let app_state = AppState::new_for_testing(connection_manager).await;

    // 验证初始状态
    assert!(!app_state.async_performance_optimizer.is_running());

    // 启动异步性能优化器
    let start_result = app_state.async_performance_optimizer.start().await;
    assert!(start_result.is_ok());
    assert!(app_state.async_performance_optimizer.is_running());

    // 等待一小段时间让优化器运行
    sleep(Duration::from_millis(200)).await;

    // 获取性能统计信息
    let stats = app_state.async_performance_optimizer.get_performance_stats().await;
    assert_eq!(stats.total_tasks_processed, 0); // 初始状态应该没有处理任务
    assert_eq!(stats.successful_tasks, 0);
    assert_eq!(stats.failed_tasks, 0);

    // 获取调度器统计信息
    let scheduler_stats = app_state.async_performance_optimizer.get_scheduler_stats().await;
    assert_eq!(scheduler_stats.total_tasks_scheduled, 0);
    assert!(scheduler_stats.queue_lengths.contains_key("critical"));
    assert!(scheduler_stats.queue_lengths.contains_key("high"));
    assert!(scheduler_stats.queue_lengths.contains_key("normal"));
    assert!(scheduler_stats.queue_lengths.contains_key("low"));

    // 获取背压统计信息
    let backpressure_stats = app_state.async_performance_optimizer.get_backpressure_stats();
    assert_eq!(backpressure_stats.total_requests, 0);
    assert_eq!(backpressure_stats.rejected_requests, 0);
    assert_eq!(backpressure_stats.rejection_rate, 0.0);

    // 获取I/O统计信息
    let io_stats = app_state.async_performance_optimizer.get_io_stats().await;
    assert_eq!(io_stats.total_batches, 0);
    assert_eq!(io_stats.total_operations, 0);
    assert_eq!(io_stats.average_batch_size, 0.0);

    // 停止异步性能优化器
    let stop_result = app_state.async_performance_optimizer.stop().await;
    assert!(stop_result.is_ok());
    assert!(!app_state.async_performance_optimizer.is_running());

    println!("✅ 异步性能优化器功能测试通过");
}

/// 【任务13.2集成测试】测试异步性能优化器配置
///
/// 【功能】: 测试异步性能优化器的配置参数
/// 【验证】:
/// - 配置参数能够正确设置
/// - 配置信息能够正确获取
#[tokio::test]
async fn test_async_performance_optimizer_config() {
    // 创建连接管理器和应用状态
    let connection_manager = ConnectionManager::new();
    let app_state = AppState::new_for_testing(connection_manager).await;

    // 获取配置信息
    let config = app_state.async_performance_optimizer.get_config();

    // 验证测试配置
    assert_eq!(config.worker_threads, Some(2)); // 测试配置使用2个线程
    assert_eq!(config.performance_monitoring_interval, 5); // 测试配置使用5秒间隔
    assert_eq!(config.max_concurrent_tasks, 100); // 测试配置使用100个并发任务
    assert!(config.enable_task_scheduling_optimization);
    assert!(config.enable_io_multiplexing_optimization);
    assert!(config.enable_backpressure_handling);
    assert!(config.enable_priority_scheduling);
    assert!(config.enable_adaptive_tuning);

    println!("✅ 异步性能优化器配置测试通过");
}
