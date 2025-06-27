// routes.rs
//
// /------------------------------------------------------------------------------------------------------\
// |                                     【路由定义模块】 (routes.rs)                                     |
// |------------------------------------------------------------------------------------------------------|
// |                                                                                                      |
// | 1. **导入依赖**:                                                                                     |
// |    - `axum::routing::{get, post, ...}, Router`: Axum 路由构建组件。                                 |
// |    - `tower_http::services::ServeDir`: 用于提供静态文件服务。                                       |
// |    - `crate::app::controller::*`: 控制器层的处理函数和共享状态 (`AppState`)。                        |
// |                                                                                                      |
// | 2. **`create_routes` 函数**: 公共函数，负责创建和配置整个应用的路由。                                  |
// |    - **输入**: `app_state: AppState` (包含数据库连接池等共享状态)。                                  |
// |    - **输出**: `axum::Router` (配置好的路由实例)。                                                  |
// |    - **内部逻辑**:                                                                                     |
// |      a. **创建 API 路由 (`api_routes`)**:                                                            |
// |         - `Router::new()`: 创建一个新的路由实例。                                                  |
// |         - `.route("/path", method(handler))`: 定义路由规则。 [[Axum 核心概念: 路由定义]]         |
// |           - `"/path"`: URL 路径 (可以是静态路径 `/tasks` 或带参数路径 `/tasks/:id`)。             |
// |           - `method`: HTTP 方法 (如 `get`, `post`, `put`, `delete`)。                              |
// |           - `handler`: 当请求匹配路径和方法时调用的【控制器函数】 (例如 `get_all_tasks`)。        |
// |         - `.with_state(app_state.clone())`: 将共享状态 `AppState` 注入到 `api_routes` 的所有        |
// |           处理器中。[[Axum 核心概念: 状态共享]] Axum 会自动将 State 作为参数传递给处理器。         |
// |           (需要克隆 `Arc<T>` 类型的 `app_state`)。                                                |
// |      b. **创建 WebSocket 路由 (`ws_routes`)**:                                                       |
// |         - 类似地定义 `/ws` 路径的 GET 请求，并映射到 `ws_handler`。                                |
// |      c. **组合路由**:                                                                                |
// |         - `Router::new()`: 创建最终的主路由。                                                    |
// |         - `.nest("/api", api_routes)`: 将 `api_routes` 下的所有路由挂载到 `/api` 前缀下。          |
// |           例如，`/tasks` 变成 `/api/tasks`。[[Axum 功能: 嵌套路由]]                              |
// |         - `.merge(ws_routes)`: 将 `ws_routes` 合并到主路由中。[[Axum 功能: 合并路由]]               |
// |         - `.nest_service("/", ServeDir::new("static"))`: 将根路径 `/` 下的所有请求（未被上面   |
// |           路由匹配的）交给 `ServeDir` 服务处理，用于提供 `static` 目录下的静态文件 (如 HTML, CSS)。 |
// |           [[Axum 功能: 静态文件服务]]                                                            |
// |                                                                                                      |
// \------------------------------------------------------------------------------------------------------/
//
// 【核心职责】: 定义应用程序的 URL 结构，将外部 HTTP 请求精确地导向到内部的业务逻辑处理函数（控制器）。
// 【关键技术】: Axum Router (`axum::Router`), HTTP 方法映射 (`get`, `post`), 状态注入 (`with_state`), 路由组织 (`nest`, `merge`), 静态文件服务 (`ServeDir`).

// --- 导入依赖 ---
// `axum::routing::{...}`: 导入 Axum 用于定义路由和 HTTP 方法处理器的函数。
// `Router`: Axum 的核心路由构建器类型。
use axum::{ routing::{ get, post, put, delete }, Router };
// `tower_http::services::ServeDir`: 导入 Tower HTTP 库提供的服务，用于从目录提供静态文件。
use tower_http::services::ServeDir;

