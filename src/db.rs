// db.rs
//
// 【数据访问模块】
// 本模块实现了数据存储和检索功能。在本项目中，我们使用内存中的哈希表来模拟数据库。
// 在实际项目中，这一层通常会连接到真实的数据库（如PostgreSQL、MySQL等）。
//
// 分层设计的重要性：
// - 数据访问代码与业务逻辑分离，使代码更易于维护
// - 可以在不改变业务逻辑的情况下切换数据库实现（例如从内存存储切换到SQL数据库）
// - 提供了统一的数据访问接口，简化了上层代码

// /--------------------------------------------------------------------------------------------------\
// |                                      【模块功能图示】                                        |
// |--------------------------------------------------------------------------------------------------|
// |                                                                                                  |
// |   +-----------------+      +---------------------------------------------------------------+   |
// |   | 服务层 (Service Layer) | ---> |                     db.rs (本模块)                              |   |
// |   | (调用者)        |      |                                                               |   |
// |   +-----------------+      |   +---------------------------------------------------------+   |   |
// |                            |   | Db = Arc<RwLock<HashMap<Uuid, Task>>>                    |   |   |
// |                            |   |  - Arc: 允许多线程共享 Db                                 |   |   |
// |                            |   |  - RwLock: 控制并发访问 (多读/单写)                        |   |   |
// |                            |   |  - HashMap<Uuid, Task>: 实际存储任务数据 (内存数据库)    |   |   |
// |                            |   +---------------------------------------------------------+   |   |
// |                            |                  /|\       |                                |   |
// |                            |                   |        |                                |   |
// |                            |  +----------------+--------+------------------------------+   |   |
// |                            |  | 公共函数 (Public Functions) (提供给服务层的接口):       |   |   |
// |                            |  |  - new_db() -> Db                                       |   |   |
// |                            |  |  - init_sample_data(db: &Db)                             |   |   |
// |                            |  |  - create_task(db: &Db, ...) -> Result<Task>            |   |   |
// |                            |  |  - get_all_tasks(db: &Db) -> Vec<Task>                    |   |   |
// |                            |  |  - get_task_by_id(db: &Db, id: Uuid) -> Result<Task>      |   |   |
// |                            |  |  - update_task(db: &Db, id: Uuid, ...) -> Result<Task>   |   |   |
// |                            |  |  - delete_task(db: &Db, id: Uuid) -> Result<Task>       |   |   |
// |                            |  +---------------------------------------------------------+   |   |
// |                                                                                                  |
// \--------------------------------------------------------------------------------------------------/
//
// 文件路径: src/db.rs
//
// 【模块核心职责】
// 这个模块是应用程序的【数据访问层 (Data Access Layer, DAL)】。
// 它的核心职责是【封装】所有与数据存储和检索相关的操作细节。
// 在这个项目中，我们使用【内存中的哈希表 (HashMap)】来模拟数据库行为。
//
// 【主要内容】
// 1.  **`Db` 类型定义**: 定义了我们内存数据库的具体类型，使用了 `Arc<RwLock<HashMap<Uuid, Task>>>` 来实现线程安全的共享可变状态。
// 2.  **`new_db()`**: 创建一个新的、空的内存数据库实例。
// 3.  **`init_sample_data()`**: （可选）向数据库填充一些初始的示例数据。
// 4.  **CRUD 操作函数**: 提供了一组公共函数 (`create_task`, `get_all_tasks`, `get_task_by_id`, `update_task`, `delete_task`)，供上层（主要是 Service 层）调用，以对任务数据执行增、删、改、查操作。
//
// 【关键技术点】
// - **`HashMap<K, V>`**: Rust 标准库提供的哈希表数据结构，用于高效地存储键值对。这里用 `Uuid` 作为键，`Task` 作为值。
// - **`Arc<T>` (原子引用计数指针)**: 允许多个所有者【共享】同一份数据的所有权。当最后一个所有者消失时，数据会被清理。这对于在多个线程之间安全地共享数据（如我们的 `Db` 实例）至关重要。[[Rust语法特性/概念: 智能指针, 共享所有权]]
// - **`RwLock<T>` (读写锁)**: 一种同步原语，用于保护共享数据。它允许多个线程【同时读取】数据（读锁），但只允许一个线程【写入】数据（写锁），并且读写操作是互斥的。这确保了并发访问时的数据一致性。[[Rust语法特性/概念: 并发, 锁]]
//    - `db.read()`: 获取读锁。如果当前有写锁，则阻塞等待。
//    - `db.write()`: 获取写锁。如果当前有任何读锁或写锁，则阻塞等待。
//    - 【RAII】: `RwLockReadGuard` 和 `RwLockWriteGuard` (分别是 `read()` 和 `write()` 返回的类型) 实现了 `Drop` 特性。当 Guard 变量离开作用域时，锁会自动释放。这大大降低了忘记释放锁的风险。[[Rust语法特性/概念: RAII, Drop Trait]]
// - **`parking_lot::RwLock`**: 我们使用了 `parking_lot` 这个第三方库提供的 `RwLock`，通常认为它比标准库 `std::sync::RwLock` 具有更好的性能和更少的饥饿问题。
// - **类型别名 (`type Db = ...`)**: 为复杂的类型创建一个更简洁、更易读的名称。[[关键语法要素: type]]
// - **`.clone()`**: 由于 `RwLock` 的保护机制以及 Rust 的所有权规则，我们通常不能直接将锁内部数据的引用返回给外部。因此，在读取数据时，我们经常需要 `.clone()` 一份数据副本出来再返回。
//
// 【面向初学者提示】
// - **数据访问层 (DAL)**: 想象成图书馆的管理员。业务逻辑（Service 层）不需要知道书（数据）具体放在哪个书架（内存、硬盘、云端），只需要告诉管理员要借什么书（`get_task_by_id`）、还什么书（`update_task`）或捐赠新书（`create_task`），管理员负责具体的存放和查找操作。
// - **线程安全**: 当多个程序执行流（线程）可能同时访问和修改同一份数据时，需要采取措施（如使用 `Arc` 和 `RwLock`）来防止数据混乱或崩溃，这就是线程安全。
// - **引用计数 (Reference Counting)**: 一种内存管理技术。`Arc` 内部维护一个计数器，记录有多少个 `Arc` 指针指向同一个数据。每次克隆 `Arc` 时计数器加一，每次 `Arc` 被销毁时计数器减一。当计数器归零时，表示数据不再被任何指针引用，可以安全地释放内存。

