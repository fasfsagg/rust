// app/service/task_service.rs
//
// 【任务服务模块】
// 这个模块实现了任务相关的业务逻辑，它是服务层的一部分。
// 服务层负责处理业务规则和流程，它调用数据访问层执行数据操作。
//
// 分层设计的好处体现在这里：
// - 业务逻辑与数据访问分离，每层专注于自己的职责
// - 控制器层调用服务层，服务层调用数据层，保持清晰的依赖方向
// - 服务层可以被单元测试，不需要HTTP请求和响应的复杂模拟

// /-------------------------------------------------------------------------\
// |                           【模块功能图示】                            |
// |-------------------------------------------------------------------------|
// |   控制器层 (Controller Layer)                                           |
// |        |                                                                |
// |        | 调用服务函数 (Calls Service Functions)                        |
// |        V                                                                |
// | +---------------------------------------------------------------------+ |
// | |                 task_service.rs (本模块)                          | |
// | |---------------------------------------------------------------------| |
// | | 依赖项 (Dependencies):                                              | |
// | |  - `model::{Task, CreateTaskPayload, UpdateTaskPayload}` (模型)    | |
// | |  - `db::{self, Db}` (数据访问层函数和类型)                          | |
// | |  - `error::Result` (自定义 Result 类型)                            | |
// | |  - `uuid::Uuid`                                                      | |
// | |---------------------------------------------------------------------| |
// | | 公共函数 (业务逻辑):                                                | |
// | |  - async fn create_task(db: &Db, payload: CreateTaskPayload)      | |
// | |      -> Result<Task>                                                | |
// | |          |-> 调用 db::create_task(...)                           | |
// | |  - async fn get_all_tasks(db: &Db) -> Vec<Task>                     | |
// | |          |-> 调用 db::get_all_tasks(...)                          | |
// | |  - async fn get_task_by_id(db: &Db, id: Uuid) -> Result<Task>       | |
// | |          |-> 调用 db::get_task_by_id(...)                         | |
// | |  - async fn update_task(db: &Db, id: Uuid, payload: UpdateTaskPayload)| |
// | |      -> Result<Task>                                                | |
// | |          |-> 调用 db::update_task(...)                           | |
// | |  - async fn delete_task(db: &Db, id: Uuid) -> Result<Task>       | |
// | |          |-> 调用 db::delete_task(...)                           | |
// | +---------------------------------------------------------------------+ |
// |        |                                                                |
// |        | 调用数据访问层函数 (Calls Data Access Layer Functions)        |
// |        V                                                                |
// |   数据访问层 (Data Access Layer - db.rs)                                |
// \-------------------------------------------------------------------------/
//
// 文件路径: src/app/service/task_service.rs
//
// 【模块核心职责】
// 这个模块是应用程序的【服务层 (Service Layer)】的一部分，专门负责处理与"任务"相关的【业务逻辑】。
// 它是连接控制器层 (Controller) 和数据访问层 (DB) 的桥梁。
//
// 【主要职责】
// 1. **实现业务规则**: 封装应用程序的核心业务流程和规则。例如，创建任务时可能需要检查标题是否唯一（本项目未实现），更新任务时可能需要验证用户权限等。
// 2. **协调数据操作**: 调用数据访问层 (`db.rs`) 提供的函数来执行实际的数据持久化操作（增删改查）。服务层本身不直接与数据库交互。
// 3. **处理输入和输出**: 接收来自控制器层的数据（通常是 DTO，如 `CreateTaskPayload`），处理后可能返回领域模型对象（如 `Task`）或结果。
// 4. **事务管理（如果需要）**: 在涉及多个数据操作的复杂业务流程中，服务层通常负责管理数据库事务的开始、提交或回滚（在本项目内存数据库中不涉及）。
//
// 【分层架构中的地位】
// - **隔离关注点**: 将业务逻辑与 HTTP 处理（Controller）和数据存储（DB）分离开来。
// - **可测试性**: 服务层的函数通常是纯粹的业务逻辑，不依赖于 Web 框架的具体实现，更容易进行单元测试。
// - **可重用性**: 业务逻辑被封装在服务层，可以被不同的入口（如 HTTP API、命令行工具、定时任务等）复用。
//
// 【关键技术点】
// - **`async fn`**: 这些函数被标记为【异步函数】。[[Rust语法特性/概念: 异步编程]]
//   - 在 Rust 中，`async fn` 表示这个函数可以被【暂停 (suspend)】并在等待某个操作（通常是 I/O 操作，如数据库查询、网络请求）完成时让出当前线程，允许其他任务运行。
//   - 使用 `async/await` 语法可以编写看起来像同步代码的异步逻辑。
//   - **注意**: 虽然这个项目中的 `db.rs` 使用的是内存同步操作，但服务层接口设计为 `async fn` 是一个【良好实践】，因为它使得未来切换到真正的异步数据库驱动（如 `sqlx`, `tokio-postgres`）时，服务层的函数签名【无需改变】。
// - **依赖注入 (通过参数传递)**: 数据库实例 `db: &db::Db` 是作为参数传递给每个服务函数的。这是一种简单的【依赖注入】形式，使得服务函数不直接依赖于全局状态，更容易测试（可以传入模拟的 `Db` 实例）。
// - **错误处理**: 函数返回 `Result<T>`（即 `Result<T, AppError>`），统一处理可能发生的错误（如 `TaskNotFound`）。
// - **结构体解构**: 使用 `let CreateTaskPayload { ... } = payload;` 语法可以方便地从载荷结构体中提取字段。[[关键语法要素: 解构]]
//
// 【面向初学者提示】
// - **业务逻辑**: 指的是应用程序要解决的特定问题的规则和流程。例如，"创建一个新任务需要标题，并且标题不能为空"就是一个简单的业务规则。
// - **服务层**: 就像一个部门经理。控制器（前台）接到客户请求（HTTP Request），将请求信息（Payload）交给经理（Service Function）。经理根据公司规定（业务逻辑）处理信息，并指示下属（DB Function）去文件柜（数据库）存取文件（数据），最后将处理结果返回给前台。
// - **异步编程**: 想象一下在厨房做饭。同步方式是：烧水->等水开->煮面->等面熟。异步方式是：开始烧水（不等它开），同时去切菜，水开了再去煮面（不等它熟），同时去准备调料。异步允许程序在等待耗时操作（如烧水、煮面、数据库查询）时去做其他事情，提高效率。

