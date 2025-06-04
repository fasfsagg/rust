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
// 导入模型层定义的结构体：任务实体 `Task`，以及用于创建和更新的载荷。
use crate::app::model::{ CreateTaskPayload, Task, UpdateTaskPayload };
// 导入数据访问层 (`db.rs`) 的所有公共项（主要是 CRUD 函数和 `Db` 类型）。
// `crate::db` 表示从当前 crate 的根目录开始查找 `db` 模块。
use crate::db;
// 导入自定义的 `Result` 类型别名，用于统一函数返回值。
use crate::error::Result;
// 导入 `Uuid` 类型，用于标识任务。
use uuid::Uuid;

// --- 服务函数定义 ---

/// 服务函数：创建新任务 (Service Function: Create Task)
///
/// 【功能】: 处理创建新任务的核心业务流程。
/// 【输入】: 数据库访问接口 (`db`) 和 从 Controller 传来的已解析的请求载荷 (`payload`)。
/// 【输出】: 一个包含新创建 `Task` 或错误的 `Result`。
/// 【标记】: `pub async fn` - 定义一个公共的异步函数。[[关键语法要素: pub, async, fn]]
///
/// # 【参数】
/// * `db: &db::Db` - 对数据库实例的【不可变引用】。服务函数通过它调用 `db.rs` 中的函数。
/// * `payload: CreateTaskPayload` - 创建任务所需的数据。[[所有权: 移动]] (Controller 将 Payload 的所有权转移给这个函数)
///
/// # 【返回值】
/// * `-> Result<Task>`: 返回一个 `Result`。
///    - `Ok(Task)`: 成功时，包含新创建的任务实体。
///    - `Err(AppError)`: 失败时，包含一个描述错误的 `AppError`。
pub async fn create_task(db: &db::Db, payload: CreateTaskPayload) -> Result<Task> {
    // --- 业务逻辑占位符 ---
    // 在一个更完整的应用程序中，这里会是实现具体业务规则的地方。
    // 例如：
    // 1. **输入验证**: 检查 `payload.title` 是否为空，长度是否符合要求等（虽然部分验证可能在 Controller 或 Model 层做）。
    //    ```rust
    //    if payload.title.is_empty() {
    //        return Err(AppError::ValidationError("标题不能为空".to_string()));
    //    }
    //    ```
    // 2. **默认值处理**: 如果某些字段有更复杂的默认逻辑。
    // 3. **唯一性检查**: 查询数据库确保不存在同名任务（如果需要）。
    //    ```rust
    //    // 伪代码: 需要添加相应的 db 函数
    //    // if db::task_exists_with_title(db, &payload.title).await? {
    //    //     return Err(AppError::BusinessRuleViolation("任务标题已存在".to_string()));
    //    // }
    //    ```
    // 4. **权限检查**: 检查当前用户是否有权创建任务。
    // 5. **审计日志**: 记录谁在什么时间创建了任务。
    // 6. **触发事件/通知**: 例如，任务创建后发送通知给相关人员。
    println!("SERVICE: 正在处理创建任务请求..."); // 简单的日志

    // --- 解构 Payload ---
    // 使用模式匹配将 `payload` 结构体的字段解构到独立的变量中。
    // 这使得后续调用 `db::create_task` 时传递参数更清晰。
    let CreateTaskPayload { title, description, completed } = payload;

    // --- 调用数据访问层 ---
    // 调用 `db.rs` 中定义的 `create_task` 函数来执行实际的数据库插入操作。
    // 将从 `payload` 解构出来的字段传递给数据库函数。
    // **关于 `await`**: 虽然 `db::create_task` 在我们当前的内存实现中是同步的，
    // 但由于 `create_task` (本函数) 被声明为 `async`，理论上它调用的其他 `async` 函数需要使用 `.await`。
    // 不过，Rust 编译器足够智能，如果调用的函数 (`db::create_task`) 不是 `async fn`，
    // 则不需要也不能使用 `.await`。如果未来 `db::create_task` 变为 `async`，则需要在这里加上 `.await`。
    // 即 `db::create_task(db, title, description, completed).await?` (如果它返回 Result)
    // 或者 `db::create_task(db, title, description, completed).await` (如果它直接返回 Task)
    let result = db::create_task(db, title, description, completed);

    println!("SERVICE: 创建任务请求处理完成。");

    // 返回数据库操作的结果。
    result
}