// --- 导入依赖 ---
// 导入我们在模型层定义的 `Task` 结构体。
use crate::app::model::Task;
// 导入我们自定义的 `Result` 类型别名和 `task_not_found` 错误创建函数。
use crate::error::{ Result, task_not_found };
// 导入 `parking_lot` 库提供的读写锁 `RwLock`。
use parking_lot::RwLock;
// 导入标准库的哈希表 `HashMap`。
use std::collections::HashMap;
// 导入标准库的原子引用计数指针 `Arc`。
use std::sync::Arc;
// 导入 `uuid` 库的 `Uuid` 类型，用作任务的唯一标识符。
use uuid::Uuid;

// --- 数据库类型定义 ---

/// 数据库类型别名 (Type Alias for the Database)
///
/// 【核心】: 定义了我们整个内存数据库的结构。
/// 【组成解析】:
/// - `HashMap<Uuid, Task>`: 这是数据的【核心存储】。[[数据结构: HashMap]]
///   - `Key`: `Uuid` - 每个任务的唯一 ID。
///   - `Value`: `Task` - 完整的任务数据。
/// - `RwLock<HashMap<Uuid, Task>>`: 将 HashMap 包裹在读写锁中。[[并发原语: RwLock]]
///   - 【目的】: 允许多个线程安全地并发访问 HashMap。读操作可以并发进行，写操作（增、删、改）则需要独占访问权限。
/// - `Arc<RwLock<HashMap<Uuid, Task>>>`: 将读写锁包裹在原子引用计数指针中。[[智能指针: Arc]]
///   - 【目的】: 使得这个包含锁和数据的结构可以在多个线程之间安全地【共享】所有权。
///     例如，在 Axum 应用中，我们通常会创建一个 `Db` 实例，然后将其克隆（`Arc::clone(&db)`）传递给每个请求处理线程或任务。
///     克隆 `Arc` 只会增加引用计数，并不会复制内部的 `RwLock` 或 `HashMap`，所有克隆都指向同一个内存数据库实例。
///
/// 【现实映射】: 想象一个上了锁的共享文件柜 (`RwLock`)，里面有很多带编号的文件夹 (`HashMap<Uuid, Task>`)。这个文件柜本身可以被复制多份钥匙 (`Arc`) 分发给不同的人，但同一时间要么多个人一起看文件夹（读锁），要么只有一个人能打开柜子修改文件夹（写锁）。
pub type Db = Arc<RwLock<HashMap<Uuid, Task>>>;

