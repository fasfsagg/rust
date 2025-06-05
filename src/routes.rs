// 文件路径: src/routes.rs

// /--------------------------------------------------------------------------------------------------\
// |                                   【模块功能图示】 (routes.rs)                                     |
// |--------------------------------------------------------------------------------------------------|
// |                                                                                                  |
// |  [startup.rs - `init_app` 函数]                                                                    |
// |      | 调用 `create_routes(app_state)`                                                            |
// |      V                                                                                           |
// |  [`create_routes` 函数 (本模块)]                                                                   |
// |   1. 定义 `auth_routes` (公共认证路由)                                                             |
// |      - `/api/register` -> `auth_controller::register_handler`                                    |
// |      - `/api/login`    -> `auth_controller::login_handler`                                     |
// |   2. 定义 `protected_routes` (受保护路由)                                                          |
// |      - `/api/protected_data` -> `protected_controller::protected_data_handler`                   |
// |      - 应用 `jwt_auth_middleware` 到此组路由 (`route_layer`)                                      |
// |   3. 合并路由 (`Router::new().nest("/api", auth_routes.merge(protected_routes))`)                 |
// |      - 将所有 `/api/...` 路由组合在 `/api` 路径前缀下。                                           |
// |   4. 应用全局应用状态 (`.with_state(app_state)`)                                                   |
// |      - 使 `app_state` (包含数据库连接池 `Arc<DatabaseConnection>`) 对所有 Handler 可用。            |
// |   5. (可选) 配置静态文件服务 (`.nest_service("/", ServeDir::new("static"))`)                      |
// |      |                                                                                           |
// |      V                                                                                           |
// |  [返回主 `Router` 给 `startup.rs`]                                                                 |
// |                                                                                                  |
// \--------------------------------------------------------------------------------------------------/
//
// 【模块核心职责】 (Module Core Responsibilities)
// 1. **集中定义 API 路由 (Centralize API Route Definitions)**:
//    此模块是应用程序所有 HTTP API 端点 (endpoints) 的“交通枢纽”。它清晰地将特定的 URL 路径 (paths) 和 HTTP 方法 (methods)
//    映射到对应的请求处理函数 (handler functions)，这些处理函数通常位于控制器模块 (`app/controller/`) 中。
// 2. **组织路由结构 (Organize Route Structure)**:
//    - 使用 Axum 的 `Router` 功能 (如 `.nest()` 用于路径前缀分组, `.merge()` 用于合并不同的路由组) 来构建一个结构清晰、
//      易于管理的路由体系。例如，所有与认证相关的公共接口和受保护的业务接口都组织在 `/api` 前缀下。
// 3. **应用中间件 (Apply Middleware)**:
//    - 将中间件（Middleware）应用于特定的路由或路由组。例如，`jwt_auth_middleware` 被应用于所有需要认证的受保护路由组。
//    - 中间件用于处理横切关注点，如认证、日志、CORS 等。
// 4. **注入应用状态 (Inject Application State)**:
//    - 将应用程序的共享状态 (`AppState`，其中包含如数据库连接池等资源) 注入到整个路由系统中，
//      使得各个请求处理函数可以通过 Axum 的 `State` 提取器来访问这些共享资源。
// 5. **(可选) 配置静态文件服务 (Configure Static File Serving)**:
//    - 可以配置 Axum 来提供静态文件服务 (如 HTML, CSS, JavaScript, 图片)，通常用于托管单页应用 (SPA) 的前端文件或项目的静态资源。
//
// 【关键技术点】 (Key Technologies)
// - **Axum `Router`**: Axum 框架的核心组件，用于构建路由树。
//   - `.route(path: &str, method_router: MethodRouter)`: 定义一个特定路径和 HTTP 方法的路由，并将其映射到一个处理服务 (通常是一个 handler 函数)。
//   - `routing::{get, post}`: Axum 提供的函数，用于创建特定 HTTP 方法 (GET, POST 等) 的 `MethodRouter`，它们接收一个 handler 函数作为参数。
//   - `.merge(other: Router)`: 将另一个 `Router` 实例中的所有路由合并到当前 `Router` 中。有助于模块化路由定义。
//   - `.nest(path: &str, router: Router)`: 将一个完整的 `Router` 实例挂载到一个指定的路径前缀下。例如，所有 `api_routes` 都挂载在 `/api` 之下。
//   - `.layer(middleware_layer: L)`: 将一个中间件层 (`L` 必须实现 `tower::Layer`) 应用到当前 `Router` 及其所有子路由和处理函数。
//     (全局中间件通常在 `startup.rs` 中通过此方法应用到最顶层的 `Router`)。
//   - `.route_layer(middleware_layer: L)`: 将一个中间件层专门应用于当前 `Router` 实例中定义的所有路由。
//     这对于给一组特定的路由应用中间件（如认证中间件）非常有用，而不会影响到其他 `Router` 或通过 `.merge()` 合并进来的路由。
//   - `.with_state(state: S)`: 将一个实现了 `Clone` 的状态 `S` (本项目中是 `AppState`) 关联到 `Router`。
//     所有此 `Router`（及其子路由）下的处理函数都可以通过 `axum::extract::State<S>` 来访问这个状态的克隆副本。
// - **`AppState`**: 在 `src/app/state.rs` 中定义的共享应用状态，包含数据库连接池 (`Arc<DatabaseConnection>`)。
// - **处理函数 (Handler Functions)**: 位于 `app/controller/` 目录下的异步函数，负责处理具体的 HTTP 请求。
// - **中间件函数 (Middleware Functions)**: 如 `jwt_auth_middleware`，用于在请求到达处理函数之前或响应返回客户端之前执行通用逻辑。
//   - `axum::middleware::from_fn`: 一个辅助函数，用于将一个符合特定签名（如 `async fn(Request, Next) -> Result<Response, AppError>`）的异步函数适配成一个可以被 `.layer()` 或 `.route_layer()` 使用的中间件层。
// - **静态文件服务 (`tower_http::services::ServeDir`)**: `tower-http` 库提供的服务，用于从文件系统目录中提供静态文件。