/// 服务函数：获取所有任务 (Service Function: Get All Tasks)
///
/// 【功能】: 处理获取所有任务的业务逻辑（在这个简单场景下，主要是直接调用 DB 层）。
/// 【输入】: 数据库访问接口 (`db`)。
/// 【输出】: 包含所有 `Task` 实体的向量。
///
/// # 【参数】
/// * `db: &db::Db` - 数据库实例的引用。
///
/// # 【返回值】
/// * `-> Vec<Task>`: 返回包含所有任务的向量。
///   【注意】: 这里直接返回 `Vec<Task>` 而不是 `Result<Vec<Task>>`，
///            隐含的假设是获取所有任务的操作本身不太可能失败（在内存实现中是这样）。
///            在真实的数据库场景中，即使是读取操作也可能因为连接问题等失败，
///            通常会返回 `Result<Vec<Task>>`。
pub async fn get_all_tasks(db: &db::Db) -> Vec<Task> {
    // --- 业务逻辑占位符 ---
    // 可能的业务逻辑：
    // - **权限检查**: 是否允许当前用户查看所有任务？
    // - **过滤/分页**: 根据用户角色或其他条件过滤任务，或者实现分页逻辑（但通常分页参数来自 Controller）。
    println!("SERVICE: 正在处理获取所有任务请求...");

    // --- 调用数据访问层 ---
    // 直接调用 `db.rs` 中的 `get_all_tasks` 函数。
    let tasks = db::get_all_tasks(db);

    println!("SERVICE: 获取所有任务请求处理完成，找到 {} 个任务", tasks.len());

    // 返回从数据库获取的任务列表。
    tasks
}

/// 服务函数：根据 ID 获取任务 (Service Function: Get Task By ID)
///
/// 【功能】: 处理根据唯一 ID 检索单个任务的业务逻辑。
/// 【输入】: 数据库访问接口 (`db`) 和 任务 ID (`id`)。
/// 【输出】: 一个包含找到的 `Task` 或错误的 `Result`。
///
/// # 【参数】
/// * `db: &db::Db` - 数据库实例的引用。
/// * `id: Uuid` - 要检索的任务的 UUID。[[所有权: 拷贝]]
///
/// # 【返回值】
/// * `-> Result<Task>`: 返回 `Result`。
///    - `Ok(Task)`: 成功找到任务。
///    - `Err(AppError::TaskNotFound)`: 未找到任务。
pub async fn get_task_by_id(db: &db::Db, id: Uuid) -> Result<Task> {
    // --- 业务逻辑占位符 ---
    // 可能的业务逻辑：
    // - **权限检查**: 当前用户是否有权查看这个特定的任务？（例如，只能看自己创建的任务）。
    // - **数据转换**: 如果需要将数据库模型转换为不同的 DTO 返回给上层。
    println!("SERVICE: 正在处理获取任务 ID: {} 的请求...", id);

    // --- 调用数据访问层 ---
    // 调用 `db.rs` 中的 `get_task_by_id` 函数。
    let result = db::get_task_by_id(db, id);

    match &result {
        Ok(_) => println!("SERVICE: 获取任务 ID: {} 的请求处理成功。", id),
        Err(_) => println!("SERVICE: 获取任务 ID: {} 的请求处理失败（未找到）。", id),
    }

    // 返回数据库操作的结果。
    result
}

