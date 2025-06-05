// 文件路径: src/startup.rs

// /--------------------------------------------------------------------------------------------------\
// |                                      【模块功能图示】 (startup.rs)                                 |
// |--------------------------------------------------------------------------------------------------|
// |                                                                                                  |
// | [main.rs 调用 init_app(config)]                                                                    |
// |      |                                                                                           |
// |      V                                                                                           |
// | [init_app() 函数开始执行]                                                                          |
// |   1. [中间件: setup_logger()] (初始化日志系统, 例如 tracing)                                       |
// |   2. [数据库: db::establish_connection()] (使用 config 中的 database_url 连接数据库)                 |
// |   3. [数据库: db::run_migrations()] (执行数据库迁移, 例如创建表)                                   |
// |   4. [应用状态: AppState 构建] (将数据库连接池 db_conn 包装在 Arc 中存入 AppState)                   |
// |   5. [中间件栈: ServiceBuilder 构建]                                                               |
// |      - layer(trace_layer) (请求追踪日志)                                                          |
// |      - layer(CorsLayer) (跨域资源共享配置)                                                        |
// |   6. [路由: routes::create_routes(app_state)] (创建所有 API 路由, 传入 AppState)                 |
// |   7. [.layer(middleware_stack)] (将构建的中间件栈应用到所有路由上)                                  |
// |      |                                                                                           |
// |      V                                                                                           |
// | [返回配置完成的 axum::Router 实例给 main.rs]                                                       |
// |                                                                                                  |
// \--------------------------------------------------------------------------------------------------/
//
// 【模块核心职责】 (Module Core Responsibilities)
// 1. **初始化核心组件 (Initialize Core Components)**: 负责应用程序启动时所有基本组件的初始化和配置工作。
//    这包括日志系统、数据库连接建立、数据库迁移执行。
// 2. **构建应用状态 (Construct Application State)**: 创建 `AppState` 实例，该实例通常包含需要在整个应用中共享的资源，
//    例如数据库连接池。通过 `Arc` 实现安全共享。
// 3. **配置中间件 (Configure Middleware)**: 定义并组装中间件栈，这些中间件将应用于所有或部分 HTTP 请求，
//    提供如日志记录、CORS 处理、认证等横切关注点功能。
// 4. **组装路由系统 (Assemble Routing System)**: 调用路由定义模块 (`routes.rs`) 来创建所有的 API 路由，并将应用状态和中间件
//    与这些路由关联起来。
// 5. **返回应用实例 (Return Application Instance)**: 最终返回一个完全配置好的 `axum::Router` 实例，该实例已准备好被 `main.rs` 用来启动 Web 服务器。
//
// 【关键技术点】 (Key Technologies)
// - **异步函数 (`async fn`)**: `init_app` 是一个异步函数，因为它内部调用了其他异步操作 (如数据库连接、迁移)。
// - **应用配置 (`AppConfig`)**: 接收从 `main.rs` 传递过来的 `AppConfig` 实例，用于获取数据库 URL 等配置。
// - **Axum Router (`axum::Router`)**: Axum 框架的核心，用于定义 HTTP 路由规则并将请求分派到相应的处理函数。
// - **Tower `ServiceBuilder` 和 `Layer`**: `tower` 是一个用于构建网络服务的库，Axum 的中间件系统基于它。
//   - `ServiceBuilder`: 用于链式地将多个中间件 (`Layer`) 组合成一个处理栈。
//   - `Layer`: 代表一个中间件层，它可以修改请求、响应或控制请求流程。
// - **`tower_http` 中间件**:
//   - `CorsLayer`: 用于处理跨域资源共享 (CORS) 的标准中间件。
//   - `TraceLayer` (通过 `middleware::trace_layer()`): 通常包装 `tower_http::trace::TraceLayer`，用于生成详细的请求/响应日志。
// - **应用状态管理 (`AppState`, `std::sync::Arc`)**:
//   - `AppState`: 自定义结构体，用于封装所有需要在 Axum 处理函数间共享的数据（如数据库连接池）。
//   - `std::sync::Arc` (Atomic Reference Counting): 智能指针，允许多个所有者安全地共享数据。
//     用于包装数据库连接 `DatabaseConnection`，使其可以在多个并发请求和线程之间安全共享。
// - **错误处理 (`.expect()`)**: 在启动阶段，如果关键步骤 (如数据库连接、迁移) 失败，使用 `.expect()` 会导致程序 panic。
//   这是一种“快速失败”策略，因为如果这些核心组件无法初始化，应用程序通常无法正常运行。

