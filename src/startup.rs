// 文件路径: src/startup.rs
//
// /--------------------------------------------------------------------------------------------\
// |                               【启动与初始化模块】 (startup.rs)                              |
// |--------------------------------------------------------------------------------------------|
// |                                                                                            |
// | 1. **导入依赖**:                                                                           |
// |    - `axum::Router`: Axum 核心路由类型。                                                 |
// |    - `tower::ServiceBuilder`: 用于构建和组合中间件 (Middleware)。                         |
// |    - `tower_http::cors::CorsLayer`: 用于配置 CORS 策略的中间件。                           |
// |    - `crate::*`: 导入应用内部的其他模块 (controller, middleware, config, db, routes)。   |
// |                                                                                            |
// | 2. **`init_app` 函数**: 公共异步函数，负责整个应用的初始化和组装。                           |
// |    - **输入**: `_config: AppConfig` (应用配置，当前未使用，但保留以备将来扩展)。             |
// |    - **输出**: `axum::Router` (完全配置好的、可运行的应用实例)。                            |
// |    - **核心步骤**:                                                                          |
// |      a. **设置日志 (`middleware::setup_logger()`)**: 初始化 `tracing` 日志系统。          |
// |      b. **创建数据库 (`db::new_db()`)**: 初始化内存数据库实例 (`Arc<RwLock<HashMap>>`)。 |
// |      c. **填充数据 (`db::init_sample_data(&db)`)**: 向内存数据库添加示例数据。             |
// |      d. **创建应用状态 (`AppState { db }`)**: 创建包含数据库连接池的共享状态结构体。         |
// |      e. **构建中间件栈 (`middleware_stack`)**: [[Axum 核心概念: 中间件]]                  |
// |         - 使用 `ServiceBuilder::new()` 开始构建。                                         |
// |         - `.layer(middleware)`: 将中间件添加到栈中。[[Tower 核心概念: Layer]]          |
// |           - `middleware::trace_layer()`: 添加请求日志跟踪中间件。                        |
// |           - `CorsLayer::new()...`: 添加 CORS 中间件，配置允许跨域请求。                   |
// |         - **执行顺序**: 中间件按添加顺序的反向执行请求处理，按添加顺序执行响应处理。         |
// |      f. **创建路由 (`routes::create_routes(app_state)`)**: 调用 `routes` 模块创建路由。    |
// |      g. **应用中间件 (`.layer(middleware_stack)`)**: 将整个中间件栈应用到所有路由上。       |
// |                                                                                            |
// \--------------------------------------------------------------------------------------------/
//
// 【核心职责】: 作为应用程序启动的"总装车间"，将日志、数据库、配置、状态、中间件和路由等各个部分有机地组合在一起，生成最终可运行的 Axum 应用实例。
// 【关键技术】: `axum::Router`, 中间件 (`tower::Layer`, `tower::ServiceBuilder`), 状态管理 (`AppState`), 依赖注入 (将 `AppState` 传递给路由)。

// --- 导入依赖 ---
use axum::Router; // Axum 的核心路由类型
use tower::ServiceBuilder; // Tower 提供的用于构建中间件栈的服务构建器
use tower_http::cors::{ Any, CorsLayer }; // Tower HTTP 提供的 CORS 中间件和相关配置
use anyhow::Result; // anyhow 用于简化错误处理
use migration::{ Migrator, MigratorTrait }; // 导入迁移器
use sea_orm::{ Database, DatabaseConnection }; // 导入 SeaORM 的核心类型
use std::sync::Arc;
use tracing::warn; // 【任务13.4新增】用于记录警告日志

