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

// --- 导入项目内部模块 ---
use crate::app::middleware; // 中间件模块 (日志等)
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
    pub db_connection: DatabaseConnection, // 数据库连接池
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
pub async fn init_app(config: AppConfig) -> Result<Router> {
    // --- 步骤 1: 设置日志系统 ---
    middleware::setup_logger();
    println!("STARTUP: 日志系统初始化完成。");

    // --- 步骤 2: 建立数据库连接 ---
    // 直接在这里建立数据库连接，而不是通过旧的 `db.rs` 模块。
    let db_connection = Database::connect(&config.database_url).await?;
    println!("STARTUP: 数据库连接池创建完成。");

    // --- (新) 步骤 2.5: 自动执行数据库迁移 ---
    // 这是最佳实践：在应用启动时自动运行所有未应用的迁移。
    // `Migrator::up` 会检查数据库中的迁移历史记录，并只运行新的迁移脚本。
    // `&db_connection` 是对连接池的引用。
    // `None` 表示我们想要运行所有待处理的迁移。
    Migrator::up(&db_connection, None).await?;
    println!("STARTUP: 数据库迁移检查与应用完成。");

    // 注意: 旧的 `db::new_db()` 和 `db::init_sample_data()` 已被移除。

    // --- 步骤 3: 创建应用状态 ---
    let app_state = AppState { db_connection };
    println!("STARTUP: 应用共享状态 (AppState) 创建完成。");

    // --- 步骤 4: 构建中间件栈 ---
    let middleware_stack = ServiceBuilder::new()
        .layer(middleware::trace_layer())
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any));
    println!("STARTUP: 中间件栈构建完成 (Trace, CORS)。");

    // --- 步骤 5: 创建应用路由并应用中间件 ---
    let app = routes::create_routes(app_state).layer(middleware_stack);
    println!("STARTUP: 路由创建并应用中间件完成。");

    // --- 步骤 6: 返回配置好的应用 ---
    println!("STARTUP: 应用初始化流程完成。");
    Ok(app)
}