// --- 导入依赖 ---
// `use` 关键字用于将其他模块或库中定义的项（结构体、函数、trait 等）引入当前文件的作用域。

// `axum::Router`: 从 `axum` crate 导入核心的 `Router` 类型，用于构建和组织应用的路由。
use axum::Router;
// `tower::ServiceBuilder`: 从 `tower` crate 导入 `ServiceBuilder`。
// `tower` 是一个构建健壮网络服务的库，Axum 的中间件系统基于 `tower` 的 `Layer` 和 `Service` 概念。
// `ServiceBuilder` 提供了一种方便的方式来将多个中间件层 (layers) 组合起来。
use tower::ServiceBuilder;
// `tower_http::cors::{Any, CorsLayer}`: 从 `tower_http` crate (一个提供常用 HTTP 中间件的库) 中导入 CORS 处理相关的组件。
//   - `CorsLayer`: 实现 CORS 逻辑的中间件层。
//   - `Any`: 通常用于 `CorsLayer` 的配置，表示允许任何来源、方法或头部 (在开发中常用，生产中应更严格)。
use tower_http::cors::{Any, CorsLayer};

// --- 导入项目内部模块 ---
// `crate::` 前缀表示从当前项目的根模块开始的路径。

// `crate::app::state::AppState`: 导入在 `src/app/state.rs` 中定义的 `AppState` 结构体。
// `AppState` 用于在 Axum 的路由处理函数之间共享数据 (例如数据库连接池)。
use crate::app::state::AppState;
// `crate::app::middleware`: 导入在 `src/app/middleware/mod.rs` 中定义的中间件模块。
// 这允许我们访问自定义的中间件，如 `setup_logger` 和 `trace_layer`。
use crate::app::middleware;
// `crate::config::AppConfig`: 导入在 `src/config.rs` 中定义的 `AppConfig` 结构体。
// `AppConfig` 持有应用的配置信息，如数据库 URL、服务器监听地址等。
use crate::config::AppConfig;
// `crate::db`: 导入在 `src/db.rs` 中定义的数据库操作模块。
// 我们将使用它来建立数据库连接 (`establish_connection`) 和运行迁移 (`run_migrations`)。
use crate::db;
// `crate::routes`: 导入在 `src/routes.rs` 中定义的路由创建模块。
// `create_routes` 函数将负责定义应用的所有 API 端点。
use crate::routes;
// `std::sync::Arc`: 导入标准库中的原子引用计数智能指针 `Arc`。
// `Arc` 用于在多线程环境下安全地共享数据所有权。
use std::sync::Arc;

// --- 初始化函数 ---