// --- 导入项目内部模块 ---
use crate::app::middleware; // 中间件模块 (日志等)
use crate::app::middleware::security_audit::security_audit_middleware; // 安全审计中间件
use crate::app::middleware::error_recovery_middleware::ErrorRecoveryState; // 错误恢复中间件状态
use crate::app::repository::task_repository::TaskRepository; // 导入仓库
use crate::app::service::{
    AsyncPerformanceOptimizer,
    ConnectionManager,
    MessageDistributor,
    NotificationService,
    StatusSyncService,
}; // 导入连接管理器、消息分发器、通知服务、状态同步服务和异步性能优化器
use crate::app::utils::{
    ErrorRecoveryManager,
    memory_manager::{ MemoryManager, MemoryManagerConfig },
    DatabasePoolManager, // 【任务13.4新增】数据库连接池管理器
    WebSocketPoolManager, // 【任务13.4新增】WebSocket连接池管理器
}; // 错误恢复管理器
use crate::config::AppConfig; // 应用配置结构体
use crate::routes; // 路由定义模块

/// 应用的共享状态结构体 (`AppState`)
///
/// 【用途】: 这个结构体持有了所有需要在不同 Axum Handler 之间共享的状态。
///          最典型的共享状态就是数据库连接池。
/// 【设计】: 使用 `#[derive(Clone)]` 宏，使得 `AppState` 可以被轻松地克隆。
///          Axum 在分发请求给不同的 Handler 时，需要克隆这个状态。
///          `DatabaseConnection` 本身是设计为可以被克隆的（它内部使用了 `Arc`）。
#[derive(Clone)]
pub struct AppState {
    pub task_repo: Arc<TaskRepository>, // 任务仓库的具体实现
    pub db: Arc<DatabaseConnection>, // 数据库连接，用于创建其他仓库实例
    pub jwt_secret: String, // JWT 签名密钥
    pub connection_manager: Arc<ConnectionManager>, // WebSocket 连接管理器
    pub message_distributor: Arc<MessageDistributor>, // 消息分发器
    pub notification_service: Arc<NotificationService>, // 用户通知服务
    pub status_sync_service: Arc<StatusSyncService>, // 状态同步服务

    // 【任务13.4新增】连接池管理器
    pub database_pool_manager: Arc<DatabasePoolManager>, // 数据库连接池管理器
    pub websocket_pool_manager: Arc<WebSocketPoolManager>, // WebSocket连接池管理器
    pub performance_metrics: Arc<middleware::PerformanceMetrics>, // 性能指标收集器
    pub error_recovery_state: ErrorRecoveryState, // 错误恢复状态
    pub async_performance_optimizer: Arc<AsyncPerformanceOptimizer>, // 【任务13.2新增】异步性能优化器
    pub memory_manager: Arc<MemoryManager>, // 【任务13.3新增】内存管理器
}