// --- 数据库初始化函数 ---

/// 创建一个新的、空的数据库实例 (Function to Create a New DB)
///
/// 【功能】: 初始化内存数据库结构。
/// 【应用场景】: 通常在应用程序启动时调用一次，创建全局的数据库实例。
///
/// # 【返回值】
/// * `-> Db`: 返回一个新创建的、空的 `Db` 实例 (即 `Arc<RwLock<HashMap<...>>>`)。
///
/// # 【实现步骤】
/// 1. `HashMap::new()`: 创建一个空的哈希表。
/// 2. `RwLock::new(map)`: 将空的哈希表放入一个新的读写锁中。
/// 3. `Arc::new(locked_map)`: 将包含锁的哈希表放入一个新的 `Arc` 中。
pub fn new_db() -> Db {
    // 1. 创建一个空的 HashMap
    let map: HashMap<Uuid, Task> = HashMap::new();

    // 2. 将 HashMap 包装在 RwLock 中
    let locked_map = RwLock::new(map);

    // 3. 将 RwLock 包装在 Arc 中并返回
    Arc::new(locked_map)
}

/// 初始化数据库，添加一些示例任务 (Function to Initialize Sample Data)
///
/// 【功能】: 向数据库中填充一些预设的任务数据。
/// 【应用场景】: 主要用于开发、测试或演示目的，方便启动应用后立即有数据可用。
///             在生产环境中，通常不会硬编码示例数据。
///
/// # 【参数】
/// * `db: &Db` - 对要初始化的数据库实例的【不可变引用】。[[Rust语法特性/概念: 不可变引用]]
///             注意：虽然我们只需要读写锁的【写】权限，但 `Db` 类型是 `Arc<...>`，克隆 `Arc` 是廉价的，
///             并且获取锁的操作是在函数内部进行的，所以传入不可变引用 `&Db` 是常见的做法。
///             我们并不需要获取 `db` 本身的所有权或可变访问权。
pub fn init_sample_data(db: &Db) {
    // --- 创建示例任务数据 ---
    // 使用 `Task::new` 创建几个 Task 实例。
    // `.into()`: 这里使用了 `.into()` 方法，这是一个方便的类型转换方法。[[Rust语法特性/概念: Into Trait]]
    //            因为 `Task::new` 的 `title` 和 `description` 参数需要 `String` 和 `Option<String>`，
    //            而我们直接写的是字符串字面量 (`&str`)。
    //            `&str` 类型实现了 `Into<String>` 特性，`&str` 也实现了 `Into<Option<String>>` (会变成 `Some(String::from(..))`)。
    //            所以 `.into()` 可以自动将 `"..."` 转换为所需的 `String` 或 `Option<String>`。
    let tasks = vec![
        Task::new("学习Rust基础".into(), Some("掌握Rust语言的基本语法和概念".into()), false),
        Task::new("学习Axum框架".into(), Some("学习如何使用Axum构建Web应用程序".into()), false),
        Task::new("完成示例项目".into(), Some("构建一个任务管理API应用程序".into()), false)
    ];

    // --- 获取数据库写锁 ---
    // `db.write()`: 尝试获取 `RwLock` 的【写锁】。[[并发原语: 获取写锁]]
    // 如果当前有其他线程持有读锁或写锁，这个调用会【阻塞】，直到锁可用。
    // 返回一个 `RwLockWriteGuard`，它提供了对内部 `HashMap` 的【可变访问权限】。
    // `let mut db_write = ...`: 我们需要将 Guard 绑定到一个【可变】变量，因为我们将通过它修改 HashMap。
    let mut db_write = db.write(); // 获取写锁，类型是 RwLockWriteGuard<HashMap<Uuid, Task>>

    // --- 插入数据 ---
    // 使用 `for` 循环遍历我们创建的示例任务。
    for task in tasks {
        // `db_write.insert(key, value)`: 调用 `HashMap` 的 `insert` 方法。[[数据结构: HashMap insert]]
        // `task.id`: 任务的 Uuid 作为键。
        // `task`: 任务结构体作为值。
        // 【所有权】: `task` 的所有权在这里被【移动】到 `insert` 方法中，最终存储在 HashMap 里。
        //           由于 `for` 循环每次迭代都会消耗一个 `task`，这是符合预期的。
        //           如果我们之后还想使用 `task`，则需要在插入前 `.clone()` 它。
        db_write.insert(task.id, task);
    }

    // --- 锁的自动释放 (RAII) ---
    // 当 `init_sample_data` 函数执行完毕，`db_write` 这个 `RwLockWriteGuard` 变量会离开作用域。
    // Rust 的 RAII 机制会自动调用 `db_write` 的 `drop` 方法，从而【释放写锁】。
    // 这确保了锁总能被正确释放，即使在函数提前返回或发生 panic 的情况下（除非是严重到无法运行 Drop 的情况）。
}

