# 中间件层 (Middleware) 流程图

## 1. 日志系统初始化流程 (`setup_logger()`)

```mermaid
graph TD
    A[应用启动 (startup.rs)] --> B(调用 `setup_logger()`);
    B --> C{尝试读取 RUST_LOG 环境变量?};
    C -- 是 --> D[解析环境变量指令];
    C -- 否 --> E[使用默认日志级别 "info"];
    D --> F[创建 EnvFilter];
    E --> F;
    F --> G[创建 tracing_subscriber Registry];
    G --> H[添加 EnvFilter Layer];
    H --> I[添加 fmt::Layer (格式化输出)];
    I --> J[调用 .init() 设置为全局日志处理器];
    J --> K[日志系统初始化完成];
```

**说明:**
- `setup_logger()` 函数在应用启动时被调用一次。
- 它配置了日志的过滤规则 (基于 `RUST_LOG` 或默认值) 和输出格式。
- `.init()` 将配置好的日志处理器设置为全局单例。

## 2. 请求跟踪中间件 (`TraceLayer`) 的应用流程

```mermaid
graph TD
    subgraph startup.rs [应用启动时]
        S1[调用 `middleware::trace_layer()`] --> S2(获取 `TraceLayer` 实例);
        S3[创建 `CorsLayer` 实例] --> S4;
        S2 --> S4[创建 `ServiceBuilder`];
        S4 --> S5[`.layer(TraceLayer)`];
        S5 --> S6[`.layer(CorsLayer)`];
        S6 --> S7(中间件栈 `middleware_stack` 构建完成);
        S8[调用 `routes::create_routes()` 创建路由 `app`] --> S9;
        S7 --> S9[将 `middleware_stack` 应用到 `app` 上: `app.layer(middleware_stack)`];
        S9 --> S10(最终的 `Router` 构建完成);
    end

    subgraph 请求处理时 [请求处理时]
        R1[HTTP 请求进入] --> R2{中间件栈开始处理};
        R2 --> R3[CorsLayer 请求处理];
        R3 --> R4[TraceLayer 请求处理 (记录请求开始)];
        R4 --> R5[路由匹配];
        R5 --> R6[控制器 (Controller) 处理函数执行];
        R6 --> R7[生成响应];
        R7 --> R8[TraceLayer 响应处理 (记录响应结束, 状态码, 延迟)];
        R8 --> R9[CorsLayer 响应处理 (添加 CORS 头)];
        R9 --> R10{中间件栈处理结束};
        R10 --> R11[HTTP 响应发送给客户端];
    end
```

**说明:**
- **启动时**: `trace_layer()` 函数被调用以创建一个 `TraceLayer` 实例。它与其他中间件 (如 `CorsLayer`) 一起通过 `ServiceBuilder` 组合成一个中间件栈。这个栈最后被应用到整个 Axum `Router` 上。
- **请求处理时**: 每个进入的请求都会按照"洋葱模型"流经中间件栈。
    - 请求首先经过 `CorsLayer`，然后是 `TraceLayer` (记录请求开始)，然后到达路由匹配和控制器处理函数。
    - 控制器处理函数生成响应后，响应首先经过 `TraceLayer` (记录响应结束信息)，然后是 `CorsLayer` (添加必要的 CORS 响应头)，最后发送给客户端。

## 3. JWT认证流程 (`Claims::from_request_parts` Extractor)

当一个 Axum 处理器 (Handler) 的参数中包含 `claims: Claims` 时，以下流程会被触发：

```mermaid
graph TD
    A["HTTP 请求 (访问受保护路由)"] -- 参数中包含 `claims: Claims` --> B{Axum 尝试解析 `Claims`};
    B --> C["调用 `Claims::from_request_parts(parts, &app_state)`"];
    subgraph Claims::from_request_parts 内部逻辑
        direction TB
        C --> D["从 `parts.headers` 获取 `Authorization` 头部"];
        D -- "头部不存在或为空" --> E["返回 `Err(AppError::Unauthorized(\"Missing or malformed Bearer token\"))`"];
        D -- "头部存在" --> F{"检查头部值是否以 \"Bearer \" 开头"};
        F -- "否" --> G["返回 `Err(AppError::Unauthorized(\"Malformed token\"))`"];
        F -- "是" --> H["提取 token 字符串"];
        H --> I["从 `app_state.config.jwt_secret` 获取 JWT 密钥"];
        I --> J["创建 `jsonwebtoken::DecodingKey` 和 `Validation` (指定算法如 HS512)"];
        J --> K{"调用 `jsonwebtoken::decode(token, key, validation)`"};
        K -- "解码/验证成功 (返回 `TokenData<Claims>`)" --> L["提取 `token_data.claims`"];
        L --> M["返回 `Ok(Claims)`"];
        K -- "解码/验证失败 (e.g., 签名无效, 过期)" --> N["记录错误详情 (eprintln)"];
        N --> O["返回 `Err(AppError::Unauthorized(\"Invalid token\"))`"];
    end
    E --> P[请求被拒绝];
    G --> P;
    O --> P;
    M --> Q["`Claims` 对象成功注入到处理器函数"];
    P --> R[Axum 将 `AppError` 转为 HTTP 401 响应并发送给客户端<br/>(处理器函数不执行)];
    Q --> S[处理器函数继续执行，可以访问 `claims` 对象];
```

**说明**:
- 此流程展示了通过 Axum 的 `FromRequestParts` trait 实现的 JWT 认证机制。
- 当路由处理函数的参数列表包含 `claims: Claims` 时，Axum 会自动调用 `Claims::from_request_parts`。
- **成功路径**: 如果请求包含有效的 Bearer Token，并且该 Token 通过了所有的验证（签名、有效期等），则 `from_request_parts` 方法返回 `Ok(Claims)`。这些 `Claims` 数据随后被 Axum 注入到处理函数的参数中，处理函数得以正常执行。
- **失败路径**: 如果 `Authorization` 头部缺失、格式错误，或者 Token 本身无效（例如签名不匹配、已过期），`from_request_parts` 方法会返回 `Err(AppError::Unauthorized)`。在这种情况下，Axum 会捕获这个错误，并调用 `AppError::into_response()` 将其转换为一个 HTTP 401 Unauthorized 响应。原始的路由处理函数将**不会**被执行。
- 这种机制使得认证逻辑可以优雅地集成到 Axum 的请求处理流程中，保持控制器代码的简洁。