impl AppState {
    /// 创建测试用的AppState
    /// 注意：这个函数创建一个简化的测试状态，用于单元测试
    pub async fn new_for_testing(connection_manager: ConnectionManager) -> Self {
        let connection_manager = Arc::new(connection_manager);
        let message_distributor = Arc::new(
            MessageDistributor::new(
                connection_manager.clone(),
                Some(10), // 小批量用于测试
                Some(1) // 单线程用于测试
            )
        );
        let notification_service = Arc::new(
            NotificationService::new(connection_manager.clone(), message_distributor.clone())
        );
        let status_sync_service = Arc::new(
            StatusSyncService::new(connection_manager.clone(), message_distributor.clone())
        );

        // 创建一个内存数据库连接（用于测试）
        let db_connection = sea_orm::Database
            ::connect("sqlite::memory:").await
            .expect("Failed to create test database");

        // 创建性能指标收集器
        let performance_config = middleware::PerformanceConfig {
            enable_detailed_logging: true,
            enable_system_monitoring: false, // 测试时关闭系统监控
            system_monitoring_interval: 30,
            enable_prometheus_metrics: true,
            slow_request_threshold_ms: 1000,
            log_request_headers: false,
            max_concurrent_connections_warning: 1000,
            enable_size_monitoring: true,
            enable_user_agent_stats: false, // 测试时关闭
            enable_geo_stats: false,
            enable_error_classification: true,
            max_user_agent_cache_size: 100,
            max_geo_cache_size: 50,
        };
        let performance_metrics =
            middleware::create_performance_monitoring_layer(performance_config);

        // 创建错误恢复管理器
        let error_recovery_manager = ErrorRecoveryManager::with_default_config();
        let error_recovery_state = ErrorRecoveryState::new(error_recovery_manager);

        // 【任务13.2新增】创建异步性能优化器（测试配置）
        let async_perf_config = crate::app::service::AsyncPerformanceConfig {
            worker_threads: Some(2), // 测试时使用较少线程
            performance_monitoring_interval: 5, // 测试时使用较短间隔
            max_concurrent_tasks: 100, // 测试时使用较小限制
            ..Default::default()
        };
        let async_performance_optimizer = Arc::new(
            AsyncPerformanceOptimizer::new(async_perf_config)
        );

        // 【任务13.3新增】创建内存管理器（测试配置）
        let memory_config = MemoryManagerConfig {
            l1_cache_max_entries: 100, // 测试时使用小缓存
            l1_cache_ttl_seconds: 60, // 1分钟TTL
            object_pool_initial_size: 10, // 小对象池
            object_pool_max_size: 50,
            memory_pool_block_size: 1024, // 1KB块
            memory_pool_max_blocks: 20,
            cache_eviction_interval_seconds: 30,
            memory_monitoring_interval_seconds: 10, // 测试时使用短间隔
            memory_pressure_threshold_bytes: 1024 * 1024, // 1MB阈值
            enable_leak_detection: false, // 测试时关闭泄漏检测
        };
        let memory_manager = Arc::new(MemoryManager::new(memory_config));

        // 【任务13.4新增】创建测试环境的连接池管理器
        let test_config = AppConfig {
            http_addr: "127.0.0.1:3000".parse().unwrap(),
            database_url: "sqlite::memory:".to_string(),
            jwt_secret: "test_secret".to_string(),
            database_pool: crate::config::DatabasePoolConfig::development(),
            websocket_pool: crate::config::WebSocketPoolConfig::development(),
        };

        // 创建数据库连接池管理器（测试环境）
        let database_pool_manager = Arc::new(
            DatabasePoolManager::new(&test_config).await.expect(
                "Failed to create test database pool manager"
            )
        );

        // 创建WebSocket连接池管理器（测试环境）
        let websocket_pool_manager = Arc::new(
            WebSocketPoolManager::new(test_config.websocket_pool)
        );

        let db_arc = Arc::new(db_connection);
        Self {
            task_repo: Arc::new(TaskRepository::from_arc(db_arc.clone())),
            db: db_arc,
            jwt_secret: "test_secret".to_string(),
            connection_manager,
            message_distributor,
            notification_service,
            status_sync_service,
            performance_metrics,
            error_recovery_state,
            async_performance_optimizer,
            memory_manager,
            database_pool_manager, // 【任务13.4新增】
            websocket_pool_manager, // 【任务13.4新增】
        }
    }
}

// --- 初始化函数 ---

