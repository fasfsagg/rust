# 控制器层 (Controller) 流程图

## 1. 处理 `POST /api/tasks` 请求 (`create_task` Handler) - 创建任务

```mermaid
graph TD
    A["HTTP POST /api/tasks<br/>(携带 JSON Body: title, description?, completed?)"] --> B{Axum Router 匹配路由};
    B --> C["`create_task` Handler 被选中"];
    
    subgraph Axum 执行 Handler
        direction TB
        C --> D{"尝试运行提取器 (Extractors)"};
        subgraph Extractors
            direction LR
            D -- "1. State<AppState>" --> E["克隆 AppState<br/>(成功)"];
            D -- "2. Json<CreateTaskPayload>" --> F{"尝试解析 JSON Body"};
            F -- "解析成功" --> G["创建 `CreateTaskPayload`<br/>(绑定到 `payload`)"];
            F -- "解析失败 (无效JSON/结构错误)" --> H["**直接返回 4xx 错误响应**<br/>(Handler 不执行)"];
        end
        E & G --> I{"所有提取器成功, 调用 Handler 函数"};
    end

    I --> J{`create_task(state, payload)` 执行};
    subgraph Handler 内部逻辑 (`create_task`)
        direction TB
        J --> K{"调用 `service::create_task(&state.db, payload).await`"};
        K --> L{"处理 Service 返回的 `Result<task::Model>`"};
        L -- "Ok(task_model)" --> M["构造 `Ok((StatusCode::CREATED, Json(task_model)))`"];
        L -- "Err(app_error)" --> N["通过 `?` 操作符<br/>返回 `Err(app_error)`"];
    end
    
    subgraph Axum 处理 Handler 返回值
        direction TB
        M --> O{"返回值 `Ok(impl IntoResponse)`"};
        N --> O; 
        O --> P{"调用 `IntoResponse::into_response()`"};
        P -- "来自 Ok(...) 的元组" --> Q["生成 HTTP 响应<br/>- Status: 201 Created<br/>- Header: Content-Type: application/json<br/>- Body: task model 的 JSON 序列化"];
        P -- "来自 Err(app_error)" --> R["调用 `AppError::into_response()`<br/>生成 HTTP 错误响应<br/>(例如 404 or 500)"];
    end
    
    Q --> S["将 HTTP 响应发送给客户端"];
    R --> S;
    H --> S;
```

## 2. 处理 `GET /api/tasks/{id}` 请求 (`get_task_by_id` Handler) - 获取特定任务

```mermaid
graph TD
    A["HTTP GET /api/tasks/{id} (e.g., /api/tasks/123)"] --> B{Axum Router 匹配路由};
    B --> C["`get_task_by_id` Handler 被选中"];
    
    subgraph Axum 执行 Handler
        direction TB
        C --> D{"尝试运行提取器 (Extractors)"};
        subgraph Extractors
            direction LR
            D -- "1. State<AppState>" --> E["克隆 AppState (成功)"];
            D -- "2. Path<i32>" --> F["从路径提取 id (整数)<br/>(绑定到 `id`)"];
        end
        E & F --> G{"所有提取器成功, 调用 Handler 函数"};
    end

    G --> H{`get_task_by_id(state, id)` 执行};
    subgraph Handler 内部逻辑 (`get_task_by_id`)
        direction TB
        H --> K{"调用 `service::get_task_by_id(&state.db, id).await`"};
        K --> L{"处理 Service 返回的 `Result<task::Model>`"};
        L -- "Ok(task_model)" --> M["构造 `Ok((StatusCode::OK, Json(task_model)))`"];
        L -- "Err(app_error)" --> N["通过 `?` 返回<br/>`Err(app_error)` (e.g., TaskNotFound)"];
    end
    
    subgraph Axum 处理 Handler 返回值
        direction TB
        M --> O{"返回值 `Ok(impl IntoResponse)`"};
        N --> O;
        O --> P{"调用 `IntoResponse::into_response()`"};
        P -- "来自 Ok(...) 的元组" --> Q["生成 HTTP 响应<br/>- Status: 200 OK<br/>- Body: task model 的 JSON"];
        P -- "来自 Err(app_error)" --> R["调用 `AppError::into_response()`<br/>生成 HTTP 错误响应 (404)"];
    end
    
    Q --> S["将 HTTP 响应发送给客户端"];
    R --> S;
```

