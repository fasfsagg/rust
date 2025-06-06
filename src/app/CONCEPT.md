# `src/app` 模块核心概念

本文档总结了 `src/app` 模块及其子模块的核心概念，重点关注其在分层架构中的职责。

## 1. `app` 模块 (`mod.rs`)

*   **核心逻辑聚合**: 作为包含应用程序核心业务逻辑的根模块。
*   **子模块声明**: 声明 `controller`, `service`, `model`, `middleware`, `state` 等子模块，定义 `app` 内部结构。
*   **`AppState` 重新导出**: 从 `state` 模块重新导出 `AppState`，使其可以通过 `crate::app::AppState` 访问。

## 2. 应用状态 (`state.rs`)
*   **`AppState` 结构体**:
    *   定义在 `src/app/state.rs`。
    *   集中管理整个应用程序的共享状态。
    *   包含数据库连接池 (`db: DatabaseConnection`) 和应用配置 (`config: AppConfig`)。
    *   派生 `Clone`，因为 Axum 的状态提取器要求状态是可克隆的。`DatabaseConnection` (内部是 `Arc`) 和 `AppConfig` (已派生 `Clone`) 都支持克隆。
    *   通过 Axum 的 `State` 提取器注入到需要它的处理器和中间件（如 `FromRequestParts` 实现）中。

## 3. 模型层 (`model/`)

*   **`model/mod.rs`**: 声明并重新导出 (`pub use`) 模型子模块（如 `user.rs`, `task.rs`）中的公共项，包括实体、模型、活动模型以及相关的载荷结构体。
*   **SeaORM 实体**:
    *   **`model/user.rs`**: 定义 `user::Entity`，映射到 `users` 数据库表。包含 `id`, `username`, `password_hash` 字段。
    *   **`model/task.rs`**: 定义 `task::Entity`，映射到 `tasks` 数据库表。包含 `id`, `title`, `description`, `completed`, `created_at`, `updated_at` 字段。
    *   每个实体文件都包含 `Model` (用于查询结果)、`ActiveModel` (用于插入/更新) 和 `Relation` (用于表关系，即使当前为空) 的定义。
    *   `ActiveModelBehavior` 被实现用来自动处理时间戳（如 `created_at`, `updated_at`）。
*   **JWT Claims**:
    *   **`model/user.rs`**: 定义 `Claims` 结构体，用于 JWT 的载荷。包含 `sub` (用户ID), `username`, `exp` (过期时间) 等字段。派生 `Serialize` 和 `Deserialize`。
*   **请求载荷 (Payloads)**:
    *   定义用于 API 请求体验证和反序列化的结构体，例如：
        *   `RegisterUserPayload`, `LoginUserPayload` (在 `model/user.rs` 中)
        *   `CreateTaskPayload`, `UpdateTaskPayload` (在 `model/task.rs` 中)
    *   使用 `serde::Deserialize` 进行解析。`UpdateTaskPayload` 可能使用如 `double_option` 的自定义逻辑来处理可选字段的更新。

## 4. 服务层 (`service/`)

*   **`service/mod.rs`**: 声明并重新导出服务子模块 (`auth_service.rs`, `task_service.rs`) 中的公共项 (主要是服务结构体或函数)。
*   **`service/auth_service.rs`**:
    *   **`AuthService`**: 实现用户认证相关的业务逻辑。
    *   **用户注册 (`register_user`)**: 接收注册载荷，检查用户名是否存在，对密码进行 `argon2` 哈希处理，然后使用 `user::ActiveModel` 将新用户插入数据库。
    *   **用户登录 (`login_user`)**: 接收登录载荷，从数据库查询用户，验证密码哈希，如果成功则使用 `jsonwebtoken::encode` 生成 JWT。JWT 的密钥和有效期从 `AppConfig` 获取。
*   **`service/task_service.rs`**:
    *   **任务管理函数**: 实现任务的 CRUD (创建、读取、更新、删除) 业务逻辑。
    *   **SeaORM 交互**: 所有函数都接收 `&DatabaseConnection`，并使用 `task::Entity` 和 `task::ActiveModel` 来执行数据库操作 (如 `find()`, `find_by_id()`, `insert()`, `update()`, `delete()`)。
    *   **错误处理**: 返回 `Result<T, AppError>`，处理或传递来自 SeaORM 的 `DbErr` (通过 `From` trait 转换为 `AppError::DatabaseError`) 以及业务逻辑错误 (如 `TaskNotFound`)。

## 5. 控制器层 (`controller/`)

*   **`controller/mod.rs`**: 声明并重新导出控制器子模块 (`auth_controller.rs`, `task_controller.rs`) 中的公共项 (主要是 Handler 函数)。也重新导出 `AppState` 以方便 `routes.rs` 使用。
*   **`controller/auth_controller.rs`**:
    *   **`register_handler`**: 处理 `/api/register` POST 请求。接收 `RegisterUserPayload`，调用 `AuthService::register_user`，返回新用户信息或错误。
    *   **`login_handler`**: 处理 `/api/login` POST 请求。接收 `LoginUserPayload`，调用 `AuthService::login_user`，返回 JWT 或错误。
*   **`controller/task_controller.rs`**:
    *   **任务 CRUD Handlers**: 实现与任务相关的 HTTP 请求处理函数 (如 `create_task`, `get_all_tasks`, `get_task_by_id`, `update_task`, `delete_task`)。
    *   **请求处理**: 使用 Axum 提取器 (`State<AppState>`, `Path<i32>` for task IDs, `Json<PayloadStruct>`)。
    *   **调用服务层**: 调用 `task_service` 中的相应函数执行业务逻辑。
    *   **响应构建**: 根据服务层返回的结果构建 HTTP 响应，成功时返回 JSON 数据和合适的 HTTP 状态码，失败时返回 `AppError`。
*   **受保护路由处理**:
    *   例如 `protected_data_handler`，在其参数中直接声明 `claims: Claims`。Axum 会自动使用 `Claims::from_request_parts` 提取器来验证 JWT 并注入 `Claims` 数据。如果验证失败，提取器会提前返回错误响应。

## 6. 中间件层 (`middleware/`)

*   **`middleware/mod.rs`**: 声明并重新导出中间件子模块 (`logger.rs`, `auth_middleware.rs`)。
*   **`middleware/logger.rs`**: 提供日志和追踪功能。
    *   **`setup_logger`**: 初始化全局 `tracing` 日志系统。
    *   **`trace_layer`**: 创建 `tower_http::trace::TraceLayer` 中间件，用于记录 HTTP 请求/响应信息。
*   **`middleware/auth_middleware.rs`**:
    *   **JWT 认证核心**: 主要通过为 `model::user::Claims` 实现 `FromRequestParts<AppState>` trait 来集成 JWT 认证。
    *   **`Claims::from_request_parts`**:
        1.  从 `AppState` 获取 `AppConfig` 以得到 JWT 密钥。
        2.  从请求的 `Authorization` 头部提取 `Bearer <token>`。
        3.  使用 `jsonwebtoken::decode` 对 token 进行解码和验证 (算法、签名、有效期)。
        4.  成功则返回 `Ok(Claims)`，供处理器函数使用。
        5.  失败则返回 `Err(AppError::Unauthorized)`，中断请求并发送 401 响应。
    *   这种方式使得在需要认证的路由处理器中，只需将 `Claims` 作为参数即可自动执行认证逻辑。