// --- 导入依赖 ---
// `use axum::{...}`: 从 `axum` crate 中导入所需的组件。
use axum::{
    routing::{get, post}, // `get` 和 `post` 是创建特定 HTTP 方法路由的辅助函数。从 `axum::routing` 模块导入。
                          // `put` 和 `delete` 在当前路由配置中未使用，因此注释掉或移除以保持整洁。
    middleware::from_fn,  // `from_fn` 是一个非常重要的函数，用于将一个普通的异步函数（符合特定签名）转换成一个 Axum 中间件层 (Layer)。
                          // 这使得我们可以用更简洁的方式编写中间件逻辑。
    Router,               // `Router` 是 Axum 的核心类型，用于定义和组织路由规则。
};
// `use tower_http::services::ServeDir;`
//   - 从 `tower_http` crate (一个提供常用 HTTP 中间件和服务的库) 的 `services` 模块中导入 `ServeDir` 服务。
//   - `ServeDir` 用于从文件系统的某个目录（例如 "static" 或 "public"）提供静态文件服务 (如 HTML, CSS, JavaScript, 图片)。
use tower_http::services::ServeDir;

// --- 导入项目内部的控制器层和状态/中间件组件 ---
// `crate::` 前缀表示从当前项目的根模块开始的路径。

// `use crate::app::controller::auth_controller::{register_handler, login_handler};`
//   - 从 `src/app/controller/auth_controller.rs` 文件中导入 `register_handler` 和 `login_handler` 这两个请求处理函数。
//   - 这些函数分别负责处理用户注册和登录的 API 请求。
use crate::app::controller::auth_controller::{register_handler, login_handler};
// `use crate::app::controller::protected_controller::protected_data_handler;`
//   - 从 `src/app/controller/protected_controller.rs` 文件中导入 `protected_data_handler` 函数。
//   - 这个函数处理访问受保护资源的 API 请求，需要用户通过 JWT 认证。
use crate::app::controller::protected_controller::protected_data_handler;
// `use crate::app::state::AppState;`
//   - 导入在 `src/app/state.rs` 中定义的 `AppState` 结构体。
//   - `AppState` 包含了需要在整个应用中共享的数据，主要是数据库连接池 (`Arc<DatabaseConnection>`)。
//   它将被注入到路由中，供所有处理函数访问。
use crate::app::state::AppState;
// `use crate::app::middleware::jwt_auth_middleware;`
//   - 导入在 `src/app/middleware/auth_middleware.rs` 中定义的 `jwt_auth_middleware` 函数。
//   - 这是一个 JWT 认证中间件，用于保护需要用户登录才能访问的路由。
use crate::app::middleware::jwt_auth_middleware;