/// 初始化并组装整个 Axum 应用程序 (Function to Initialize the Application)
///
/// 【功能】: 这个函数是应用程序启动过程的核心协调者。
///          它负责按顺序执行所有必要的初始化步骤，并将各个组件连接起来，
///          最终返回一个配置完整、准备好运行的 `axum::Router`。
///
/// # 【参数】
/// * `config: AppConfig` - 应用程序的配置信息。[[所有权: 移动]]
///
/// # 【返回值】
/// * `-> Result<Router>`: 返回一个 `anyhow::Result`。
///    - `Ok(Router)`: 成功时返回配置好的 `Router`。
///    - `Err(error)`: 如果在初始化过程中（如数据库连接失败）发生错误，则返回错误。
pub async fn init_app(config: AppConfig) -> Result<(Router, Arc<DatabaseConnection>)> {
    // --- 步骤 1: 设置日志系统 ---
    middleware::setup_logger();
    println!("STARTUP: 日志系统初始化完成。");

    // --- 步骤 2: 建立优化的数据库连接池 ---
    // 【任务13.4】使用新的数据库连接池管理器，支持企业级连接池优化
    let database_pool_manager = Arc::new(DatabasePoolManager::new(&config).await?);
    let db_arc = database_pool_manager.get_connection();
    println!("STARTUP: 优化的数据库连接池创建完成。");

    // --- (新) 步骤 2.5: 自动执行数据库迁移 ---
    // 这是最佳实践：在应用启动时自动运行所有未应用的迁移。
    // `Migrator::up` 会检查数据库中的迁移历史记录，并只运行新的迁移脚本。
    // `&db_connection` 是对连接池的引用。
    // `None` 表示我们想要运行所有待处理的迁移。
    Migrator::up(db_arc.as_ref(), None).await?;
    println!("STARTUP: 数据库迁移检查与应用完成。");

    // 注意: 旧的 `db::new_db()` 和 `db::init_sample_data()` 已被移除。

    // --- 步骤 3: 创建仓库和应用状态 ---
    // 创建仓库实例
    let task_repo = Arc::new(TaskRepository::from_arc(db_arc.clone()));
    // 创建连接管理器实例
    let connection_manager = Arc::new(ConnectionManager::new());
    // 创建消息分发器实例
    let message_distributor = Arc::new(
        MessageDistributor::new(
            connection_manager.clone(),
            Some(100), // 批量处理大小
            Some(4) // 工作线程数量
        )
    );
    // 创建通知服务实例
    let notification_service = Arc::new(
        NotificationService::new(connection_manager.clone(), message_distributor.clone())
    );
    // 创建状态同步服务实例
    let status_sync_service = Arc::new(
        StatusSyncService::new(connection_manager.clone(), message_distributor.clone())
    );
    // 创建性能监控中间件
    let performance_config = middleware::PerformanceConfig {
        enable_prometheus_metrics: false, // 暂时禁用Prometheus以简化初始实现
        ..Default::default()
    };
    let performance_metrics = middleware::create_performance_monitoring_layer(performance_config);

    // 创建错误恢复管理器
    let error_recovery_manager = ErrorRecoveryManager::with_default_config();
    let error_recovery_state = ErrorRecoveryState::new(error_recovery_manager);

    // 【任务13.2新增】创建异步性能优化器（生产配置）
    let async_perf_config = crate::app::service::AsyncPerformanceConfig {
        worker_threads: None, // 使用CPU核心数
        performance_monitoring_interval: 10, // 生产环境使用10秒间隔
        max_concurrent_tasks: 10000, // 生产环境支持更多并发
        enable_adaptive_tuning: true, // 启用自适应调优
        ..Default::default()
    };
    let async_performance_optimizer = Arc::new(AsyncPerformanceOptimizer::new(async_perf_config));

    // 【任务13.3新增】创建内存管理器（生产配置）
    let memory_config = MemoryManagerConfig {
        l1_cache_max_entries: 10000, // 生产环境使用大缓存
        l1_cache_ttl_seconds: 300, // 5分钟TTL
        object_pool_initial_size: 1000, // 大对象池
        object_pool_max_size: 10000,
        memory_pool_block_size: 4096, // 4KB块
        memory_pool_max_blocks: 1000,
        cache_eviction_interval_seconds: 60,
        memory_monitoring_interval_seconds: 30, // 生产环境30秒间隔
        memory_pressure_threshold_bytes: 1024 * 1024 * 1024, // 1GB阈值
        enable_leak_detection: true, // 生产环境启用泄漏检测
    };
    let memory_manager = Arc::new(MemoryManager::new(memory_config));

    // 【任务13.4新增】创建WebSocket连接池管理器
    let websocket_pool_manager = Arc::new(WebSocketPoolManager::new(config.websocket_pool.clone()));
    println!("STARTUP: WebSocket连接池管理器创建完成。");

    // 创建应用状态，包含任务仓库、数据库连接、JWT 密钥、连接管理器、消息分发器、通知服务、状态同步服务、性能指标、错误恢复状态、异步性能优化器、内存管理器和连接池管理器
    let app_state = AppState {
        task_repo,
        db: db_arc, // 添加数据库连接到应用状态
        jwt_secret: config.jwt_secret.clone(), // 添加 JWT 密钥到应用状态
        connection_manager, // 添加连接管理器到应用状态
        message_distributor, // 添加消息分发器到应用状态
        notification_service, // 添加通知服务到应用状态
        status_sync_service, // 添加状态同步服务到应用状态
        performance_metrics: performance_metrics.clone(), // 克隆性能指标收集器到应用状态
        error_recovery_state: error_recovery_state.clone(), // 克隆错误恢复状态到应用状态
        async_performance_optimizer, // 【任务13.2新增】添加异步性能优化器到应用状态
        memory_manager, // 【任务13.3新增】添加内存管理器到应用状态
        database_pool_manager: database_pool_manager.clone(), // 【任务13.4新增】添加数据库连接池管理器到应用状态
        websocket_pool_manager, // 【任务13.4新增】添加WebSocket连接池管理器到应用状态
    };
    println!("STARTUP: 应用共享状态 (AppState) 创建完成。");

    // --- 步骤 3.5: 启动消息分发器工作线程 ---
    let _worker_handles = app_state.message_distributor.start_workers();
    println!("STARTUP: 消息分发器工作线程已启动。");

    // 【任务13.2新增】--- 步骤 3.6: 启动异步性能优化器 ---
    if let Err(e) = app_state.async_performance_optimizer.start().await {
        eprintln!("STARTUP ERROR: 异步性能优化器启动失败: {}", e);
        return Err(anyhow::anyhow!("异步性能优化器启动失败: {}", e));
    }
    println!("STARTUP: 异步性能优化器已启动。");

    // 【任务13.3新增】--- 步骤 3.7: 启动内存管理器 ---
    app_state.memory_manager.start().await;
    println!("STARTUP: 内存管理器已启动。");

    // 【任务13.4新增】--- 步骤 3.8: 启动连接池监控任务 ---
    let _db_monitoring_handle = app_state.database_pool_manager.start_monitoring();
    println!("STARTUP: 数据库连接池监控任务已启动。");

    // 执行初始健康检查
    if let Err(e) = app_state.database_pool_manager.health_check().await {
        warn!("STARTUP: 数据库连接池初始健康检查失败: {}", e);
    }

    if let Err(e) = app_state.websocket_pool_manager.health_check().await {
        warn!("STARTUP: WebSocket连接池初始健康检查失败: {}", e);
    }
    println!("STARTUP: 连接池健康检查完成。");

    // --- 步骤 4: 构建中间件栈 ---
    let middleware_stack = ServiceBuilder::new()
        .layer(middleware::trace_layer())
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        // 添加安全审计中间件 - 记录所有HTTP请求的安全相关信息
        .layer(axum::middleware::from_fn(security_audit_middleware))
        // 添加性能监控中间件 - 使用 from_fn_with_state
        .layer(
            axum::middleware::from_fn_with_state(
                performance_metrics.clone(),
                middleware::performance_monitoring_middleware
            )
        )
        // 添加错误恢复中间件 - 提供自动重试、断路器和降级处理
        .layer(
            axum::middleware::from_fn_with_state(
                app_state.clone(),
                middleware::error_recovery_middleware::error_recovery_middleware
            )
        );
    println!(
        "STARTUP: 中间件栈构建完成 (Trace, CORS, Security Audit, Performance, Error Recovery)。"
    );

    // --- 步骤 5: 创建应用路由并应用中间件 ---
    let app = routes::create_routes(app_state.clone()).layer(middleware_stack);
    println!("STARTUP: 路由创建并应用中间件完成。");

    // --- 步骤 6: 返回配置好的应用和数据库连接 ---
    println!("STARTUP: 应用初始化流程完成。");
    // 返回Arc<DatabaseConnection>
    let db_for_return = database_pool_manager.get_connection();
    Ok((app, db_for_return))
}
