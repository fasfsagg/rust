# 服务层 (Service) 流程图

## 1. 创建任务 (`create_task`) 典型流程

```mermaid
graph TD
    A["控制器层: 接收 POST /tasks 请求"] --> B("提取 `CreateTaskPayload`<br/>(从 JSON Body)");
    B --> C{"调用 `service::create_task(db, payload)`"};
    
    subgraph 服务层 (`task_service::create_task`)
        direction TB
        C --> D{"开始 `create_task` 函数"};
        D --> E("占位符: 业务逻辑校验<br/>(例如, 检查标题是否为空)");
        E -- "校验 OK" --> F("解构 `payload` 为<br/>`title`, `description`, `completed`");
        F --> G{"调用 `db::create_task(...)`"};
        
        subgraph 数据访问层 (`db::create_task`)
            direction TB
            G --> H("创建 `Task` 实例<br/>(使用 `Task::new`)");
            H --> I("获取 `db` 的写锁");
            I --> J("克隆 `Task` 实例");
            J --> K("将克隆的 `Task` 插入 `HashMap`");
            K --> L("释放写锁");
            L --> M["返回原始 `Task` 实例<br/>(在 `Ok(Task)` 中)"];
        end
        
        G --> N{"接收来自 `db::create_task` 的<br/>`Result<Task>`"};
        N --> O["返回此 `Result<Task>`"];
    end
    
    O --> P{"控制器层: 接收 `Result<Task>`"};
    P -- "Ok(task)" --> Q("序列化 `task`<br/>为 JSON 响应");
    P -- "Err(e)" --> R("转换 `AppError`<br/>为 HTTP 错误响应");
```

**说明**:
1.  控制器层接收请求，解析 JSON 得到 `CreateTaskPayload`。
2.  控制器调用 `service::create_task` 函数，并将 `Db` 引用和 `payload` 传入。
3.  服务层函数 (`task_service::create_task`) 开始执行。
4.  (占位符) 执行业务逻辑校验。如果校验失败，可能直接返回 `Err`。
5.  解构 `payload` 获取字段。
6.  调用数据访问层函数 `db::create_task`。
7.  数据访问层执行具体操作：创建 `Task` 对象、获取写锁、克隆 `Task`、插入克隆体、释放锁、返回原始 `Task` 的 `Result`。
8.  服务层接收 `db` 函数的 `Result` 并将其返回。
9.  控制器层接收服务层的 `Result`。
10. 根据 `Result` 是 `Ok` 还是 `Err`，控制器构造相应的 HTTP 响应（成功则序列化 Task 为 JSON，失败则转换为错误状态码）。

## 2. 获取单个任务 (`get_task_by_id`) 典型流程

```mermaid
graph TD
    A["控制器层: 接收 GET /tasks/:id 请求"] --> B("提取 `id: Uuid`<br/>(从路径参数)");
    B --> C{"调用 `service::get_task_by_id(db, id)`"};
    
    subgraph 服务层 (`task_service::get_task_by_id`)
        direction TB
        C --> D{"开始 `get_task_by_id` 函数"};
        D --> E("占位符: 业务逻辑<br/>(例如, 检查权限)");
        E -- "权限 OK" --> F{"调用 `db::get_task_by_id(db, id)`"};
        
        subgraph 数据访问层 (`db::get_task_by_id`)
            direction TB
            F --> G("获取 `db` 的读锁");
            G --> H{"尝试 `HashMap.get(&id)`"};
            H -- "找到 (Some(&task_ref))" --> I("克隆 `task_ref`");
            H -- "未找到 (None)" --> J["返回 `Err(TaskNotFound)`"];
            I --> K["返回 `Ok(cloned_task)`"];
            G --> L("释放读锁");
        end
        
        F --> M{"接收来自 `db::get_task_by_id` 的<br/>`Result<Task>`"};
        M --> N["返回此 `Result<Task>`"];
    end
    
    N --> O{"控制器层: 接收 `Result<Task>`"};
    O -- "Ok(task)" --> P("序列化 `task`<br/>为 JSON 响应");
    O -- "Err(e)" --> Q("转换 `AppError`<br/>为 HTTP 错误响应");
```

**说明**:
- 流程与创建类似，但涉及读锁和查找操作。
- 服务层调用 `db::get_task_by_id`。
- 数据访问层获取读锁，尝试在 `HashMap` 中查找。
- 如果找到，克隆任务并返回 `Ok(Task)`；如果未找到，返回 `Err(AppError::TaskNotFound)`。
- 控制器层根据返回的 `Result` 生成响应。

## 3. 更新/删除任务流程

更新 (`update_task`) 和删除 (`delete_task`) 的流程与获取单个任务类似，主要区别在于：
- **服务层调用**: 调用 `service::update_task` 或 `service::delete_task`。
- **数据访问层操作**: 
    - 获取【写锁】而不是读锁。
    - `update_task`: 使用 `get_mut` 获取可变引用，修改字段，然后克隆返回。
    - `delete_task`: 使用 `remove` 从 `HashMap` 中移除并返回所有权。
- **错误处理**: 同样处理 `TaskNotFound` 错误。 