// --- CRUD 操作函数 ---
// 这些函数提供了数据库的基本操作接口，供 Service 层调用。

/// 在数据库中创建新任务 (Create Operation)
///
/// 【功能】: 接收任务的基本信息，创建一个新的 `Task` 实例，并将其存入数据库。
///
/// # 【参数】
/// * `db: &Db` - 数据库实例的引用。
/// * `title: String` - 新任务的标题。[[所有权: 移动]]
/// * `description: Option<String>` - 新任务的可选描述。[[所有权: 移动]]
/// * `completed: bool` - 新任务的初始完成状态。[[所有权: 拷贝]]
///
/// # 【返回值】
/// * `-> Result<Task>`: 使用了我们定义的 `Result` 类型别名 (`Result<T, AppError>`)。
///    - `Ok(Task)`: 如果创建成功，返回新创建的任务的【副本】。
///    - `Err(AppError)`: 如果在创建过程中发生错误（虽然在这个简单的内存实现中不太可能，但在真实数据库交互中可能发生）。
pub fn create_task(
    db: &Db, // 数据库引用
    title: String, // 标题 (所有权移入)
    description: Option<String>, // 描述 (所有权移入)
    completed: bool // 完成状态 (值拷贝)
) -> Result<Task> {
    // 返回自定义 Result
    // 1. 创建 Task 实例
    // 调用我们之前在 model/task.rs 中定义的 Task::new 关联函数。
    let task = Task::new(title, description, completed);

    // 2. （可选）保存 ID，以便后续使用（例如打印日志或直接返回）
    // 因为 `task` 的所有权将在下一步被移动到 HashMap 中。
    // Uuid 是 Copy 的，所以这里是拷贝。
    let id = task.id;

    // 3. 获取写锁并插入数据
    // `db.write()` 获取写锁。
    // `.insert(id, task.clone())`: 插入数据。
    //    - **重要**: 这里使用了 `task.clone()`。[[所有权: 克隆]]
    //      因为 `insert` 方法需要获取 `task` 的所有权，但我们还想在函数末尾返回 `task`。
    //      如果不克隆，`task` 的所有权会被移动到 HashMap 中，我们就无法在 `Ok(task)` 中返回它了。
    //      克隆创建了一个 `Task` 的独立副本存入 HashMap。
    // 【性能考量】: 克隆可能涉及堆内存分配（特别是对于 String 字段），如果 Task 很大或创建操作非常频繁，需要考虑性能影响。
    // 【替代方案】: 可以先插入，然后通过 ID 再从 HashMap 中 get 并 clone 出来返回，但这会涉及两次锁操作（一次写，一次读）。当前实现（先 clone 再插入）通常更简洁。
    db.write().insert(id, task.clone());
    // 写锁在这里被获取，并在 insert 操作完成后立即释放（因为 RwLockWriteGuard 是临时对象，执行完立即 Drop）。

    // 4. 打印日志 (用于演示)
    println!("DB: 已创建任务 ID: {}", id);

    // 5. 返回结果
    // 将原始的 `task` 实例（不是克隆的那个）包装在 `Ok` 中返回。
    Ok(task)
}