// --- 导入依赖 ---
// 导入模型层定义的结构体：任务 DTO `Task`，以及用于创建和更新的载荷。
use crate::app::model::task::{ CreateTaskPayload, Task, UpdateTaskPayload };
// 导入自定义的 `Result` 类型别名，用于统一函数返回值。
use crate::error::{ AppError, Result };
// 导入仓库层 Trait
use crate::app::repository::task_repository::TaskRepositoryContract;
// 导入 SeaORM 相关模块和数据库实体
use migration::task_entity::ActiveModel; // 直接导入 ActiveModel
use sea_orm::{ prelude::Uuid, ActiveValue, IntoActiveModel };
use std::sync::Arc;

// --- 服务函数定义 ---

/// 服务函数：创建新任务
pub async fn create_task(
    repo: Arc<dyn TaskRepositoryContract>,
    payload: CreateTaskPayload
) -> Result<Task> {
    println!("SERVICE: 正在处理创建任务请求...");

    // 将来自 API 的 payload 转换为 SeaORM 的 ActiveModel。
    // ActiveModel 是用于执行插入和更新操作的可变模型。
    let new_task = ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()), // 生成新的 UUID
        title: ActiveValue::Set(payload.title),
        description: ActiveValue::Set(payload.description),
        completed: ActiveValue::Set(payload.completed),
        ..Default::default() // 其他字段使用默认值 (如 created_at, updated_at 由数据库生成)
    };

    // 调用仓库层来执行数据库插入。
    // 错误（DbErr）会通过 `?` 操作符自动转换为 AppError::DbErr。
    let created_task = repo.create(new_task).await?;

    println!("SERVICE: 创建任务请求处理完成。");
    // 将数据库模型转换为 API DTO 并返回
    Ok(created_task.into())
}

/// 服务函数：获取所有任务
pub async fn get_all_tasks(repo: Arc<dyn TaskRepositoryContract>) -> Result<Vec<Task>> {
    println!("SERVICE: 正在处理获取所有任务请求...");

    // 直接调用仓库层函数。
    let db_tasks = repo.find_all().await?;

    // 使用迭代器的 `map` 和 `collect` 将 Vec<db_model::Model> 转换为 Vec<Task>
    let tasks: Vec<Task> = db_tasks
        .into_iter()
        .map(|db_task| db_task.into())
        .collect();

    println!("SERVICE: 获取所有任务请求处理完成，找到 {} 个任务", tasks.len());
    Ok(tasks)
}

