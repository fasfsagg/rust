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
    *   **数据库连接与迁移**:
        *   调用 `db::establish_connection` 使用配置的 `DATABASE_URL` 建立数据库连接池。
        *   调用 `db::run_migrations` 确保数据库表（如 `users`, `tasks`）已根据实体定义创建。
    *   **状态创建**: 创建 `AppState` (包含数据库连接 `DatabaseConnection` 和应用配置 `AppConfig`)。
    *   **中间件配置**: 使用 `tower::ServiceBuilder` 组合中间件层 (`CorsLayer`, `TraceLayer`)。
    *   **路由创建**: 调用 `routes::create_routes` 并传入 `AppState`。
    *   **应用中间件**: 将中间件栈应用到整个路由 (`.layer(middleware_stack)`)。
    *   **返回就绪 `Router`**: 将配置好的 `Router` 返回给 `main` 函数。

## 2. 配置管理 (`config.rs`)

*   **`AppConfig` 结构体**: 定义应用程序所需的所有配置参数 (e.g., `http_addr`, `jwt_secret`, `jwt_expiration_seconds`)，提供类型安全。
*   **环境变量加载 (`from_env`)**: 从环境变量读取配置值，如果未设置则使用默认值。
*   **类型转换与验证**: 将字符串环境变量解析为目标类型 (e.g., `SocketAddr`, `u64`)，并在失败时 panic (通过 `.expect`)。
*   **默认值**: 为配置项提供硬编码的默认值。

## 3. 数据访问与ORM (`db.rs`, SeaORM)

### 3.1. `db.rs` 模块
*   **职责**: 管理数据库连接的建立和基础的 schema 设置（通过编程方式的迁移）。
*   **`establish_connection()`**: 异步函数，读取环境变量 `DATABASE_URL`，配置连接选项（如超时、日志级别），并使用 `sea_orm::Database::connect()` 创建一个 `DatabaseConnection`（数据库连接池）。
*   **`run_migrations()`**: 异步函数，接收一个 `DatabaseConnection` 引用。使用 SeaORM 的 `Schema` API（`Schema::new()` 和 `create_table_from_entity()`) 以编程方式创建数据库表（例如 `users` 和 `tasks` 表）。此函数确保应用启动时表存在。
    *   **注意**: 对于复杂的数据库 schema 演变和版本控制，通常推荐使用 `sea-orm-cli` 生成和管理专门的迁移文件，而不是仅依赖 `create_table_from_entity`。

### 3.2. SeaORM 核心概念
*   **角色**: SeaORM 是一个 Rust 的异步动态 ORM (Object Relational Mapper)，旨在提供一种类型安全且符合人体工程学的方式与数据库交互。
*   **主要组件**:
    *   **`DatabaseConnection`**: 代表一个数据库连接池，是执行所有数据库操作的句柄。
    *   **实体 (Entity)**: 例如 `user::Entity` 和 `task::Entity`。通过 `#[derive(DeriveEntityModel)]` 宏根据定义的 `Model` 结构体生成。实体描述了数据库表的结构、列、主键和关系。它是与数据库表结构的一对一映射。
    *   **模型 (Model)**: 例如 `user::Model` 和 `task::Model`。这些是普通的 Rust 结构体，代表从数据库中检索到的一行数据。它们通常派生 `Serialize` 和 `Deserialize` 以便在 API 中使用。
    *   **活动模型 (ActiveModel)**: 例如 `user::ActiveModel` 和 `task::ActiveModel`。这些结构体用于创建新记录或更新现有记录。它们允许部分更新，只设置需要更改的字段。`ActiveModel` 实现了 `ActiveModelTrait`，提供了如 `insert()`, `update()`, `delete()` 等方法。
    *   **关系 (Relation)**: 在实体定义中声明，用于描述表之间的连接（如一对多、多对多）。本项目中 `Task` 和 `User` 之间尚未建立显式关系，但 `Relation` 枚举已为未来扩展准备好。
    *   **迁移 (Migrations)**: SeaORM 支持通过 `sea-orm-cli` 工具进行结构化的数据库迁移，允许版本控制和逐步演变数据库 schema。本项目中的 `run_migrations` 函数提供了一种更简单的、基于实体定义的表创建方式，适用于开发或简单场景。
*   **在项目中的使用**:
    *   `User` 和 `Task` 数据都通过 SeaORM 实体进行管理。
    *   服务层 (`AuthService`, `TaskService`) 使用 `DatabaseConnection` 和相应的实体/活动模型来执行 CRUD (创建、读取、更新、删除) 操作。
    *   提供了类型安全的数据库查询和操作，减少了编写原始 SQL 的需要。

## 4. JSON Web Tokens (JWT) 认证

*   **结构**: JWT 由三部分组成，通过 `.` 分隔：
    1.  **头部 (Header)**: 通常包含类型 (JWT) 和所使用的签名算法 (如 HS512)。Base64Url 编码。
    2.  **载荷 (Payload)**: 包含一组声明 (Claims)。声明是关于实体（通常是用户）和附加数据的语句。Base64Url 编码。
        *   **标准声明**: 如 `sub` (Subject，通常是用户ID)，`exp` (Expiration Time，令牌过期时间戳)，`iat` (Issued At，令牌签发时间戳)。
        *   **自定义声明**: 项目中使用了 `username` 和 `company` (示例) 作为自定义声明。
        *   这些声明在 `src/app/model/user.rs` 中的 `Claims` 结构体中定义。
    3.  **签名 (Signature)**: 用于验证消息在传递过程中没有被篡改。通过对编码后的头部、编码后的载荷、一个密钥 (secret) 使用指定的签名算法生成。
