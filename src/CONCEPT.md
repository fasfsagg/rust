# `src` 模块核心概念 (`main`, `startup`, `config`, `db`, `error`, `routes`)

本文档总结了 `src` 目录下顶层模块的核心概念。

## 1. 应用入口与启动 (`main.rs`, `startup.rs`)

*   **入口点 (`main.rs`)**:
    *   **`main` 函数**: Rust 程序执行的起点。
    *   **`#[tokio::main]`**: 使用 Tokio 异步运行时，允许 `main` 函数执行 `async/await`。
    *   **模块声明**: 使用 `mod` 关键字引入其他模块 (`app`, `config`, `db`, `error`, `routes`, `startup`)，定义项目结构。
    *   **执行顺序**:
        1.  加载配置 (`config::AppConfig::from_env`)。
        2.  初始化应用 (`startup::init_app`)。
        3.  绑定 TCP 监听地址 (`TcpListener::bind`)。
        4.  启动 Axum 服务器 (`axum::serve`)。
*   **启动协调器 (`startup.rs`)**:
    *   **`init_app` 函数**: 被 `main` 调用，负责集中执行初始化任务。
    *   **日志初始化**: 调用 `middleware::setup_logger` 设置 `tracing` 日志系统。
    *   **数据库初始化**: 调用 `db::new_db` 创建数据库实例 (内存 `HashMap`)，并可选地填充示例数据 (`db::init_sample_data`)。
    *   **状态创建**: 创建 `AppState` (包含数据库实例 `Db`)。
    *   **中间件配置**: 使用 `tower::ServiceBuilder` 组合中间件层 (`CorsLayer`, `TraceLayer`)。
    *   **路由创建**: 调用 `routes::create_routes` 并传入 `AppState`。
    *   **应用中间件**: 将中间件栈应用到整个路由 (`.layer(middleware_stack)`)。
    *   **返回就绪 `Router`**: 将配置好的 `Router` 返回给 `main` 函数。

## 2. 配置管理 (`config.rs`)

*   **`AppConfig` 结构体**: 定义应用程序所需的所有配置参数 (e.g., `http_addr`, `ws_ping_interval`)，提供类型安全。
*   **环境变量加载 (`from_env`)**: 从环境变量读取配置值，如果未设置则使用默认值。
*   **类型转换与验证**: 将字符串环境变量解析为目标类型 (e.g., `SocketAddr`, `u64`)，并在失败时 panic (通过 `.expect`)。
*   **默认值**: 为配置项提供硬编码的默认值。

## 3. 数据访问 (`db.rs`) - 内存模拟

*   **模拟数据库**: 使用内存中的 `HashMap` 存储 `Task` 数据，简化项目设置。
*   **线程安全核心类型 (`Db = Arc<RwLock<HashMap<Uuid, Task>>>`)**:
    *   `HashMap<Uuid, Task>`: 实际存储数据，本身非线程安全。
    *   `parking_lot::RwLock`: 读写锁，允许多读或单写，保护 `HashMap` 的并发访问。
    *   `Arc`: 原子引用计数，允许多个线程安全地共享同一个 `RwLock<HashMap>`。
*   **CRUD 接口**: 提供 `create_task`, `get_all_tasks`, `get_task_by_id`, `update_task`, `delete_task` 等公共函数，封装了获取锁和操作 `HashMap` 的逻辑。
*   **错误处理**: 函数返回 `Result<T, AppError>`，在找不到任务时返回 `AppError::NotFound`。
*   **克隆**: 读取操作 (`get_all`, `get_by_id`) 返回数据的**克隆**副本，因为不能将锁内部数据的所有权移出。`delete_task` 返回被移除值的**所有权**。
*   **RAII**: 锁守卫 (`RwLockReadGuard`, `RwLockWriteGuard`) 利用 RAII 模式确保锁在离开作用域时自动释放。

## 4. 错误处理 (`error.rs`)

*   **`AppError` 枚举**: 定义统一的应用错误类型 (e.g., `NotFound`, `BadRequest`, `InternalServerError`, `Conflict`)，每个变体包含错误消息。
*   **`IntoResponse` 实现**: 使 `AppError` 可以被 Axum 自动转换为 HTTP 响应。
    *   根据错误变体映射到合适的 `StatusCode`。
    *   构造包含 `message` 和 `code` 的 JSON 错误响应体。
*   **`Result<T>` 别名**: `type Result<T> = std::result::Result<T, AppError>;` 简化函数签名。
*   **辅助函数**: 提供 `task_not_found`, `invalid_uuid` 等便捷函数来创建特定的 `AppError` 实例。

## 5. 路由定义 (`routes.rs`)

*   **`create_routes` 函数**: 创建并返回配置好的 Axum `Router`。
*   **`Router`**: Axum 核心类型，用于定义 URL 路径与处理函数的映射关系。
*   **HTTP 方法映射**: 使用 `.route()` 和 `routing::{get, post, put, delete}` 将特定路径和 HTTP 方法绑定到 `controller` 中的处理函数。
*   **路径参数**: 使用 `:id` 定义动态路径段。
*   **状态注入 (`.with_state(app_state)`)**: 将 `AppState` 注入到需要访问共享状态 (如数据库) 的路由处理函数中。
*   **中间件应用**: 可以通过 `.layer()` 将中间件应用到特定路由或整个 `Router` (虽然主要在 `startup.rs` 中应用)。
*   **路由组织**: 使用 `.nest()` (挂载子路由到前缀下) 和 `.merge()` (合并路由) 来组织路由结构。
*   **静态文件服务**: 使用 `.nest_service(\"/\", ServeDir::new(\"public\"))` 将 `/` 路径映射到 `public` 目录，用于托管静态文件。
*   **WebSocket 路由**: 定义 `/ws` 路径并将其路由到 `ws_handler`。
*   **测试**: 包含使用 `#[cfg(test)]` 和 `tower::ServiceExt::oneshot` 进行基本路由测试的代码。 