/// 服务函数：根据 ID 获取任务
pub async fn get_task_by_id(repo: Arc<dyn TaskRepositoryContract>, id: Uuid) -> Result<Task> {
    println!("SERVICE: 正在处理获取任务 ID: {} 的请求...", id);

    // 调用仓库层函数。
    let db_task = repo.find_by_id(id).await?;

    // `find_by_id` 返回 `Option<Model>`，我们需要处理 `None` 的情况。
    // 如果是 `None`，表示任务未找到，我们返回一个特定的应用错误。
    // 如果是 `Some(task)`，我们返回任务本身。
    match db_task {
        Some(db_task) => {
            println!("SERVICE: 获取任务 ID: {} 的请求处理成功。", id);
            // 将数据库模型转换为 API DTO 并返回
            Ok(db_task.into())
        }
        None => {
            println!("SERVICE: 获取任务 ID: {} 的请求处理失败（未找到）。", id);
            Err(AppError::TaskNotFound(id))
        }
    }
}

/// 服务函数：更新任务
pub async fn update_task(
    repo: Arc<dyn TaskRepositoryContract>,
    id: Uuid,
    payload: UpdateTaskPayload
) -> Result<Task> {
    println!("SERVICE: 正在处理更新任务 ID: {} 的请求...", id);

    // 1. 根据 ID 从数据库中获取现有的任务实体。
    //    我们使用 `.into_active_model()` 将其转换为 ActiveModel，以便进行修改。
    let mut active_task = match repo.find_by_id(id).await? {
        Some(task) => task.into_active_model(),
        None => {
            return Err(AppError::TaskNotFound(id));
        }
    };

    // 2. 检查 payload 中的每个字段，如果提供了新值，则更新 ActiveModel。
    if let Some(title) = payload.title {
        active_task.title = ActiveValue::Set(title);
    }

    if let Some(description) = payload.description {
        active_task.description = ActiveValue::Set(description);
    }

    if let Some(completed) = payload.completed {
        active_task.completed = ActiveValue::Set(completed);
    }

    // 3. 调用仓库层来执行数据库更新。
    let updated_task = repo.update(active_task).await?;

    println!("SERVICE: 更新任务 ID: {} 的请求处理完成。", id);
    // 将数据库模型转换为 API DTO 并返回
    Ok(updated_task.into())
}

/// 服务函数：删除任务
pub async fn delete_task(repo: Arc<dyn TaskRepositoryContract>, id: Uuid) -> Result<()> {
    println!("SERVICE: 正在处理删除任务 ID: {} 的请求...", id);

    // 调用仓库层执行删除操作。
    let delete_result = repo.delete(id).await?;

    // `delete` 返回 `DeleteResult`，其中包含 `rows_affected`。
    // 我们检查这个值来确定是否真的有任务被删除了。
    if delete_result.rows_affected == 0 {
        // 如果没有行受影响，说明数据库中没有这个 ID 的任务。
        println!("SERVICE: 删除任务 ID: {} 的请求处理失败（未找到）。", id);
        Err(AppError::TaskNotFound(id))
    } else {
        // 如果 `rows_affected` 是 1 (或更大，但主键删除应该是1)，说明删除成功。
        println!("SERVICE: 删除任务 ID: {} 的请求处理成功。", id);
        // 删除成功，我们不需要返回任何数据，所以返回 `Ok(())`。
        Ok(())
    }
}

