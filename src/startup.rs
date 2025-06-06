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

// --- 导入项目内部模块 ---
use crate::app::AppState; // 导入新的 AppState
use crate::app::middleware; // 中间件模块 (日志等)
use crate::config::AppConfig; // 应用配置结构体
use crate::db; // 数据库模块 (SeaORM)
use crate::routes; // 路由定义模块

// --- 初始化函数 ---

/// 初始化并组装整个 Axum 应用程序 (Function to Initialize the Application)
///
/// 【功能】: 这个函数是应用程序启动过程的核心协调者。
///          它负责按顺序执行所有必要的初始化步骤，并将各个组件连接起来，
///          最终返回一个配置完整、准备好运行的 `axum::Router`。
///
/// # 【参数】
/// * `config: AppConfig` - 应用程序的配置信息。
///
/// # 【返回值】
/// * `-> Router`: 返回一个 `axum::Router` 实例。
///                这个实例已经包含了所有定义的路由、应用的中间件栈以及注入的共享状态。
///                它将被传递给 `main.rs` 中的 `axum::serve` 来启动服务器。
pub async fn init_app(config: AppConfig) -> Router {
    // --- 步骤 1: 设置日志系统 ---
    // 调用 `middleware` 模块中的 `setup_logger` 函数。
    // 这通常会配置 `tracing` crate，设置日志级别、格式和输出目标（例如控制台）。
    middleware::setup_logger();
    println!("STARTUP: 日志系统初始化完成。");

    // --- 步骤 2: 连接数据库 ---
    println!("STARTUP: 正在建立数据库连接...");
    let db_conn = db::establish_connection()
        .await
        .expect("STARTUP: 数据库连接失败");
    println!("STARTUP: 数据库连接成功。");

    // --- 步骤 3: 运行数据库迁移 (如果需要) ---
    // 注意: run_migrations 是一个占位符，实际的迁移逻辑可能更复杂
    // 或者通过外部工具 (如 sea-orm-cli) 处理。
    // 如果 DATABASE_URL 指向的是一个需要通过 mode=rwc 创建的文件，
    // 并且 users 表尚不存在，则需要运行迁移或手动创建。
    if let Err(e) = db::run_migrations(&db_conn).await {
        eprintln!("STARTUP: 数据库迁移失败: {:?}", e);
        // 根据策略，这里可以选择 panic 或者继续 (如果表可能已存在)
        // For now, we'll proceed, assuming tables might exist or are handled otherwise.
    } else {
        println!("STARTUP: 数据库迁移（占位符）执行完成。");
    }

    // --- 步骤 4: 创建应用状态 ---
    // 创建 AppState 结构体的实例。
    // 它现在包含 SeaORM 数据库连接 (`db_conn`) 和应用配置 (`config`)。
    // AppConfig 需要实现 Clone。
    let app_state = AppState {
        db: db_conn,
        config: config.clone(), // AppConfig 必须是 Clone
    };
    println!("STARTUP: 应用共享状态 (AppState) 创建完成。");

    // --- 步骤 5: 构建中间件栈 ---
    // 使用 `tower::ServiceBuilder` 来定义和组合中间件。
    // 中间件是在请求到达处理函数之前和响应返回给客户端之后执行的逻辑层。
    let middleware_stack = ServiceBuilder::new()
        // `.layer()` 方法将一个中间件层添加到构建器中。
        // 中间件按 `.layer()` 调用的【顺序】应用。
        // 请求会【反向】通过这些层（最后添加的最先处理请求），响应会【正向】通过这些层（最先添加的最先处理响应）。

        // 添加日志跟踪中间件 (来自 middleware 模块)。
        // 这个中间件通常会记录每个请求的详细信息（方法、路径、状态码、耗时等）。
        .layer(middleware::trace_layer())
        // 添加 CORS (跨域资源共享) 中间件。
        // 这对于允许来自不同源（例如，运行在不同端口的前端应用）的 JavaScript 代码访问 API 至关重要。
        .layer(
            CorsLayer::new()
                // `allow_origin(Any)`: 允许来自任何源的请求。
                // 在生产环境中，通常应配置为只允许特定的源。
                .allow_origin(Any)
                // `allow_methods(Any)`: 允许所有常见的 HTTP 方法 (GET, POST, PUT, DELETE, etc.)。
                // 也可以指定具体允许的方法列表。
                .allow_methods(Any)
                // `allow_headers(Any)`: 允许客户端发送任何自定义的请求头。
                // 也可以指定具体允许的头部列表。
                .allow_headers(Any)
        );
    println!("STARTUP: 中间件栈构建完成 (Trace, CORS)。");

    // --- 步骤 6: 创建应用路由并应用中间件 ---
    // 调用 `routes` 模块的 `create_routes` 函数，并将 `app_state` 传递给它。
    // 这会返回一个包含所有已定义路由（API 路由、WebSocket 路由、静态文件服务）的 `Router`。
    let app = routes
        ::create_routes(app_state)
        // `.layer(middleware_stack)`: 将之前构建的整个 `middleware_stack` 应用到【所有】路由上。
        // 这意味着每个进入应用的请求都会先经过 CORS 和 Trace 中间件的处理。
        .layer(middleware_stack);
    println!("STARTUP: 路由创建并应用中间件完成。");

    // --- 步骤 7: 返回配置好的应用 ---
    // 返回最终的 `Router` 实例，它现在包含了所有的路由、中间件和共享状态，
    // 准备好被 `main.rs` 中的服务器使用了。
    println!("STARTUP: 应用初始化流程完成。");
    app
}
