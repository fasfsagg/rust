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
    ws_handler, // 处理 GET /ws
};
// 导入在 `src/startup.rs` 中定义的唯一的共享应用状态 `AppState`。
use crate::startup::AppState;

// --- 路由创建函数 ---

/// 创建并配置应用程序的所有路由 (Function to Create Application Routes)
///
/// 【功能】: 集中定义应用程序的 URL 结构，并将每个 URL 路径 + HTTP 方法组合映射到相应的控制器处理函数。
///          同时，配置状态共享和静态文件服务。
///
/// # 【参数】
/// * `app_state: AppState` - 应用程序的共享状态。[[所有权: 移动]]
///                           它通常包含数据库连接池 (`Db`) 或其他需要在多个请求处理函数之间共享的资源。
///                           这个 `AppState` 会被注入到需要它的路由处理函数中。
///
/// # 【返回值】
/// * `-> Router`: 返回一个完全配置好的 `axum::Router` 实例。
///                这个 `Router` 实例随后会被传递给 `axum::serve` 来启动服务器。
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
        // 定义 GET /tasks/:id 路由，映射到 get_task_by_id 控制器函数。
        // `:id` 是一个【路径参数】(Path Parameter)。[[Axum 功能: 路径参数]]
        // Axum 会自动解析 URL 中的这部分，并通过 `axum::extract::Path` 提取器将其传递给处理函数。
        .route("/tasks/:id", get(get_task_by_id))
        // 定义 PUT /tasks/:id 路由，映射到 update_task 控制器函数。
        .route("/tasks/:id", put(update_task))
        // 定义 DELETE /tasks/:id 路由，映射到 delete_task 控制器函数。
        .route("/tasks/:id", delete(delete_task))
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
        .with_state(app_state); // 如果 ws_handler 需要 AppState，也注入

    // --- 组合所有路由 ---
    // 创建最终的根路由，并将上面定义的子路由和静态文件服务组合起来。
    Router::new()
        // `.nest("/api", api_routes)`: 将 `api_routes` 下定义的所有路由挂载到 `/api` 路径前缀下。
        // 例如，之前定义的 `/tasks` 会变成 `/api/tasks`。
        // 这有助于组织路由，将所有 API 相关端点归类。
        .nest("/api", api_routes)
        // `.merge(ws_routes)`: 将 `ws_routes` 定义的路由合并到当前路由层级。
        // 这里 `/ws` 路由仍然是根路径下的 `/ws`。
        .merge(ws_routes)
        // `.nest_service("/", ServeDir::new("static"))`: 配置静态文件服务。
        //   - `"/"`: 匹配根路径及其下的所有子路径（如果未被前面的路由匹配）。
        //   - `ServeDir::new("static")`: 创建一个服务，它会查找并返回 `static` 目录下对应的文件。
        //   - `nest_service`: 将一个 `Service` (实现了 Tower 的 `Service` trait) 挂载到指定的路径下。
        //   【效果】: 当请求 `http://localhost:3000/` 时，会返回 `static/index.html`。
        //           当请求 `http://localhost:3000/styles.css` 时，会返回 `static/styles.css`。
        //           这对于提供前端页面、CSS、JavaScript 文件非常有用。
        // **重要**: 静态文件服务通常放在路由定义的【最后】，因为它会匹配所有未被前面更具体路由捕获的路径。
        .nest_service("/", ServeDir::new("static"))
}