// --- 单元测试 ---
#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use migration::task_entity;
    use sea_orm::{ prelude::Uuid, DbErr, DeleteResult };
    use std::sync::{ Arc, Mutex };

    // 1. 创建模拟仓库 (Mock Repository)
    // 这个结构体将模拟真实的 TaskRepository，但它不与数据库交互。
    // 它允许我们精确控制测试场景下的返回值（成功或失败）。
    #[derive(Default)]
    struct MockTaskRepository {
        // 使用 Mutex 来安全地修改测试数据
        create_result: Mutex<Option<std::result::Result<task_entity::Model, DbErr>>>,
        find_all_result: Mutex<Option<std::result::Result<Vec<task_entity::Model>, DbErr>>>,
        find_by_id_result: Mutex<Option<std::result::Result<Option<task_entity::Model>, DbErr>>>,
        update_result: Mutex<Option<std::result::Result<task_entity::Model, DbErr>>>,
        delete_result: Mutex<Option<std::result::Result<DeleteResult, DbErr>>>,
    }

    // 2. 为模拟仓库实现 `TaskRepositoryContract` Trait
    // 我们在这里实现 trait 的所有方法，但返回的是预设在 Mutex 中的结果。
    #[async_trait]
    impl TaskRepositoryContract for MockTaskRepository {
        async fn create(
            &self,
            _data: ActiveModel
        ) -> std::result::Result<task_entity::Model, DbErr> {
            self.create_result.lock().unwrap().take().unwrap()
        }
        async fn find_all(&self) -> std::result::Result<Vec<task_entity::Model>, DbErr> {
            self.find_all_result.lock().unwrap().take().unwrap()
        }
        async fn find_by_id(
            &self,
            _id: Uuid
        ) -> std::result::Result<Option<task_entity::Model>, DbErr> {
            self.find_by_id_result.lock().unwrap().take().unwrap()
        }
        async fn update(
            &self,
            _data: ActiveModel
        ) -> std::result::Result<task_entity::Model, DbErr> {
            self.update_result.lock().unwrap().take().unwrap()
        }
        async fn delete(&self, _id: Uuid) -> std::result::Result<DeleteResult, DbErr> {
            self.delete_result.lock().unwrap().take().unwrap()
        }
    }

    // 辅助函数，创建一个包含预设数据的 task model
    fn create_dummy_task_model(id: Uuid, title: &str) -> task_entity::Model {
        task_entity::Model {
            id,
            title: title.to_string(),
            description: Some("Dummy Description".to_string()),
            completed: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    // 3. 编写单元测试用例
    #[tokio::test]
    async fn test_get_task_by_id_found() {
        // --- 准备 (Arrange) ---
        let mock_repo = MockTaskRepository::default();
        let task_id = Uuid::new_v4();
        let expected_task = create_dummy_task_model(task_id, "Test Task");

        // 设置模拟仓库的返回值
        *mock_repo.find_by_id_result.lock().unwrap() = Some(Ok(Some(expected_task.clone())));

        let repo: Arc<dyn TaskRepositoryContract> = Arc::new(mock_repo);

        // --- 执行 (Act) ---
        let result = get_task_by_id(repo, task_id).await;

        // --- 断言 (Assert) ---
        assert!(result.is_ok());
        let task = result.unwrap();
        assert_eq!(task.id, task_id);
        assert_eq!(task.title, "Test Task");
    }

    #[tokio::test]
    async fn test_get_task_by_id_not_found() {
        // --- 准备 (Arrange) ---
        let mock_repo = MockTaskRepository::default();
        let task_id = Uuid::new_v4();

        // 模拟仓库返回 Ok(None)，表示数据库中没有找到
        *mock_repo.find_by_id_result.lock().unwrap() = Some(Ok(None));

        let repo: Arc<dyn TaskRepositoryContract> = Arc::new(mock_repo);

        // --- 执行 (Act) ---
        let result = get_task_by_id(repo, task_id).await;

        // --- 断言 (Assert) ---
        assert!(result.is_err());
        match result.err().unwrap() {
            AppError::TaskNotFound(id) => assert_eq!(id, task_id),
            _ => panic!("Expected TaskNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_create_task_success() {
        // --- 准备 (Arrange) ---
        let mock_repo = MockTaskRepository::default();
        let task_id = Uuid::new_v4();
        let expected_task_model = create_dummy_task_model(task_id, "New Created Task");

        // 模拟 create 方法成功返回
        *mock_repo.create_result.lock().unwrap() = Some(Ok(expected_task_model.clone()));

        let repo: Arc<dyn TaskRepositoryContract> = Arc::new(mock_repo);
        let payload = CreateTaskPayload {
            title: "New Created Task".to_string(),
            description: Some("Description".to_string()),
            completed: false,
        };

        // --- 执行 (Act) ---
        let result = create_task(repo, payload).await;

        // --- 断言 (Assert) ---
        assert!(result.is_ok());
        let created_task = result.unwrap();
        assert_eq!(created_task.id, task_id);
        assert_eq!(created_task.title, "New Created Task");
    }

    #[tokio::test]
    async fn test_delete_task_success() {
        // --- 准备 (Arrange) ---
        let mock_repo = MockTaskRepository::default();
        let task_id = Uuid::new_v4();

        // 模拟 delete 方法返回成功，影响了 1 行
        *mock_repo.delete_result.lock().unwrap() = Some(Ok(DeleteResult { rows_affected: 1 }));

        let repo: Arc<dyn TaskRepositoryContract> = Arc::new(mock_repo);

        // --- 执行 (Act) ---
        let result = delete_task(repo, task_id).await;

        // --- 断言 (Assert) ---
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_task_not_found() {
        // --- 准备 (Arrange) ---
        let mock_repo = MockTaskRepository::default();
        let task_id = Uuid::new_v4();

        // 模拟 delete 方法返回成功，但影响了 0 行
        *mock_repo.delete_result.lock().unwrap() = Some(Ok(DeleteResult { rows_affected: 0 }));

        let repo: Arc<dyn TaskRepositoryContract> = Arc::new(mock_repo);

        // --- 执行 (Act) ---
        let result = delete_task(repo, task_id).await;

        // --- 断言 (Assert) ---
        assert!(result.is_err());
        match result.err().unwrap() {
            AppError::TaskNotFound(id) => assert_eq!(id, task_id),
            _ => panic!("Expected TaskNotFound error"),
        }
    }
}