## 3. 用户注册流程 (`POST /api/register`)

```mermaid
graph TD
    A["HTTP POST /api/register<br/>(携带 JSON Body: username, password)"] --> B{Axum Router 匹配路由};
    B --> C["`register_handler` 被选中"];

    subgraph Axum 执行 Handler
        direction TB
        C --> D{"尝试运行提取器"};
        D -- "1. State<AppState>" --> E["克隆 AppState"];
        D -- "2. Json<RegisterUserPayload>" --> F{"解析 JSON Body"};
        F -- "成功" --> G["创建 `RegisterUserPayload`"];
        F -- "失败" --> X["返回 4xx 错误"];
    end

    E & G --> H{"调用 `register_handler(app_state, payload)`"};
    subgraph Handler 内部逻辑 (`register_handler`)
        H --> I{"调用 `AuthService::register_user(&app_state.db, payload).await`"};
        subgraph AuthService::register_user
            I --> J["检查用户名是否存在 (SeaORM 查询)"];
            J -- "已存在" --> K["返回 `Err(AppError::UsernameAlreadyExists)`"];
            J -- "不存在" --> L["哈希密码 (argon2)"];
            L --> M["创建 `user::ActiveModel`"];
            M --> N["插入数据库 (SeaORM `insert`)"];
            N -- "成功" --> O["返回 `Ok(user::Model)`"];
            N -- "失败" --> P["返回 `Err(AppError::DatabaseError)`"];
        end
        O --> Q["构造 `Ok(Json(UserResponse))`"];
        K --> R["返回 `Err(AppError)`"]; P --> R;
    end

    subgraph Axum 处理 Handler 返回值
        Q --> S{"调用 `IntoResponse`"}; S --> T["生成 HTTP 200 OK 响应 (含 UserResponse JSON)"];
        R --> U{"调用 `AppError::into_response()`"}; U --> V["生成 HTTP 错误响应 (e.g., 409, 500)"];
    end

    T --> W["发送响应给客户端"]; V --> W; X --> W;
```

## 4. 用户登录流程 (`POST /api/login`)

```mermaid
graph TD
    A["HTTP POST /api/login<br/>(携带 JSON Body: username, password)"] --> B{Axum Router 匹配路由};
    B --> C["`login_handler` 被选中"];

    subgraph Axum 执行 Handler
        C --> D{"提取 `State<AppState>` 和 `Json<LoginUserPayload>`"};
        D --> E{"调用 `login_handler(app_state, payload)`"};
    end

    subgraph Handler 内部逻辑 (`login_handler`)
        E --> F{"调用 `AuthService::login_user(&app_state.db, &app_state.config, payload).await`"};
        subgraph AuthService::login_user
            F --> G["查询用户 (SeaORM `find().filter()`)"];
            G -- "未找到" --> H["返回 `Err(AppError::UserNotFound)`"];
            G -- "找到 (user_model)" --> I["验证密码 (argon2 `verify_password`)"];
            I -- "密码错误" --> J["返回 `Err(AppError::InvalidPassword)`"];
            I -- "密码正确" --> K["准备 JWT Claims (sub, username, exp, iat)"];
            K --> L["生成 JWT (jsonwebtoken `encode`, 使用 `app_state.config.jwt_secret`)"];
            L -- "成功" --> M["返回 `Ok(token_string)`"];
            L -- "失败" --> N["返回 `Err(AppError::JwtCreationError)`"];
        end
        M --> O["构造 `Ok(Json(LoginResponse))`"];
        H --> P["返回 `Err(AppError)`"]; J --> P; N --> P;
    end

    subgraph Axum 处理 Handler 返回值
        O --> Q["生成 HTTP 200 OK 响应 (含 LoginResponse JSON with token)"];
        P --> R["生成 HTTP 错误响应 (e.g., 401, 500)"];
    end

    Q --> S["发送响应给客户端"]; R --> S;
```