/// 从数据库检索所有任务 (Read All Operation)
///
/// 【功能】: 获取数据库中存储的所有任务。
///
/// # 【参数】
/// * `db: &Db` - 数据库实例的引用。
///
/// # 【返回值】
/// * `-> Vec<Task>`: 返回一个包含所有任务【副本】的向量 (Vector)。
pub fn get_all_tasks(db: &Db) -> Vec<Task> {
    // 1. 获取读锁
    // `db.read()`: 获取 `RwLock` 的【读锁】。[[并发原语: 获取读锁]]
    // 允许多个线程同时执行这个函数（并发读取）。
    // 返回 `RwLockReadGuard`，提供对内部 `HashMap` 的【不可变访问权限】。
    let db_read = db.read(); // 类型是 RwLockReadGuard<HashMap<Uuid, Task>>

    // 2. 获取所有值并克隆
    // `db_read.values()`: 调用 `HashMap` 的 `values()` 方法，返回一个迭代器，该迭代器【借用】 HashMap 中的所有值 (`&Task`)。[[数据结构: HashMap values]]
    // `.cloned()`: 对迭代器中的每个元素（`&Task`）调用 `.clone()` 方法，生成一个新的迭代器，其元素是拥有的 `Task` 副本。[[迭代器方法: cloned]]
    // `.collect()`: 将迭代器中的所有元素（`Task` 副本）收集到一个新的 `Vec<Task>` 中。[[迭代器方法: collect]]
    // **为什么需要克隆?** 因为 `db_read` 是一个读锁 Guard，它持有着对 HashMap 的借用。
    // 我们不能直接返回一个包含对锁内部数据引用 (`&Task`) 的 Vec，因为一旦 `db_read` 离开作用域，锁被释放，这些引用就会失效（悬垂引用），违反 Rust 的借用规则。
    // 通过克隆，我们创建了独立的数据副本，可以安全地返回给调用者。
    let tasks: Vec<Task> = db_read.values().cloned().collect();

    // 3. 打印日志 (用于演示)
    println!("DB: 获取了所有任务，共 {} 个", tasks.len());

    // 4. 返回任务向量
    // 读锁 `db_read` 在函数结束时自动释放。
    tasks
}

/// 根据 ID 从数据库检索特定任务 (Read One Operation)
///
/// 【功能】: 根据提供的 UUID 查找并返回对应的任务。
///
/// # 【参数】
/// * `db: &Db` - 数据库实例的引用。
/// * `id: Uuid` - 要查找的任务的 UUID。[[所有权: 拷贝]] (Uuid 实现了 Copy)
///
/// # 【返回值】
/// * `-> Result<Task>`:
///    - `Ok(Task)`: 如果找到任务，返回该任务的【副本】。
///    - `Err(AppError::TaskNotFound)`: 如果数据库中不存在具有该 ID 的任务。
pub fn get_task_by_id(db: &Db, id: Uuid) -> Result<Task> {
    // 1. 获取读锁
    let db_read = db.read();

    // 2. 在 HashMap 中查找任务
    // `db_read.get(&id)`: 调用 `HashMap` 的 `get` 方法，传入要查找的键的【引用】。[[数据结构: HashMap get]]
    // 返回一个 `Option<&Task>`。
    //   - `Some(&Task)`: 如果找到键，返回对值的【不可变引用】。
    //   - `None`: 如果找不到键。
    match db_read.get(&id) {
        // 3. 处理找到的情况
        Some(task_ref) => {
            // task_ref 是一个 &Task 类型，是对锁内数据的引用
            // 打印日志 (用于演示)
            println!("DB: 获取到任务 ID: {}", id);

            // 4. 克隆并返回
            // 同样，因为不能返回对锁内数据的引用 `task_ref`，我们需要克隆它。
            // `task_ref.clone()` 调用 `Task` 的 `clone` 方法 (我们之前 derive 了 Clone)。
            Ok(task_ref.clone())
        }
        // 5. 处理未找到的情况
        None => {
            // 打印日志 (用于演示)
            println!("DB: 任务 ID: {} 未找到", id);
            // 使用我们错误模块中定义的辅助函数创建并返回 TaskNotFound 错误。
            Err(task_not_found(id))
        }
    }
    // 读锁 `db_read` 在 match 结束后或函数返回时自动释放。
}