*   **用途**: 实现无状态认证。服务器在用户成功登录后生成并发送 JWT 给客户端。客户端在后续请求中通过 HTTP `Authorization` 头部 (通常使用 `Bearer` 方案) 将 JWT 发回服务器。服务器验证 JWT 的签名和有效性（如过期时间）来认证用户，无需在服务器端存储会话状态。
*   **签名过程**:
    *   使用 `jsonwebtoken` crate。
    *   `encode` 函数接收头部、`Claims` 和一个 `EncodingKey` (由 `AppConfig.jwt_secret` 生成)。
    *   本项目使用 HS512 算法。
*   **验证过程**:
    *   `decode` 函数接收 JWT 字符串、一个 `DecodingKey` (由 `AppConfig.jwt_secret` 生成) 和验证选项 (指定算法、是否检查过期等)。
    *   如果签名有效且声明通过验证 (如未过期)，则解码成功。

## 5. Axum 中间件与认证实现

*   **中间件概念**: 中间件是在请求到达最终的业务逻辑处理函数之前（或响应返回给客户端之前）可以执行的一段代码。它可以检查请求、修改请求/响应、实现横切关注点（如日志、CORS、认证）。
*   **`FromRequestParts` 提取器**: Axum 提供的一种强大机制，用于从请求的各个部分（如头部、路径、查询参数、状态等）创建自定义提取器。这不仅用于数据提取，也可用于请求的验证和预处理，从而起到类似中间件的作用。
    *   **`Claims::from_request_parts`**: 在 `src/app/middleware/auth_middleware.rs` 中为 `Claims` 类型实现了此 trait。
        1.  它尝试从 `Authorization` HTTP 头部提取 "Bearer Token"。
        2.  如果头部不存在或格式错误，则拒绝请求 (返回 `AppError::Unauthorized`)。
        3.  如果找到 token，它使用 `jsonwebtoken::decode` 和从 `AppState.config` 获取的 `jwt_secret` 来验证和解码 token。
        4.  如果解码失败（token 无效、过期、签名不匹配等），则拒绝请求 (返回 `AppError::Unauthorized`)。
        5.  如果解码成功，则将解析出的 `Claims` 对象返回。
    *   **在处理器中使用**: 当 Axum 路由处理函数的参数包含 `claims: Claims` 时，Axum 会自动调用 `Claims::from_request_parts`。如果此函数返回 `Ok(claims)`，则 `claims` 会被注入到处理器中。如果返回 `Err(rejection)`，则请求处理会提前中止，并将 `rejection` (在这里是 `AppError`) 转换为 HTTP 错误响应。这使得路由级别的认证非常简洁。
*   **`tower::Layer`**: 这是更通用的中间件定义方式，用于创建可应用于整个路由或子路由的中间件。本项目中 `CorsLayer` 和 `TraceLayer` 是通过这种方式应用的。虽然JWT认证主要通过 `FromRequestParts` 实现，但 `Layer` 是理解 Axum 中间件生态系统的重要部分。

## 6. 错误处理 (`error.rs`)

*   **`AppError` 枚举**: 定义统一的应用错误类型 (e.g., `NotFound`, `BadRequest`, `InternalServerError`, `Conflict`, 以及新增的认证和数据库错误如 `UserNotFound`, `InvalidPassword`, `Unauthorized`, `DatabaseError`, `TaskNotFound`)。
*   **`IntoResponse` 实现**: 使 `AppError` 可以被 Axum 自动转换为 HTTP 响应。
    *   根据错误变体映射到合适的 `StatusCode` (例如, `Unauthorized` 映射到 `401 Unauthorized`)。
    *   构造包含 `message` 和 `code` 的 JSON 错误响应体。
*   **`Result<T>` 别名**: `type Result<T> = std::result::Result<T, AppError>;` 简化函数签名。
*   **辅助函数**: 提供 `task_not_found`, `invalid_uuid` 等便捷函数来创建特定的 `AppError` 实例。
*   **`From<sea_orm::DbErr>` for `AppError`**: 允许使用 `?` 操作符将 SeaORM 的 `DbErr` 自动转换为 `AppError::DatabaseError`。

## 7. 路由定义 (`routes.rs`)

*   **`create_routes` 函数**: 创建并返回配置好的 Axum `Router`。
*   **HTTP 方法映射**: 使用 `.route()` 和 `routing::{get, post, put, delete}` 将特定路径和 HTTP 方法绑定到 `controller` 中的处理函数。
*   **路径参数**: 使用 `:id` 定义动态路径段 (例如 `/api/tasks/:id`，其中 `id` 现在是 `i32`)。
*   **状态注入 (`.with_state(app_state.clone())`)**: 将 `AppState` (包含 `DatabaseConnection` 和 `AppConfig`) 注入到路由处理函数中。
*   **受保护路由**: 对于需要认证的路由 (如 `/api/protected_data`)，其处理函数直接接收 `claims: Claims` 作为参数。Axum 会通过 `Claims::from_request_parts` 自动处理认证。
*   **静态文件服务**: 保持不变，使用 `.nest_service("/", ServeDir::new("static"))`。
*   **WebSocket 路由**: 保持不变。

(注意: 原文档中的测试部分已移除，因为测试有专门的 `tests/` 目录和运行方式。)