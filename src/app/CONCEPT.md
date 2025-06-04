# `src/app` 模块核心概念

本文档总结了 `src/app` 模块及其子模块的核心概念，重点关注其在分层架构中的职责。

## 1. `app` 模块 (`mod.rs`)

*   **核心逻辑聚合**: 作为包含应用程序核心业务逻辑的根模块。
*   **子模块声明**: 声明 `controller`, `service`, `model`, `middleware` 等子模块，定义 `app` 内部结构。
*   **无重新导出**: 有意不重新导出子模块内容，以保持依赖路径的明确性，反映分层结构。

## 2. 模型层 (`model/`)

*   **`model/mod.rs`**: 声明并重新导出 (`pub use`) 模型子模块（如 `task.rs`）中的公共项。
*   **`model/task.rs`**: 定义核心数据结构 (`Task` 实体) 和相关的请求/响应载荷 (`CreateTaskPayload`, `UpdateTaskPayload`)。
    *   **`Task` 结构体**: 包含任务的属性 (id, title, description, completed, created_at, updated_at)。使用 `serde::{Serialize, Deserialize}` 进行 JSON 序列化/反序列化，`Clone`, `Debug` 等派生宏。
    *   **Payload 结构体**: 定义 API 请求体的数据格式，使用 `serde::Deserialize`。
    *   **数据形态**: 定义数据在系统不同部分流转时的结构。

## 3. 服务层 (`service/`)

*   **`service/mod.rs`**: 声明并重新导出 (`pub use`) 服务子模块（如 `task_service.rs`）中的公共项。
*   **`service/task_service.rs`**: 实现与任务相关的业务逻辑。
    *   **业务规则封装**: （理想情况下）包含数据验证、权限检查、流程控制、事务管理等。
    *   **协调者**: 作为 Controller 和 DB 层之间的桥梁。
    *   **调用 DB**: 调用 `db.rs` 中的函数执行数据持久化操作。
    *   **异步接口**: 提供 `async fn` 接口给 Controller 层。
    *   **错误处理**: 返回 `Result<T, AppError>`，处理或传递来自 DB 层的错误。
    *   *(当前实现较薄，主要直接调用 DB)*

## 4. 控制器层 (`controller/`)

*   **`controller/mod.rs`**: 声明并重新导出 (`pub use`) 控制器子模块（如 `task_controller.rs`）中的公共项 (主要是 Handler 函数和 `AppState`)。
*   **`controller/task_controller.rs`**: 实现具体的 HTTP 请求处理函数 (Handlers)。
    *   **请求处理**: 接收 Axum 传递的请求数据（路径参数 `Path`, 请求体 `Json`, 状态 `State`）。
    *   **调用 Service**: 调用 `service` 层相应的函数来执行业务逻辑。
    *   **响应构建**: 根据 Service 层返回的 `Result` 构建 HTTP 响应。
        *   成功 (`Ok(data)`) 时，通常返回 `Json(data)` (200 OK 或 201 Created)。
        *   失败 (`Err(AppError)`) 时，直接返回错误，Axum 会通过 `AppError` 的 `IntoResponse` 实现将其转换为适当的 HTTP 错误响应。
    *   **`AppState`**: 包含共享状态（如数据库实例 `Db`），通过 Axum 的状态注入机制 (`State<AppState>`) 访问。
    *   **WebSocket 处理 (`ws_handler`)**: 处理 WebSocket 升级请求和后续的消息交互。

## 5. 中间件层 (`middleware/`)

*   **`middleware/mod.rs`**: 声明并重新导出 (`pub use`) 中间件子模块（如 `logger.rs`）。
*   **`middleware/logger.rs`**: 提供日志和追踪功能。
    *   **`setup_logger`**: 初始化全局 `tracing` 日志系统，使用 `tracing_subscriber` 和 `EnvFilter` 配置日志级别和输出。
    *   **`trace_layer`**: 创建 `tower_http::trace::TraceLayer` 中间件，用于自动记录 HTTP 请求/响应的详细信息。 