// --- 导入控制器层组件 ---
// 导入在 `src/app/controller/` 模块中定义的处理函数。
// 这是路由层与控制器层的连接点。
use crate::app::controller::{
    create_task, // 处理 POST /api/tasks
    delete_task, // 处理 DELETE /api/tasks/:id
    get_all_tasks, // 处理 GET /api/tasks
    get_task_by_id, // 处理 GET /api/tasks/:id
    update_task, // 处理 PUT /api/tasks/:id
    get_online_users, // 【任务7实现】处理 GET /api/online-users
    ws_handler, // 处理 GET /ws
    // 认证相关处理函数
    login_handler, // 处理 POST /api/auth/login
    register_handler, // 处理 POST /api/auth/register
    // 消息相关处理函数
    search_messages, // 处理 GET /api/messages/search
    get_chat_room_messages, // 处理 GET /api/messages/chat-room/:id
    // 性能监控相关处理函数
    get_performance_stats, // 处理 GET /api/performance/stats
    health_check, // 处理 GET /api/performance/health
    get_detailed_metrics, // 处理 GET /api/performance/metrics
    get_prometheus_metrics, // 处理 GET /api/performance/prometheus
    readiness_check, // 处理 GET /api/performance/ready
    liveness_check, // 处理 GET /api/performance/live
    deep_health_check, // 处理 GET /api/health/deep
    // Favicon处理函数
    favicon_handler, // 处理 GET /favicon.ico
    // 【任务12.7实现】WebSocket监控相关处理函数
    get_websocket_stats, // 处理 GET /api/websocket/stats
    get_websocket_connections, // 处理 GET /api/websocket/connections
    get_websocket_metrics, // 处理 GET /api/websocket/metrics
    // 【任务12.8实现】系统资源监控和告警阈值检查处理函数
    get_system_alerts, // 处理 GET /api/monitoring/alerts
};
// 导入在 `src/startup.rs` 中定义的唯一的共享应用状态 `AppState`。
use crate::startup::AppState;
// 导入JWT认证中间件
use crate::app::middleware::auth_middleware::create_jwt_auth_middleware;
// 导入Axum中间件相关功能
use axum::middleware;

// --- 路由创建函数 ---

/// 创建认证相关路由 (Function to Create Authentication Routes)
///
/// 【功能】: 创建并配置所有认证相关的路由，包括用户注册和登录。
///
/// # 【参数】
/// * `app_state: AppState` - 应用程序的共享状态，包含数据库连接等资源。
///
/// # 【返回值】
/// * `-> Router`: 返回配置好的认证路由实例，包含状态注入。
fn auth_routes(app_state: AppState) -> Router {
    Router::new()
        // POST /register - 用户注册
        .route("/register", post(register_handler))
        // POST /login - 用户登录
        .route("/login", post(login_handler))
        // 注入应用状态，使处理函数可以访问数据库连接等资源
        .with_state(app_state)
}