// --- 路由创建函数 ---

// `pub fn create_routes(app_state: AppState) -> Router`
//   - `pub fn`: 定义一个公共函数 `create_routes`，可以被其他模块（主要是 `startup.rs`）调用。
//   - `app_state: AppState`: 此函数接收一个 `AppState` 类型的参数 `app_state`。
//     - `AppState` 结构体本身派生了 `Clone` trait。当 `startup.rs` 调用此函数时，
//       它传递 `app_state.clone()`。这意味着 `create_routes` 函数获得了 `AppState` 的一个克隆副本的所有权。
//     - 这个 `app_state` 实例包含了应用范围的共享数据，如 `Arc<DatabaseConnection>`。
//   - `-> Router`: 此函数返回一个配置好的 `axum::Router` 实例。
//     这个返回的 `Router` 将是应用的主路由，包含了所有定义的端点、中间件和状态。
/// 创建并配置应用程序的所有路由。
///
/// 此函数负责将不同的 API 端点（URL 路径 + HTTP 方法）映射到对应的控制器处理函数。
/// 它还负责组织路由结构（例如，将相关的 API 归类到 `/api` 前缀下），
/// 并为需要认证的路由组应用 JWT 认证中间件。
/// 最终，它将共享的应用状态 (`AppState`) 注入到整个路由系统中。
///
/// # 参数
/// * `app_state: AppState` - 应用程序的共享状态实例，包含了如数据库连接池等资源。
///                           此函数会取得 `app_state` 的所有权 (通过克隆传递)。
///
/// # 返回值
/// * `Router` - 一个完全配置好的 `axum::Router` 实例，准备好被服务器用来处理请求。
pub fn create_routes(app_state: AppState) -> Router {
    // --- 定义公开的认证 API 路由 (`auth_routes`) ---
    // 这些路由用于用户注册和登录，因此它们是公开的，不需要 JWT 认证。
    // `Router::new()`: 创建一个新的、空的 `Router` 实例，专门用于组织认证相关的路由。
    let auth_routes = Router::new()
        // `.route(path_str, method_router)`: 定义一条路由规则。
        //   - `path_str`: 字符串，表示 URL 路径。
        //   - `method_router`: 一个 `axum::routing::MethodRouter` 实例，它将特定的 HTTP 方法映射到一个处理函数。
        // `.route("/register", post(register_handler))`:
        //   - 当客户端向 `/register` 路径发送 HTTP POST 请求时，
        //   - Axum 会调用 `auth_controller::register_handler` 函数来处理这个请求。
        //   - `post(handler_fn)` 是 `axum::routing::post` 函数的简写，它接收一个处理函数并返回一个 `MethodRouter`。
        .route("/register", post(register_handler))
        // `.route("/login", post(login_handler))`:
        //   - 类似地，将对 `/login` 路径的 HTTP POST 请求路由到 `auth_controller::login_handler` 函数。
        .route("/login", post(login_handler));
        // 注意：此时的 `auth_routes` 还没有应用 `app_state`。状态将在最后统一应用到主路由器上。

    // --- 定义受保护的 API 路由 (`protected_routes`) ---
    // 这些路由用于访问需要用户认证后才能操作的资源。
    // `Router::new()`: 创建一个新的、空的 `Router` 实例，用于组织受保护的路由。
    let protected_routes = Router::new()
        // `.route("/protected_data", get(protected_data_handler))`:
        //   - 将对 `/protected_data` 路径的 HTTP GET 请求路由到 `protected_controller::protected_data_handler` 函数。
        .route("/protected_data", get(protected_data_handler))
        // **`.route_layer(middleware_layer)`**: 这是将中间件应用于特定路由组的关键方法。
        //   - `.route_layer()` 将提供的中间件层 `middleware_layer` 应用到当前 `protected_routes` 这个 `Router` 实例中定义的所有路由上。
        //     这意味着，任何对 `/protected_data` (或其他将来可能添加到 `protected_routes` 中的路由) 的请求，
        //     在到达其处理函数之前，都会先经过 `jwt_auth_middleware` 的处理。
        //   - `from_fn(jwt_auth_middleware)`:
        //     - `jwt_auth_middleware` 是我们定义的异步函数，其签名符合 Axum 函数式中间件的要求
        //       (例如 `async fn(Request, Next) -> Result<Response, AppError>`)。
        //     - `axum::middleware::from_fn` 是一个辅助函数，它将这样的异步函数包装成一个实现了 `tower::Layer` trait 的类型，
        //       这样它就可以被 Axum 的路由系统用作中间件层。
        .route_layer(from_fn(jwt_auth_middleware));
        // `jwt_auth_middleware` 会检查请求中是否有有效的 JWT。如果认证失败，它会直接返回错误响应，
        // 请求就不会到达 `protected_data_handler`。如果认证成功，它会将 `Claims` 放入请求扩展中。

    // --- 组合所有路由并配置主路由器 ---
    // `Router::new()`: 创建一个新的主 `Router` 实例。
    Router::new()
        // `.nest(path_prefix, sub_router)`: 将一个子路由器 (`sub_router`) 挂载到一个路径前缀 (`path_prefix`) 下。
        //   - `"/api"`: 指定所有后续定义的 API 路由都将以 `/api` 作为其 URL 的起始部分。
        //   - `Router::new().merge(auth_routes).merge(protected_routes)`:
        //     - 这里创建了一个临时的匿名 `Router`。
        //     - `.merge(auth_routes)`: 将 `auth_routes` 中定义的所有路由（`/register`, `/login`）合并到这个临时路由器中。
        //     - `.merge(protected_routes)`: 接着，将 `protected_routes` 中定义的所有路由（`/protected_data` 及其应用的中间件）也合并进来。
        //     - 最终效果是，`/api/register`, `/api/login`, `/api/protected_data` 这些路径都被定义好了。
        //       并且 `/api/protected_data` 路由组依然保留了其 `.route_layer(jwt_auth_middleware)` 配置。
        //     (另一种稍微不同的组织方式可能是先创建 `api_router = Router::new().merge(auth_routes).merge(protected_routes);`
        //      然后再 `.nest("/api", api_router)`)
        .nest("/api",
            Router::new() // 创建一个新的子路由器来组织所有 /api 下的路由
                .merge(auth_routes)      // 合并公共认证路由
                .merge(protected_routes) // 合并受保护的路由 (这些路由已附加了 JWT 中间件)
        )
        // `.with_state(app_state)`: 将 `app_state` (包含了 `Arc<DatabaseConnection>`) 注入到这个主路由器中。
        //   - 这使得所有通过此主路由器（包括其嵌套和合并的子路由器）分派的请求的处理函数，
        //     都能够通过 `State<AppState>` 提取器访问到这份共享的应用状态。
        //   - `app_state` 参数（其所有权之前已移入 `create_routes` 函数）在这里被再次移交所有权给 `Router`。
        //     由于 `AppState` 实现了 `Clone`，Axum 可以在需要时克隆它。
        .with_state(app_state)
        // `.nest_service("/", ServeDir::new("static"))`: 配置静态文件服务。
        //   - `nest_service` 用于将一个实现了 `tower::Service` trait 的服务挂载到指定的路径。
        //   - `"/"`: 表示这个服务将处理所有未被前面更具体的 API 路由 (`/api/...`) 匹配到的、以根路径 `/` 开头的请求。
        //   - `ServeDir::new("static")`: 创建一个 `ServeDir` 服务，它会从项目根目录下的 "static" 文件夹中查找并提供文件。
        //     例如，如果客户端请求 `/index.html`，它会尝试返回 `static/index.html` 文件。
        //     如果请求 `/css/style.css`，它会尝试返回 `static/css/style.css`。
        //   - **重要**: 静态文件服务通常放在路由定义的【最后】，因为它通常作为一种“全匹配 (catch-all)”规则。
        //     如果放在前面，它可能会意外地拦截掉本应由其他 API 路由处理的请求。
        .nest_service("/", ServeDir::new("static"))
}
