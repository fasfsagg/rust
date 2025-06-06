# 服务层 (Service) 核心概念 (`src/app/service/`)

## 1. 职责与定位

服务层 (`src/app/service/`) 是应用程序【业务逻辑的核心】所在。它位于控制器层 (Controller) 和数据模型/ORM (SeaORM) 之间，扮演着协调者的角色。其主要职责包括：

- **封装业务规则**: 实现应用程序独有的业务流程、计算和校验逻辑。例如，用户注册时检查用户名是否已存在、密码的哈希处理、JWT的生成与校验；任务创建时的默认值设定、状态流转规则等。
- **协调数据操作**: 根据业务需求，调用 SeaORM 提供的接口来操作数据库实体 (Entities) 和活动模型 (ActiveModels)，完成数据的持久化和检索。服务层本身不直接构造 SQL 语句，而是通过 ORM 与数据库交互。
- **事务管理**: 对于需要原子性保证的复杂操作（涉及多个数据更改），服务层可以负责管理数据库事务的开始、提交或回滚 (通过 SeaORM 的事务 API)。
- **提供清晰的接口**: 向控制器层暴露定义良好的函数接口，隐藏底层的数据访问细节和复杂的业务流程。

**关键点**: 服务层应该【独立于】具体的 Web 框架 (Axum)。它只关注业务逻辑本身，并通过 ORM (SeaORM) 与数据持久化层解耦。

## 2. 服务模块示例

本项目包含两个主要的服务模块：`auth_service.rs` 和 `task_service.rs`。

### 2.1. `auth_service.rs` (`AuthService`)
*   **用户注册 (`register_user`)**:
    *   接收控制器传来的 `RegisterUserPayload`。
    *   使用 `user::Entity::find()` 检查用户名是否已存在。
    *   使用 `argon2`库哈希用户密码。
    *   创建一个 `user::ActiveModel` 实例，填充字段。
    *   调用 `.insert(db)` 将新用户数据持久化到数据库。
    *   返回新创建的 `user::Model`。
*   **用户登录 (`login_user`)**:
    *   接收 `LoginUserPayload`。
    *   使用 `user::Entity::find()` 根据用户名查询用户。
    *   如果用户存在，使用 `argon2` 验证提供的密码和存储的哈希是否匹配。
    *   如果验证成功，准备 JWT 的 `Claims` (包含 `sub`, `username`, `exp` 等)。
    *   使用 `jsonwebtoken::encode` 和从 `AppConfig` 中获取的密钥 (`jwt_secret`) 及有效期 (`jwt_expiration_seconds`) 来生成 JWT 字符串。
    *   返回 JWT 字符串。

### 2.2. `task_service.rs`
*   **任务创建 (`create_task`)**:
    *   接收 `CreateTaskPayload`。
    *   创建一个 `task::ActiveModel` 实例。`ActiveModelBehavior` 的 `new()` 方法会自动设置 `created_at` 和 `updated_at` 的初始值。
    *   填充 `title`, `description`, `completed` 字段。
    *   调用 `.insert(db)` 将新任务存入数据库。
    *   返回新创建的 `task::Model`。
*   **获取所有任务 (`get_all_tasks`)**:
    *   调用 `task::Entity::find()->all(db)` 来获取所有任务记录。
    *   返回 `Vec<task::Model>`。
*   **根据ID获取任务 (`get_task_by_id`)**:
    *   接收任务 `id` (i32)。
    *   调用 `task::Entity::find_by_id(id)->one(db)` 查询特定任务。
    *   如果找到，返回 `Some(task::Model)`，否则返回 `None` (服务层会将其映射到 `AppError::TaskNotFound`)。
*   **更新任务 (`update_task`)**:
    *   接收任务 `id` (i32) 和 `UpdateTaskPayload`。
    *   首先调用 `task::Entity::find_by_id(id)->one(db)` 获取待更新的任务模型。
    *   将查询到的 `task::Model` 转换为 `task::ActiveModel` (使用 `.into_active_model()`)。
    *   根据 `UpdateTaskPayload` 中的值（如果为 `Some`）来设置 `ActiveModel` 中相应的字段。
    *   `ActiveModelBehavior` 的 `before_save()` 方法会自动更新 `updated_at` 时间戳。
    *   调用 `.update(db)` 将更改持久化到数据库。
    *   返回更新后的 `task::Model`。
*   **删除任务 (`delete_task`)**:
    *   接收任务 `id` (i32)。
    *   (可选) 先查询任务是否存在及其数据，以便返回被删除的任务信息。
    *   调用 `task::Entity::delete_by_id(id)->exec(db)` 来删除记录。
    *   检查 `DeleteResult::rows_affected` 来确认是否真的有记录被删除。
    *   返回被删除的 `task::Model` 或确认信息。

## 3. 异步 (`async/await`)

- **与 ORM 协同**: SeaORM 的所有数据库操作都是异步的，因此服务层函数自然也都是 `async fn`。
- **非阻塞**: `async` 确保了服务层在执行数据库查询等 I/O 密集型操作时不会阻塞当前线程，这对于构建高并发 Web 应用至关重要。

## 4. 依赖注入与可测试性

- **依赖注入**:
    *   服务函数通过参数接收 `&DatabaseConnection` (SeaORM 的数据库连接池实例) 和 `&AppConfig` (如果需要配置信息，如 JWT 密钥)。
    *   这些依赖由上层 (通常是 `startup.rs` 中创建 `AppState` 时，或直接在测试代码中) 提供。
- **解耦与可测试性**:
    *   服务层不直接创建数据库连接或配置实例，增强了模块的独立性。
    *   在单元测试或集成测试中，可以方便地传入一个连接到测试数据库的 `DatabaseConnection` 实例和一个测试用的 `AppConfig` 实例，从而实现对服务层业务逻辑的隔离测试。

## 5. 与其他层的关系

- **被控制器层 (Controller) 调用**: 控制器解析 HTTP 请求，提取数据后调用相应的服务层函数来处理业务。
- **使用模型层 (Model)**:
    *   服务层接收模型层定义的 `Payload` 结构体作为输入参数。
    *   服务层使用模型层定义的 SeaORM **实体 (`Entity`)** 和 **活动模型 (`ActiveModel`)** 来与数据库交互。
    *   服务层通常返回 **模型 (`Model`)** 结构体的实例（或包含它们的 `Vec`）给控制器层。
- **使用错误处理层 (Error)**: 服务层函数通常返回自定义的 `Result<T, AppError>` 类型，并在出错时构造并返回适当的 `AppError` 枚举变体 (例如，将 SeaORM 的 `DbErr` 转换为 `AppError::DatabaseError`，或返回 `AppError::TaskNotFound` 等业务错误)。