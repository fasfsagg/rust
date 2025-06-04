# 模型层 (Model) 流程图

## 1. 创建任务 (Create Task) - 反序列化流程

```mermaid
graph LR
    A[客户端发起 POST /tasks 请求 (JSON Body)] --> B{Axum 接收请求};
    B --> C{Serde 反序列化};
    subgraph Serde 反序列化 (Deserialization)
        C -- JSON 数据 --> D(解析 JSON 结构);
        D -- "title": "..." --> E[创建 CreateTaskPayload.title (String)];
        D -- "description": "..."? --> F{存在 description?};
        F -- 是 --> G[创建 CreateTaskPayload.description (Some(String))];
        F -- 否/null<br/>(serde(default)) --> H[创建 CreateTaskPayload.description (None)];
        D -- "completed": ...? --> I{存在 completed?};
        I -- 是 (true/false) --> J[创建 CreateTaskPayload.completed (bool)];
        I -- 否/null<br/>(serde(default)) --> K[创建 CreateTaskPayload.completed (false)];
        E & G & J --> L(构建 CreateTaskPayload 实例);
        H & K --> L;
    end
    C --> M[控制器 (Controller) 获取 CreateTaskPayload];
```

**说明**: 
- Axum 框架接收到 HTTP 请求后，将 JSON 请求体交给 `serde`。
- `serde` 根据 `CreateTaskPayload` 结构体的定义和 `#[serde(default)]` 属性，尝试将 JSON 字段映射到结构体字段。
- 如果必需字段（如 `title`）缺失，反序列化会失败。
- 可选字段（`description`, `completed`）如果缺失或为 `null`，会使用 `Option::None` 或 `false` 作为默认值。
- 最终生成一个 `CreateTaskPayload` 实例供 Controller 使用。

## 2. 更新任务 (Update Task) - 反序列化流程 (重点关注 description)

```mermaid
graph LR
    A[客户端发起 PUT /tasks/:id 请求 (JSON Body)] --> B{Axum 接收请求};
    B --> C{Serde 反序列化};
    subgraph Serde 反序列化 UpdateTaskPayload
        C -- JSON 数据 --> D(解析 JSON 结构);
        D -- "title": "..."? --> E{存在 title?};
        E -- 是 (String) --> F[UpdateTaskPayload.title = Some(String)];
        E -- 否/null --> G[UpdateTaskPayload.title = None];
        
        D -- "description": ...? --> H{存在 description?};
        H -- 是 --> I{调用 double_option::deserialize};
        H -- 否 --> J{调用 double_option::deserialize (因为 serde(default))};
        
        subgraph double_option::deserialize
            I -- "description": "新值" --> K(解析为 Some(Some("新值")));
            I -- "description": null --> L(解析为 Some(None));
            J -- (字段不存在) --> M(解析为 None);
            K --> N[返回 Ok(Some("新值"))];
            L --> O[返回 Ok(None)];
            M --> P[返回 Ok(None)];
        end
        
        N --> Q[UpdateTaskPayload.description = Some("新值")];
        O --> R[UpdateTaskPayload.description = None];
        P --> R;

        D -- "completed": ...? --> S{存在 completed?};
        S -- 是 (true/false) --> T[UpdateTaskPayload.completed = Some(bool)];
        S -- 否/null --> U[UpdateTaskPayload.completed = None];
        
        F & Q & T --> V(构建 UpdateTaskPayload 实例);
        G & R & U --> V;
    end
    C --> W[控制器 (Controller) 获取 UpdateTaskPayload];
```
**说明**: 
- 更新流程类似创建，但所有字段都是可选的 (`Option<T>`)。
- 关键在于 `description` 字段的处理：
    - `#[serde(default, with = "double_option")]` 确保无论 JSON 中是否有 `description`，都会调用 `double_option::deserialize`。
    - `double_option::deserialize` 内部处理 JSON 的三种情况（键不存在、值为 null、有值），并统一返回 `Option<String>` 给 `UpdateTaskPayload` 结构体。
    - 最终 `UpdateTaskPayload.description` 的值：
        - `Some(String)`: 如果 JSON 中提供了非 null 的 `description`。
        - `None`: 如果 JSON 中 **没有** `description` 键 或者 `description` 的值是 `null`。

## 3. 获取任务 (Get Task) - 序列化流程

```mermaid
graph LR
    A[服务层(Service)/数据访问层(DB)获取 Task 实例] --> B{控制器 (Controller) 准备响应};
    B --> C{Serde 序列化};
    subgraph Serde 序列化 Task (Serialization)
        C -- Task 实例 --> D{遍历 Task 字段};
        D -- id (Uuid) --> E[序列化为 "uuid-string"];
        D -- title (String) --> F[序列化为 "title-string"];
        D -- description (Option<String>) --> G{description 是 Some?};
        G -- 是 (Some(String)) --> H[序列化为 "desc-string"];
        G -- 否 (None)<br/>(skip_serializing_if) --> I[跳过该字段];
        D -- completed (bool) --> J[序列化为 true/false];
        D -- created_at (u64) --> K[序列化为 number];
        D -- updated_at (u64) --> L[序列化为 number];
        E & F & H & J & K & L --> M(构建 JSON 对象);
        I --> M; 
    end
    C --> N[生成 JSON 响应体];
    N --> O[客户端接收 JSON];
```

**说明**: 
- Controller 将从 Service 获取到的 `Task` 实例交给 `serde` 进行序列化。
- `serde` 根据 `Task` 结构体和 `#[derive(Serialize)]` 以及 `#[serde(skip_serializing_if = "Option::is_none")]` 属性，将字段转换为 JSON 格式。
- 如果 `description` 字段是 `None`，则由于 `skip_serializing_if` 属性，最终的 JSON 输出中将不包含 `description` 键。
- 其他字段正常序列化为其对应的 JSON 类型。 