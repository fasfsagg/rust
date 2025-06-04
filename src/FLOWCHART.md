# `src` 模块执行流程图

本文档使用 Mermaid 语法绘制应用程序的启动流程和典型 HTTP 请求处理流程。

## 应用启动流程 (`main` -> `startup` -> `axum::serve`)

```mermaid
graph TD
    A[main 函数开始] --> B{加载配置 (config.rs)};
    B --> C{初始化应用 (startup.rs)};
    C --> D[设置日志 (middleware/logger.rs)];
    D --> E[创建 DB 实例 (db.rs)];
    E --> F[初始化示例数据 (db.rs)];
    F --> G[创建 AppState];
    G --> H[配置中间件 (middleware/logger.rs, tower_http)];
    H --> I{创建路由 (routes.rs)};
    I --> J[创建 API 路由];
    J --> K[创建 WS 路由];
    K --> L[合并路由];
    L --> M[配置静态文件服务];
    M --> N[应用中间件到路由];
    N --> O{返回配置好的 Router}; 
    O --> P[绑定 TCP 地址 (main.rs)];
    P --> Q[启动 Axum 服务器 (axum::serve)];
    Q --> R[服务器开始监听请求];
```

## 典型 HTTP API 请求处理流程 (GET /api/tasks/{id})

```mermaid
sequenceDiagram
    participant 客户端 (Client)
    participant Axum服务器 (AxumServer)
    participant 日志中间件 (TraceMiddleware)
    participant 跨域中间件 (CorsMiddleware)
    participant 路由层 (Router - routes.rs)
    participant 处理函数 (Handler - task_controller.rs)
    participant 服务层 (Service - task_service.rs)
    participant 数据访问层 (DB - db.rs)

    客户端 (Client)->>+Axum服务器 (AxumServer): GET /api/tasks/{id}
    Axum服务器 (AxumServer)->>+日志中间件 (TraceMiddleware): 处理请求
    日志中间件 (TraceMiddleware)->>+跨域中间件 (CorsMiddleware): 继续处理 (记录请求开始)
    跨域中间件 (CorsMiddleware)->>+路由层 (Router - routes.rs): 继续处理 (检查 CORS)
    路由层 (Router - routes.rs)->>+处理函数 (Handler - task_controller.rs): 路由匹配, 调用 get_task_by_id(State(state), Path(id))
    处理函数 (Handler - task_controller.rs)->>+服务层 (Service - task_service.rs): 调用 service::get_task_by_id(&state.db, id)
    服务层 (Service - task_service.rs)->>+数据访问层 (DB - db.rs): 调用 db::get_task_by_id(db, id)
    数据访问层 (DB - db.rs)-->>-服务层 (Service - task_service.rs): 返回 Result<Task>
    服务层 (Service - task_service.rs)-->>-处理函数 (Handler - task_controller.rs): 返回 Result<Task>
    处理函数 (Handler - task_controller.rs)-->>-路由层 (Router - routes.rs): 返回 Result<Task>
    路由层 (Router - routes.rs)-->>-跨域中间件 (CorsMiddleware): 返回响应 (Result 转换为 Response)
    跨域中间件 (CorsMiddleware)->>日志中间件 (TraceMiddleware): 继续处理响应 (添加 CORS 头)
    日志中间件 (TraceMiddleware)-->>-Axum服务器 (AxumServer): 继续处理响应 (记录响应结束)
    Axum服务器 (AxumServer)-->>-客户端 (Client): 返回 HTTP 响应 (JSON 或错误)
```

## 错误处理流程 (当 Service/DB 返回 Err)

```mermaid
sequenceDiagram
    participant 处理函数 (Handler - task_controller.rs)
    participant 服务层 (Service - task_service.rs)
    participant 数据访问层 (DB - db.rs)
    participant 错误类型 (AppError - error.rs)
    participant Axum框架 (Axum)
    participant 客户端 (Client)

    处理函数 (Handler - task_controller.rs)->>+服务层 (Service - task_service.rs): 调用服务函数
    服务层 (Service - task_service.rs)->>+数据访问层 (DB - db.rs): 调用 DB 函数
    数据访问层 (DB - db.rs)-->>-服务层 (Service - task_service.rs): 返回 Err(AppError::NotFound)
    服务层 (Service - task_service.rs)-->>-处理函数 (Handler - task_controller.rs): 返回 Err(AppError::NotFound)
    处理函数 (Handler - task_controller.rs)-->>Axum框架 (Axum): 返回 Err(AppError::NotFound) (因为函数返回 Result<T>)
    Axum框架 (Axum)->>+错误类型 (AppError - error.rs): 调用 AppError::into_response(Err(..))
    错误类型 (AppError - error.rs)-->>-Axum框架 (Axum): 返回 Response (e.g., 404 + JSON body)
    Axum框架 (Axum)-->>客户端 (Client): 发送最终 HTTP 错误响应
``` 