/// 更新数据库中的任务 (Update Operation)
///
/// 【功能】: 根据 ID 查找任务，并使用提供的新值（如果存在）更新其字段。
///
/// # 【参数】
/// * `db: &Db` - 数据库实例的引用。
/// * `id: Uuid` - 要更新的任务的 UUID。[[所有权: 拷贝]]
/// * `title: Option<String>` - 新的任务标题（如果为 `Some` 则更新）。[[所有权: 移动]]
/// * `description: Option<String>` - 新的任务描述（无条件更新，`None` 表示清除）。[[所有权: 移动]]
/// * `completed: Option<bool>` - 新的完成状态（如果为 `Some` 则更新）。[[所有权: 拷贝]] (Option<bool> 是 Copy)
///
/// # 【返回值】
/// * `-> Result<Task>`:
///    - `Ok(Task)`: 如果更新成功，返回更新后任务的【副本】。
///    - `Err(AppError::TaskNotFound)`: 如果数据库中不存在具有该 ID 的任务。
pub fn update_task(
    db: &Db,
    id: Uuid,
    title: Option<String>,
    description: Option<String>,
    completed: Option<bool>
) -> Result<Task> {
    // 1. 获取写锁
    // 因为我们要修改 HashMap 中的 Task，所以需要写锁。
    let mut db_write = db.write(); // 获取写锁

    // 2. 查找可变的任务引用
    // `db_write.get_mut(&id)`: 调用 `HashMap` 的 `get_mut` 方法。[[数据结构: HashMap get_mut]]
    // 返回一个 `Option<&mut Task>`。
    //   - `Some(&mut Task)`: 如果找到，返回对值的【可变引用】，允许我们直接修改 HashMap 中的 Task。
    //   - `None`: 如果找不到。
    if let Some(task_ref_mut) = db_write.get_mut(&id) {
        // task_ref_mut 是 &mut Task
        // 3. 更新字段 (如果提供了新值)
        //    直接修改 `task_ref_mut` 的字段，就是在修改 HashMap 中存储的那个 Task 实例。

        // 更新标题 (只有在 title is Some 时更新)
        if let Some(new_title) = title {
            // `new_title` (String) 的所有权被移动到 `task_ref_mut.title`。
            task_ref_mut.title = new_title;
        }

        // 更新描述 (总是更新，Some 或 None)
        // `description` (Option<String>) 的所有权被移动到 `task_ref_mut.description`。
        // 如果传入的是 `None`，效果就是清除了描述。
        task_ref_mut.description = description;

        // 更新完成状态 (只有在 completed is Some 时更新)
        if let Some(new_completed) = completed {
            // `new_completed` (bool) 的值被拷贝到 `task_ref_mut.completed`。
            task_ref_mut.completed = new_completed;
        }

        // 4. 更新 `updated_at` 时间戳
        // 无论哪些字段被修改，都应该更新最后修改时间。
        task_ref_mut.updated_at = std::time::SystemTime
            ::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("系统时间错误，早于 EPOCH")
            .as_secs();

        // 5. 打印日志 (用于演示)
        println!("DB: 已更新任务 ID: {}", id);

        // 6. 克隆并返回更新后的任务
        // 同样，需要克隆才能将数据安全地传出锁的作用域。
        Ok(task_ref_mut.clone())
    } else {
        // 7. 处理未找到的情况
        println!("DB: 更新失败，任务 ID: {} 未找到", id);
        Err(task_not_found(id))
    }
    // 写锁 `db_write` 在 if/else 结束后或函数返回时自动释放。
}

/// 从数据库中删除任务 (Delete Operation)
///
/// 【功能】: 根据 ID 查找并移除任务。
///
/// # 【参数】
/// * `db: &Db` - 数据库实例的引用。
/// * `id: Uuid` - 要删除的任务的 UUID。[[所有权: 拷贝]]
///
/// # 【返回值】
/// * `-> Result<Task>`:
///    - `Ok(Task)`: 如果删除成功，返回被删除的那个任务实例的【所有权】。
///    - `Err(AppError::TaskNotFound)`: 如果数据库中不存在具有该 ID 的任务。
pub fn delete_task(db: &Db, id: Uuid) -> Result<Task> {
    // 1. 获取写锁
    // 因为我们要从 HashMap 中移除元素，所以需要写锁。
    let mut db_write = db.write(); // 获取写锁

    // 2. 尝试移除任务
    // `db_write.remove(&id)`: 调用 `HashMap` 的 `remove` 方法。[[数据结构: HashMap remove]]
    // 它会根据键 `&id` 查找元素，如果找到，就将其从 HashMap 中【移除】并返回。
    // 返回一个 `Option<Task>`。
    //   - `Some(Task)`: 如果找到并成功移除，返回被移除的那个 `Task` 实例的【所有权】。
    //   - `None`: 如果找不到具有该键的元素。
    match db_write.remove(&id) {
        // 3. 处理删除成功的情况
        Some(removed_task) => {
            // 打印日志 (用于演示)
            println!("DB: 已删除任务 ID: {}", id);

            // 4. 返回被删除的任务
            // 因为 `remove` 方法直接返回了任务的所有权，我们不需要克隆。
            Ok(removed_task)
        }
        // 5. 处理未找到的情况
        None => {
            println!("DB: 删除失败，任务 ID: {} 未找到", id);
            Err(task_not_found(id))
        }
    }
    // 写锁 `db_write` 在 match 结束后或函数返回时自动释放。
}