## 5. 受保护路由访问流程 (例如 `GET /api/protected_data`)

```mermaid
graph TD
    A["HTTP GET /api/protected_data<br/>(携带 Header: \"Authorization: Bearer <token>\")"] --> B{Axum Router 匹配路由};
    B --> C["`protected_data_handler` 被选中"];

    subgraph Axum 执行 Handler (参数提取)
        direction TB
        C --> D{"尝试运行提取器"};
        D -- "1. `claims: Claims`" --> E{"调用 `Claims::from_request_parts()`"};
        subgraph Claims::from_request_parts
            direction TB
            E --> F["获取 `AppState` (含 `AppConfig`)"];
            F --> G["提取 `Authorization` 头部"];
            G -- "无或格式错误" --> H["返回 `Err(AppError::Unauthorized)`"];
            G -- "有 Bearer token" --> I["解码和验证 JWT (使用 `jwt_secret`, 检查 `exp`)"];
            I -- "验证失败 (无效/过期)" --> J["返回 `Err(AppError::Unauthorized)`"];
            I -- "验证成功" --> K["返回 `Ok(Claims)`"];
        end
        H --> L{提取失败}; J --> L;
        K --> M{提取成功};
    end

    subgraph Handler 调用与响应
        direction TB
        M --> N{"调用 `protected_data_handler(claims)`"};
        N --> O["Handler 执行业务逻辑 (使用 `claims` 数据)"];
        O --> P["构造 `Ok(Json(response_data))`"];
        P --> Q["生成 HTTP 200 OK 响应 (含 JSON)"];
        L --> R{"Axum 将 `AppError` 转为 HTTP 401 响应"};
    end

    Q --> S["发送响应给客户端"]; R --> S;
```

## 6. WebSocket 升级流程 (`ws_handler`) (基本不变)

```mermaid
graph TD
    A["Client 发起 GET /ws 请求<br/>(包含 Upgrade Headers)"] --> B{Axum Router 匹配路由};
    B --> C["`ws_handler` Handler 被选中"];
    
    subgraph Axum 执行 Handler
        direction TB
        C --> D{"尝试运行提取器 (Extractors)"};
        subgraph Extractors
            direction LR
            D -- "1. WebSocketUpgrade" --> E["检测到升级请求 (成功)"];
            D -- "2. State<AppState>" --> F["克隆 AppState (成功)"];
        end
        E & F --> G{"所有提取器成功, 调用 Handler 函数"};
    end

    G --> H{`ws_handler(ws, state)` 执行};
    subgraph Handler 内部逻辑 (`ws_handler`)
        direction TB
        H --> I{"调用 `ws.on_upgrade(move |socket| handle_socket(socket, state))`"};
        I --> J["返回一个特殊的<br/>`impl IntoResponse`<br/>(由 `on_upgrade` 生成)"];
    end
    
    subgraph Axum 处理 Handler 返回值
        direction TB
        J --> K{"调用 `IntoResponse::into_response()`"};
        K --> L["生成 HTTP 响应<br/>- Status: 101 Switching Protocols<br/>- 包含必要的 WebSocket Headers"];
    end
    
    L --> M["将 101 响应发送给客户端"];
    M --> N["HTTP 连接升级为 WebSocket 连接"];
    N --> O{"`handle_socket(socket, state)`<br/>在一个新 Task 中被异步调用"};
    O --> P["WebSocket 通信开始<br/>(发送欢迎消息, 进入 recv 循环)"];
```