/// 服务函数：更新任务 (Service Function: Update Task)
///
/// 【功能】: 处理更新现有任务的业务逻辑。
/// 【输入】: 数据库访问接口 (`db`)，要更新的任务 ID (`id`)，以及包含更新信息的载荷 (`payload`)。
/// 【输出】: 一个包含更新后 `Task` 或错误的 `Result`。
///
/// # 【参数】
/// * `db: &db::Db` - 数据库实例的引用。
/// * `id: Uuid` - 要更新的任务的 UUID。[[所有权: 拷贝]]
/// * `payload: UpdateTaskPayload` - 包含可选更新字段的数据。[[所有权: 移动]]
///
/// # 【返回值】
/// * `-> Result<Task>`: 返回 `Result`。
///    - `Ok(Task)`: 成功更新任务，返回更新后的任务状态。
///    - `Err(AppError::TaskNotFound)`: 未找到要更新的任务。
///    - `Err(AppError::...)`: 其他可能的业务逻辑错误。
pub async fn update_task(db: &db::Db, id: Uuid, payload: UpdateTaskPayload) -> Result<Task> {
    // --- 业务逻辑占位符 ---
    // 可能的业务逻辑：
    // 1. **权限检查**: 用户是否有权修改这个任务？
    // 2. **状态检查**: 任务是否处于允许修改的状态？（例如，已完成的任务可能不允许修改标题）
    // 3. **输入验证**: 对 `payload` 中的字段进行更复杂的验证。
    // 4. **部分更新处理**: 确保只更新了 `payload` 中实际提供的字段。
    // 5. **审计日志**: 记录谁在何时修改了哪些字段。
    println!("SERVICE: 正在处理更新任务 ID: {} 的请求...", id);

    // --- 解构 Payload ---
    // 将 `payload` 中的可选字段解构出来。
    let UpdateTaskPayload { title, description, completed } = payload;

    // --- 调用数据访问层 ---
    // 调用 `db.rs` 中的 `update_task` 函数。
    // 将 ID 和解构出来的可选字段传递给它。
    let result = db::update_task(db, id, title, description, completed);

    match &result {
        Ok(_) => println!("SERVICE: 更新任务 ID: {} 的请求处理成功。", id),
        Err(_) => println!("SERVICE: 更新任务 ID: {} 的请求处理失败（未找到或其他错误）。", id),
    }

    // 返回数据库操作的结果。
    result
}

/// 服务函数：删除任务 (Service Function: Delete Task)
///
/// 【功能】: 处理删除任务的业务逻辑。
/// 【输入】: 数据库访问接口 (`db`) 和 要删除的任务 ID (`id`)。
/// 【输出】: 一个包含被删除 `Task` 或错误的 `Result`。
///
/// # 【参数】
/// * `db: &db::Db` - 数据库实例的引用。
/// * `id: Uuid` - 要删除的任务的 UUID。[[所有权: 拷贝]]
///
/// # 【返回值】
/// * `-> Result<Task>`: 返回 `Result`。
///    - `Ok(Task)`: 成功删除任务，返回被删除的任务数据（有时可能只返回 `Ok(())` 表示成功）。
///    - `Err(AppError::TaskNotFound)`: 未找到要删除的任务。
///    - `Err(AppError::...)`: 其他可能的业务逻辑错误（例如，权限不足）。
pub async fn delete_task(db: &db::Db, id: Uuid) -> Result<Task> {
    // --- 业务逻辑占位符 ---
    // 可能的业务逻辑：
    // 1. **权限检查**: 用户是否有权删除这个任务？
    // 2. **依赖检查**: 是否有其他数据依赖于这个任务，导致不能删除？
    // 3. **软删除**: 可能不是真的从数据库删除，而是标记为"已删除"（添加 `deleted_at` 字段）。
    // 4. **审计日志**: 记录删除操作。
    println!("SERVICE: 正在处理删除任务 ID: {} 的请求...", id);

    // --- 调用数据访问层 ---
    // 调用 `db.rs` 中的 `delete_task` 函数。
    let result = db::delete_task(db, id);

    match &result {
        Ok(_) => println!("SERVICE: 删除任务 ID: {} 的请求处理成功。", id),
        Err(_) => println!("SERVICE: 删除任务 ID: {} 的请求处理失败（未找到或其他错误）。", id),
    }

    // 返回数据库操作的结果。
    result
}
