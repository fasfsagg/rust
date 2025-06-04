# 控制器层 (Controller) 流程图

## 1. 处理 `POST /tasks` 请求 (`create_task` Handler)

```mermaid
graph TD
    A["HTTP POST /tasks<br/>(携带 JSON Body)"] --> B{Axum Router 匹配路由};
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
        K --> L{"处理 Service 返回的 `Result<Task>`"};
        L -- "Ok(task)" --> M["构造 `Ok((StatusCode::CREATED, Json(task)))`"];
        L -- "Err(app_error)" --> N["通过 `?` 操作符<br/>返回 `Err(app_error)`"];
    end
    
    subgraph Axum 处理 Handler 返回值
        direction TB
        M --> O{"返回值 `Ok(impl IntoResponse)`"};
        N --> O; 
        O --> P{"调用 `IntoResponse::into_response()`"};
        P -- "来自 Ok(...) 的元组" --> Q["生成 HTTP 响应<br/>- Status: 201 Created<br/>- Header: Content-Type: application/json<br/>- Body: task 的 JSON 序列化"];
        P -- "来自 Err(app_error)" --> R["调用 `AppError::into_response()`<br/>生成 HTTP 错误响应<br/>(例如 404 or 500)"];
    end
    
    Q --> S["将 HTTP 响应发送给客户端"];
    R --> S;
    H --> S;
```

**说明**:
- 这个流程展示了 Axum 如何处理一个典型的 POST 请求。
- **关键点**: 提取器的执行先于 Handler 函数本体。任何提取器失败都会导致请求被短路，直接返回错误。
- Handler 函数返回 `Result<impl IntoResponse>`，Axum 会根据 `Ok` 或 `Err` 调用相应的 `into_response()` 方法来生成最终的 HTTP 响应。

## 2. 处理 `GET /tasks/:id` 请求 (`get_task_by_id` Handler)

```mermaid
graph TD
    A["HTTP GET /tasks/some-uuid"] --> B{Axum Router 匹配路由};
    B --> C["`get_task_by_id` Handler 被选中"];
    
    subgraph Axum 执行 Handler
        direction TB
        C --> D{"尝试运行提取器 (Extractors)"};
        subgraph Extractors
            direction LR
            D -- "1. State<AppState>" --> E["克隆 AppState (成功)"];
            D -- "2. Path<String>" --> F["从路径提取 "some-uuid"<br/>(绑定到 `id_str`)"];
        end
        E & F --> G{"所有提取器成功, 调用 Handler 函数"};
    end

    G --> H{`get_task_by_id(state, id_str)` 执行};
    subgraph Handler 内部逻辑 (`get_task_by_id`)
        direction TB
        H --> I{"调用 `parse_uuid(&id_str)`"};
        I -- "解析失败 (Err)" --> J["通过 `?` 返回<br/>`Err(AppError::InvalidUuid)`"];
        I -- "解析成功 (Ok(id))" --> K{"调用 `service::get_task_by_id(&state.db, id).await`"};
        K --> L{"处理 Service 返回的 `Result<Task>`"};
        L -- "Ok(task)" --> M["构造 `Ok((StatusCode::OK, Json(task)))`"];
        L -- "Err(app_error)" --> N["通过 `?` 返回<br/>`Err(app_error)` (e.g., TaskNotFound)"];
    end
    
    subgraph Axum 处理 Handler 返回值
        direction TB
        M --> O{"返回值 `Ok(impl IntoResponse)`"};
        N --> O;
        J --> O {"返回值 `Err(AppError::InvalidUuid)`"};
        O --> P{"调用 `IntoResponse::into_response()`"};
        P -- "来自 Ok(...) 的元组" --> Q["生成 HTTP 响应<br/>- Status: 200 OK<br/>- Body: task 的 JSON"];
        P -- "来自 Err(app_error)" --> R["调用 `AppError::into_response()`<br/>生成 HTTP 错误响应 (404 or 400)"];
    end
    
    Q --> S["将 HTTP 响应发送给客户端"];
    R --> S;
```

**说明**:
- 这个流程展示了 `Path` 提取器的使用，以及 Handler 内部的错误处理（UUID 解析错误）。
- 无论错误发生在 Handler 内部（如 `parse_uuid`）还是 Service 层，最终都会通过返回 `Err(AppError)` 并由 Axum 的 `IntoResponse` 机制统一转换为 HTTP 错误响应。

## 3. WebSocket 升级流程 (`ws_handler`)

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

**说明**: 
- WebSocket 的处理流程比较特殊。
- Handler (`ws_handler`) 的主要作用是使用 `WebSocketUpgrade` 提取器检测升级请求，并调用 `.on_upgrade()` 注册一个回调函数 (`handle_socket`)。
- Handler 返回一个特殊的响应 (101 Switching Protocols)。
- 真正的 WebSocket 消息处理逻辑在回调函数 (`handle_socket`) 中进行，它会在连接成功建立后被 Axum 异步执行。
``` 