// `pub async fn init_app(config: AppConfig) -> Router`: 定义一个公共的异步函数 `init_app`。
// - `pub`: 表示这个函数是公共的，可以从项目中的其他模块（如 `main.rs`）调用。
// - `async fn`: 表示这是一个【异步函数】。异步函数允许在等待 I/O 操作 (如数据库查询、网络请求) 完成时，
//   程序可以暂停当前函数的执行，转而去处理其他任务，从而提高并发性能和响应性。
//   异步函数的执行由异步运行时 (如 Tokio) 管理。
// - `init_app`: 函数名，清晰地表明其职责是初始化应用程序。
// - `config: AppConfig`: 参数列表。此函数接收一个 `AppConfig` 类型的参数，名为 `config`。
//   - `AppConfig`: 这是我们之前定义的配置结构体。
//   - `config`: 当调用 `init_app` 时，一个 `AppConfig` 的实例会被传递进来。
//     这里参数 `config` 获取了 `AppConfig` 实例的【所有权 (ownership)】。
//     如果 `AppConfig` 实现了 `Clone` trait (本项目中是这样)，调用者可以通过 `config.clone()` 来传递副本，
//     从而保留原始 `config` 的所有权。
// - `-> Router`: 返回类型。此函数在完成后会返回一个 `axum::Router` 类型的实例。
//   这个 `Router` 实例将包含所有配置好的路由、中间件和共享状态。
/// 初始化并组装整个 Axum 应用程序。
///
/// 【功能】: 这个函数是应用程序启动过程的核心协调者。
///          它负责按顺序执行所有必要的初始化步骤 (日志、数据库、状态、中间件、路由)，
///          并将各个组件连接起来，最终返回一个配置完整、准备好运行的 `axum::Router`。
///
/// # 【参数】
/// * `config: AppConfig` - 应用程序的配置信息。函数会取得此配置的所有权。
///                         用于获取数据库 URL、服务器地址等。
///
/// # 【返回值】
/// * `-> Router`: 返回一个 `axum::Router` 实例。
///                这个实例已经包含了所有定义的路由、应用的中间件栈以及注入的共享状态。
///                它将被传递给 `main.rs` 中的服务器启动逻辑。
pub async fn init_app(config: AppConfig) -> Router {
    // --- 步骤 1: 设置日志系统 ---
    // `middleware::setup_logger()`: 调用 `middleware` 模块 (具体是 `logger.rs`) 中的 `setup_logger` 函数。
    // 这个函数通常会配置 `tracing` crate (一个强大的日志和分布式追踪框架)，
    // 包括设置日志的格式、输出级别 (例如 DEBUG, INFO, ERROR)、以及日志输出的目标 (例如控制台或文件)。
    // **重要性**: 日志对于观察应用行为、调试问题和监控生产环境至关重要。
    middleware::setup_logger();
    // `println!` 是一个标准宏，用于向控制台输出文本。这里用于在启动时提供一些反馈。
    // 在 `tracing` 配置完成后，通常会使用 `tracing::info!`, `tracing::debug!` 等宏来记录日志。
    println!("STARTUP: 日志系统初始化完成。");

    // --- 步骤 2: 建立数据库连接 ---
    // `println!` 输出提示信息。
    println!("STARTUP: 正在建立数据库连接...");
    // `db::establish_connection(&config.database_url).await`:
    //   - `db::establish_connection`: 调用 `db` 模块中的 `establish_connection` 异步函数。
    //   - `&config.database_url`: 将 `config` 结构体中的 `database_url` 字段的【不可变引用】传递给函数。
    //     `&` 表示借用，函数会读取这个 URL 但不会获取其所有权。
    //   - `.await`: 因为 `establish_connection` 是异步的，我们在此等待数据库连接操作完成。
    // `match ... { ... }`: `match` 表达式用于处理 `establish_connection` 返回的 `Result<DatabaseConnection, DbErr>`。
    let db_conn = match db::establish_connection(&config.database_url).await {
        // `Ok(conn)`: 如果连接成功，`conn` 是 `sea_orm::DatabaseConnection` 类型的实例。
        Ok(conn) => {
            println!("STARTUP: 数据库连接成功建立。");
            conn // 将成功的连接 `conn` 作为 `match` 表达式的结果，赋值给 `db_conn`。
        }
        // `Err(e)`: 如果连接失败，`e` 是 `sea_orm::DbErr` 类型的错误。
        Err(e) => {
            // `eprintln!` 将错误信息输出到标准错误流 (stderr)。
            eprintln!("STARTUP: 数据库连接失败: {:?}", e);
            // `panic!("...")`: 这是一个【关键错误处理点】。
            // 如果数据库连接无法建立，应用程序通常无法继续运行。
            // `panic!` 会立即终止当前线程的执行，并通常导致整个程序退出，同时打印提供的错误消息和调用栈信息。
            // 在启动阶段，对于这种关键依赖的失败，"快速失败" (fail-fast) 是一种常见的策略。
            panic!("数据库连接失败，应用无法启动: {:?}", e);
        }
    };
    // `db_conn` 现在持有一个有效的数据库连接 (通常是一个连接池)。

    // --- 步骤 3: 运行数据库迁移 ---
    println!("STARTUP: 正在运行数据库迁移...");
    // `db::run_migrations(&db_conn).await`:
    //   - `db::run_migrations`: 调用 `db` 模块中的 `run_migrations` 异步函数。
    //   - `&db_conn`: 将数据库连接 `db_conn` 的【不可变引用】传递给函数。迁移操作需要使用这个连接来执行 SQL。
    //   - `.await`: 等待迁移操作完成。
    // `if let Err(e) = ...`: `if let` 是一种简洁的模式匹配方式，用于处理 `Result` 只关心 `Err` 的情况。
    if let Err(e) = db::run_migrations(&db_conn).await {
        eprintln!("STARTUP: 数据库迁移失败: {:?}", e);
        // 与数据库连接类似，如果迁移失败 (例如表结构定义错误)，应用程序可能无法正常工作。
        panic!("数据库迁移失败，应用无法启动: {:?}", e);
    }
    println!("STARTUP: 数据库迁移成功完成。");

    // --- 步骤 4: 创建应用状态 (AppState) ---
    // `AppState` 是我们定义的结构体，用于在 Axum 的请求处理函数之间共享数据。
    // `let app_state = AppState { ... };`: 创建 `AppState` 的一个实例。
    //   - `db_conn: Arc::new(db_conn)`: 这是核心部分。
    //     - `db_conn`: 我们之前建立的 `sea_orm::DatabaseConnection` 实例。
    //     - `Arc::new(db_conn)`: 将 `db_conn` 包装在一个 `std::sync::Arc` (原子引用计数智能指针) 中。
    //       **为什么使用 `Arc`?**
    //       - **共享所有权**: Axum 的路由处理函数可能会在不同的线程上并发执行。`Arc` 允许多个部分“共享”对 `db_conn` 的所有权。
    //         `DatabaseConnection` 本身可能不是 `Clone` 的，或者克隆成本很高。`Arc<T>` 实现了 `Clone`，
    //         克隆 `Arc` 只是增加内部的引用计数，而不会复制 `T` (即 `db_conn`) 本身。这非常高效。
    //       - **线程安全**: `Arc` 确保了对内部数据的访问是线程安全的 (只要内部数据 `T` 本身是 `Send + Sync`的，`DatabaseConnection` 通常是这样设计的)。
    //       - **Axum 要求**: Axum 的 `State` 提取器要求共享状态必须是 `Clone` 的。
    //       **内存示意**:
    //       - `app_state` (栈上变量)
    //         - `db_conn` 字段 (类型 `Arc<DatabaseConnection>`) (栈上，存储指向堆的指针和引用计数)
    //           |
    //           V
    //       - `DatabaseConnection` 实例 (堆上分配的实际连接池数据) <- 多个 `Arc` 指针可以指向这里
    let app_state = AppState { db_conn: Arc::new(db_conn) };
    println!("STARTUP: 应用共享状态 (AppState) 创建完成。");

    // --- 步骤 5: 构建中间件栈 (Middleware Stack) ---
    // `ServiceBuilder::new()`: 创建一个新的 `tower::ServiceBuilder` 实例。
    // `ServiceBuilder` 用于通过链式调用 `.layer()` 方法来组合多个中间件。
    // 中间件是处理 HTTP 请求和响应的组件，它们可以被“层叠”起来形成一个处理管道。
    let middleware_stack = ServiceBuilder::new()
        // `.layer(middleware)`: 将一个中间件层添加到 `ServiceBuilder` 中。
        // 中间件的应用顺序很重要：
        // - 对于请求：最后添加的中间件最先处理请求 (像洋葱的外层)。
        // - 对于响应：最先添加的中间件最先处理响应 (像洋葱的内层先出来)。

        // `middleware::trace_layer()`: 添加我们自定义的日志跟踪中间件。
        //   - 这个函数通常返回一个配置好的 `tower_http::trace::TraceLayer` 实例。
        //   - `TraceLayer` 会记录关于每个请求的详细信息，例如请求方法、路径、状态码、处理延迟等。
        //     这对于调试和监控应用非常有用。
        .layer(middleware::trace_layer())
        // `CorsLayer::new()...`: 添加 `tower_http::cors::CorsLayer` 中间件来处理 CORS (跨域资源共享)。
        //   - **CORS 是什么?** 是一种浏览器安全机制，用于控制来自不同源 (域名、协议、端口) 的 Web 应用发起的 HTTP 请求。
        //     默认情况下，浏览器会阻止跨域 AJAX 请求。CORS 允许服务器指定哪些源可以访问其资源。
        //   - `.allow_origin(Any)`: 允许来自任何源 (origin) 的跨域请求。
        //     在生产环境中，通常会配置为只允许特定的、受信任的前端应用源，例如 `.allow_origin("https://myfrontend.com".parse::<HeaderValue>().unwrap())`。
        //   - `.allow_methods(Any)`: 允许所有常见的 HTTP 方法 (GET, POST, PUT, DELETE, OPTIONS 等)。
        //     也可以指定一个具体的列表，例如 `[Method::GET, Method::POST]`。
        //   - `.allow_headers(Any)`: 允许客户端在请求中发送任何 HTTP 头部。
        //     也可以指定一个具体的列表。
        .layer(
            CorsLayer::new()
                .allow_origin(Any) // 允许任何来源 (开发时方便，生产环境应更严格)
                .allow_methods(Any) // 允许任何 HTTP 方法
                .allow_headers(Any) // 允许任何 HTTP 请求头
        );
    println!("STARTUP: 中间件栈构建完成 (Trace, CORS)。");

    // --- 步骤 6: 创建应用路由并应用顶层中间件 ---
    // `routes::create_routes(app_state.clone())`: 调用 `routes` 模块中的 `create_routes` 函数。
    //   - `app_state.clone()`: 将 `AppState` 的克隆版本传递给路由创建函数。
    //     因为 `app_state.db_conn` 是 `Arc<DatabaseConnection>`，所以克隆 `app_state` 也是廉价的 (只增加 `Arc` 的引用计数)。
    //     `create_routes` 函数内部会将这个 `AppState` 注入到需要它的路由处理函数中。
    //   - `create_routes` 返回一个定义了所有 API 端点（如 /api/register, /api/login, /api/protected_data）的 `axum::Router`。
    let app_router = routes::create_routes(app_state.clone()) // 注意：这里再次克隆 app_state
        // `.layer(middleware_stack)`: 将之前构建的整个 `middleware_stack` (包含 Trace 和 CORS) 应用到由 `create_routes` 返回的 `Router` 上。
        // 这意味着所有通过 `app_router` 处理的请求，都会先经过 `middleware_stack` 中定义的中间件。
        // 这种方式是将中间件应用到一组路由的顶层。
        .layer(middleware_stack);
    println!("STARTUP: 路由创建并应用中间件完成。");

    // --- 步骤 7: 返回配置好的应用 ---
    // `app_router` 现在是一个完全配置好的 `axum::Router` 实例，包含了：
    //   - 所有定义的 API 路由。
    //   - 注入到路由中的共享应用状态 (`AppState`)。
    //   - 应用于所有路由的顶层中间件 (Trace, CORS)。
    //   - （`routes.rs` 内部可能还为特定路由组应用了其他中间件，如 `jwt_auth_middleware`）。
    // 这个 `Router` 将被返回给 `main.rs`，用于启动 Web 服务器。
    println!("STARTUP: 应用初始化流程完成。");
    app_router
}

[end of src/startup.rs]