/// 创建并配置应用程序的所有路由 (Function to Create Application Routes)
///
/// 【功能】: 集中定义应用程序的 URL 结构，并将每个 URL 路径 + HTTP 方法组合映射到相应的控制器处理函数。
///          同时，配置状态共享和静态文件服务。
///
/// # 【参数】
/// * `app_state: AppState` - 应用程序的共享状态。[[所有权: 移动]]
///   它通常包含数据库连接池 (`Db`) 或其他需要在多个请求处理函数之间共享的资源。
///   这个 `AppState` 会被注入到需要它的路由处理函数中。
///
/// # 【返回值】
/// * `-> Router`: 返回一个完全配置好的 `axum::Router` 实例。
///   这个 `Router` 实例随后会被传递给 `axum::serve` 来启动服务器。
pub fn create_routes(app_state: AppState) -> Router {
    // --- 定义 API 相关路由 ---
    // 创建一个专门用于处理 `/api` 前缀下所有请求的子路由。
    let api_routes = Router::new()
        // 定义 GET /tasks 路由，映射到 get_all_tasks 控制器函数。调用.route() 方法来定义一个路由。返回一个 Router<AppState> 实例。
        // 这个 Router 实例是"携带"了 AppState 这种共享状态的。处理函数可以访问到 AppState 中包含的数据
        // 注意: 同一个路径 "/tasks" 可以根据 HTTP 方法 (GET vs POST) 映射到不同的处理函数。
        .route("/tasks", get(get_all_tasks))
        // 定义 POST /tasks 路由，映射到 create_task 控制器函数。
        // 注意: 同一个路径 "/tasks" 可以根据 HTTP 方法 (GET vs POST) 映射到不同的处理函数。
        .route("/tasks", post(create_task))
        // 定义 GET /tasks/{id} 路由，映射到 get_task_by_id 控制器函数。
        // `{id}` 是一个【路径参数】(Path Parameter)。[[Axum 功能: 路径参数]]
        // Axum 会自动解析 URL 中的这部分，并通过 `axum::extract::Path` 提取器将其传递给处理函数。
        .route("/tasks/{id}", get(get_task_by_id))
        // 定义 PUT /tasks/{id} 路由，映射到 update_task 控制器函数。
        .route("/tasks/{id}", put(update_task))
        // 定义 DELETE /tasks/{id} 路由，映射到 delete_task 控制器函数。
        .route("/tasks/{id}", delete(delete_task))
        // 【任务7实现】定义 GET /online-users 路由，映射到 get_online_users 控制器函数。
        // 用于获取当前在线用户列表，支持聊天大厅的在线用户显示功能。
        .route("/online-users", get(get_online_users))
        // 【任务9实现】消息搜索和过滤路由
        // 定义 GET /messages/search 路由，映射到 search_messages 控制器函数。
        // 支持关键词搜索、高级过滤和分页，为企业级聊天应用提供强大的消息检索功能。
        .route("/messages/search", get(search_messages))
        // 定义 GET /messages/chat-room/{id} 路由，映射到 get_chat_room_messages 控制器函数。
        // 获取特定聊天室的消息列表，支持过滤和分页功能。
        .route("/messages/chat-room/{id}", get(get_chat_room_messages))
        // --- 应用JWT认证中间件到任务路由 ---
        // 使用 `.route_layer()` 将JWT认证中间件应用到所有上述任务路由
        // 这确保了只有携带有效JWT令牌的请求才能访问任务相关的API端点
        .route_layer(middleware::from_fn(create_jwt_auth_middleware(app_state.jwt_secret.clone())))
        // --- 注入共享状态 ---
        // `.with_state(app_state.clone())`: 将 `app_state` 注入到上面定义的所有 API 路由的处理函数中。
        // **重要**: 因为 `AppState` 通常包含 `Arc<...>` 类型（如我们的 `Db`），所以克隆 `app_state` 是一个廉价的操作
        //           (只会增加 `Arc` 的引用计数，不会复制内部数据)。[[所有权: 克隆 Arc]]
        //           Axum 要求 State 必须是 `Clone` 的。
        //           处理函数可以通过添加 `axum::extract::State<AppState>` 类型的参数来访问这个状态。
        .with_state(app_state.clone()); // 使用 `.clone()` 传递 Arc 包装的状态

    // --- 定义 WebSocket 相关路由 ---
    // 创建一个处理 WebSocket 连接的子路由。
    let ws_routes = Router::new()
        // 定义 GET /ws 路由，映射到 ws_handler 控制器函数，用于处理 WebSocket 升级请求。
        .route("/ws", get(ws_handler))
        // WebSocket 通常不需要共享数据库状态，所以这里没有 `.with_state()`。
        // 如果需要，也可以像 api_routes 一样添加 `.with_state()`。
        .with_state(app_state.clone()); // 如果 ws_handler 需要 AppState，也注入

    // --- 创建认证路由 ---
    // 创建认证相关的路由，包括用户注册和登录
    let auth_routes = auth_routes(app_state.clone());

    // --- 创建性能监控路由（公开访问，无需认证）---
    // 创建性能监控相关的路由，用于系统监控和健康检查
    let performance_routes = Router::new()
        // 【任务11.5实现】性能监控和指标日志路由
        // 定义 GET /performance/stats 路由，映射到 get_performance_stats 控制器函数。
        // 获取当前的性能统计信息，包括请求数、连接数、成功率等。
        .route("/performance/stats", get(get_performance_stats))
        // 定义 GET /performance/health 路由，映射到 health_check 控制器函数。
        // 系统健康检查，包括数据库连接、内存使用等状态检查。
        .route("/performance/health", get(health_check))
        // 定义 GET /performance/metrics 路由，映射到 get_detailed_metrics 控制器函数。
        // 获取详细的性能指标，包括系统信息和应用信息。
        .route("/performance/metrics", get(get_detailed_metrics))
        // 定义 GET /performance/prometheus 路由，映射到 get_prometheus_metrics 控制器函数。
        // 导出Prometheus格式的指标，用于监控系统集成。
        .route("/performance/prometheus", get(get_prometheus_metrics))
        // 定义 GET /performance/ready 路由，映射到 readiness_check 控制器函数。
        // 应用就绪状态检查，用于Kubernetes等容器编排系统的就绪探针。
        .route("/performance/ready", get(readiness_check))
        // 定义 GET /performance/live 路由，映射到 liveness_check 控制器函数。
        // 应用存活状态检查，用于Kubernetes等容器编排系统的存活探针。
        .route("/performance/live", get(liveness_check))
        // 注入应用状态，使处理函数可以访问性能指标收集器等资源
        .with_state(app_state.clone());

    // --- 创建健康检查路由（公开访问，无需认证）---
    // 创建专门的健康检查路由，提供不同级别的健康检查服务
    let health_routes = Router::new()
        // 【任务12.5实现】深度健康检查路由
        // 定义 GET /health/deep 路由，映射到 deep_health_check 控制器函数。
        // 提供最详细的系统诊断信息，包括性能基准对比、历史趋势分析、错误统计等。
        // 适用于运维人员深度诊断系统问题。
        .route("/health/deep", get(deep_health_check))
        // 注入应用状态，使处理函数可以访问所有系统资源
        .with_state(app_state.clone());

    // --- 创建错误恢复监控路由（公开访问，无需认证）---
    // 创建错误恢复相关的路由，用于监控错误恢复状态和统计
    let error_recovery_routes = Router::new()
        // 【任务11.7实现】错误恢复状态监控路由
        // 定义 GET /error-recovery/status 路由，映射到 error_recovery_status_handler 控制器函数。
        // 获取当前的错误恢复状态，包括重试统计、断路器状态、降级状态等。
        .route(
            "/error-recovery/status",
            get(crate::app::middleware::error_recovery_middleware::error_recovery_status_handler)
        )
        // 注入应用状态，使处理函数可以访问错误恢复管理器等资源
        .with_state(app_state.clone());

    // --- 创建WebSocket监控路由（公开访问，无需认证）---
    // 创建WebSocket监控相关的路由，用于监控WebSocket连接状态和性能
    let websocket_monitoring_routes = Router::new()
        // 【任务12.7实现】WebSocket连接监控和统计路由
        // 定义 GET /websocket/stats 路由，映射到 get_websocket_stats 控制器函数。
        // 提供WebSocket连接的详细统计信息，包括连接数、消息吞吐量、连接质量等。
        .route("/websocket/stats", get(get_websocket_stats))
        // 定义 GET /websocket/connections 路由，映射到 get_websocket_connections 控制器函数。
        // 提供当前活跃WebSocket连接的详细信息，包括在线用户列表。
        .route("/websocket/connections", get(get_websocket_connections))
        // 定义 GET /websocket/metrics 路由，映射到 get_websocket_metrics 控制器函数。
        // 提供WebSocket性能相关的详细指标，适用于监控系统集成。
        .route("/websocket/metrics", get(get_websocket_metrics))
        // 注入应用状态，使处理函数可以访问连接管理器等资源
        .with_state(app_state.clone());

    // --- 创建系统监控路由（公开访问，无需认证）---
    // 创建系统资源监控相关的路由，用于监控系统资源和告警状态
    let system_monitoring_routes = Router::new()
        // 【任务12.8实现】系统资源监控和告警阈值检查路由
        // 定义 GET /monitoring/alerts 路由，映射到 get_system_alerts 控制器函数。
        // 提供系统资源告警检查，包括CPU使用率、内存使用率、磁盘空间、网络连接数等阈值监控和告警状态。
        .route("/monitoring/alerts", get(get_system_alerts))
        // 注入应用状态，使处理函数可以访问系统资源监控等资源
        .with_state(app_state.clone());

    // --- 创建标准Prometheus指标路由（公开访问，无需认证）---
    // 【任务12.2实现】创建标准的 /metrics 端点，符合Prometheus监控系统的标准
    let metrics_routes = Router::new()
        // 定义 GET /metrics 路由，映射到 get_prometheus_metrics 控制器函数。
        // 这是Prometheus监控系统的标准端点，用于抓取应用指标。
        .route("/metrics", get(get_prometheus_metrics))
        // 注入应用状态，使处理函数可以访问性能指标收集器等资源
        .with_state(app_state.clone());

    // --- 组合所有路由 ---
    // 创建最终的根路由，并将上面定义的子路由和静态文件服务组合起来。
    Router::new()
        // 【Favicon路由】：专门处理 /favicon.ico 请求，避免404错误
        // 使用专门的路由处理器，确保在静态文件服务之前匹配
        .route("/favicon.ico", get(favicon_handler))
        // `.nest("/api", api_routes)`: 将 `api_routes` 下定义的所有路由挂载到 `/api` 路径前缀下。
        // 例如，之前定义的 `/tasks` 会变成 `/api/tasks`。
        // 这有助于组织路由，将所有 API 相关端点归类。
        .nest("/api", api_routes)
        // `.nest("/api/auth", auth_routes)`: 将认证路由挂载到 `/api/auth` 路径前缀下。
        // 例如，`/register` 会变成 `/api/auth/register`，`/login` 会变成 `/api/auth/login`。
        .nest("/api/auth", auth_routes)
        // `.nest("/api", performance_routes)`: 将性能监控路由挂载到 `/api` 路径前缀下。
        // 例如，`/performance/stats` 会变成 `/api/performance/stats`。
        .nest("/api", performance_routes)
        // `.nest("/api", health_routes)`: 将健康检查路由挂载到 `/api` 路径前缀下。
        // 例如，`/health/deep` 会变成 `/api/health/deep`。
        .nest("/api", health_routes)
        // `.nest("/api", error_recovery_routes)`: 将错误恢复监控路由挂载到 `/api` 路径前缀下。
        // 例如，`/error-recovery/status` 会变成 `/api/error-recovery/status`。
        .nest("/api", error_recovery_routes)
        // `.nest("/api", websocket_monitoring_routes)`: 将WebSocket监控路由挂载到 `/api` 路径前缀下。
        // 例如，`/websocket/stats` 会变成 `/api/websocket/stats`。
        .nest("/api", websocket_monitoring_routes)
        // `.nest("/api", system_monitoring_routes)`: 将系统监控路由挂载到 `/api` 路径前缀下。
        // 例如，`/monitoring/alerts` 会变成 `/api/monitoring/alerts`。
        .nest("/api", system_monitoring_routes)
        // `.merge(ws_routes)`: 将 `ws_routes` 定义的路由合并到当前路由层级。
        // 这里 `/ws` 路由仍然是根路径下的 `/ws`。
        .merge(ws_routes)
        // `.merge(metrics_routes)`: 将 `metrics_routes` 定义的路由合并到当前路由层级。
        // 这里 `/metrics` 路由是根路径下的 `/metrics`，符合Prometheus标准。
        .merge(metrics_routes)
        // `.fallback_service(ServeDir::new("static"))`: 配置静态文件服务。
        //   - Axum 0.8.4 变更：根路径的 nest_service 不再支持，改用 fallback_service
        //   - `ServeDir::new("static")`: 创建一个服务，它会查找并返回 `static` 目录下对应的文件。
        //   - `fallback_service`: 当所有路由都不匹配时，使用此服务处理请求。
        //   【效果】: 当请求 `http://localhost:3000/` 时，会返回 `static/index.html`。
        //           当请求 `http://localhost:3000/styles.css` 时，会返回 `static/styles.css`。
        //           这对于提供前端页面、CSS、JavaScript 文件非常有用。
        // **重要**: 静态文件服务作为 fallback，会处理所有未被前面更具体路由捕获的路径。
        .fallback_service(ServeDir::new("static"))
}
