# `src/app` 模块执行流程图

本文档使用 Mermaid 语法绘制 `app` 模块内部，特别是 Controller, Service, DB 层之间的典型交互流程。

## Controller -> Service -> DB 交互流程 (以创建任务为例)

```mermaid
sequenceDiagram
    participant Controller (task_controller.rs)
    participant Service (task_service.rs)
    participant DB (db.rs)
    participant Model (task.rs)

    Note over Controller: 接收 Axum 请求 (POST /api/tasks)
    Controller->>Model: 解析请求体 JSON 为 CreateTaskPayload
    Controller->>+Service: 调用 service::create_task(&state.db, payload)
    Service->>Model: (内部) 解构 CreateTaskPayload 获取字段
    Service->>+DB: 调用 db::create_task(db, title, desc, completed)
    DB->>Model: (内部) 创建 Task 实例
    Note over DB: 获取写锁, 操作 HashMap.insert
    DB-->>-Service: 返回 Result<Task>
    Service-->>-Controller: 返回 Result<Task>
    Note over Controller: 构建 HTTP 响应 (201 Created + Json(Task) 或错误响应)
```

## Controller -> Service -> DB 交互流程 (以获取任务为例)

```mermaid
sequenceDiagram
    participant Controller (task_controller.rs)
    participant Service (task_service.rs)
    participant DB (db.rs)

    Note over Controller: 接收 Axum 请求 (GET /api/tasks/{id})
    Controller->>+Service: 调用 service::get_task_by_id(&state.db, id)
    Service->>+DB: 调用 db::get_task_by_id(db, id)
    Note over DB: 获取读锁, 操作 HashMap.get
    DB-->>-Service: 返回 Result<Task>
    Service-->>-Controller: 返回 Result<Task>
    Note over Controller: 构建 HTTP 响应 (200 OK + Json(Task) 或 404 Not Found)

```

**注意**: 上述流程图简化了错误处理路径，错误 (`Err(AppError)`) 通常会直接从 DB 或 Service 传递回 Controller，然后由 Axum 通过 `IntoResponse` 处理。 