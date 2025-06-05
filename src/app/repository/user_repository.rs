// 文件路径: src/app/repository/user_repository.rs

// /--------------------------------------------------------------------------------------------------\
// |                               【模块功能图示】 (user_repository.rs)                                |
// |--------------------------------------------------------------------------------------------------|
// |                                                                                                  |
// |  [服务层 (AuthService)]                                                                           |
// |   - 调用 `UserRepository::find_by_username(...)`                                                 |
// |   - 调用 `UserRepository::create_user(...)`                                                      |
// |      |                                                                                           |
// |      V (方法调用)                                                                                  |
// |  [数据仓库层 (`UserRepository`)]                                                                   |
// |   - `find_by_username(db, username)`:                                                            |
// |     - 使用 SeaORM 构建查询: `UserEntity::find().filter(Column::Username.eq(username)).one(db)`    |
// |   - `create_user(db, user_data)`:                                                                |
// |     - 使用 SeaORM 执行插入: `user_data.insert(db)`                                                 |
// |     - (然后根据返回的 ID 重新获取完整的 Model)                                                      |
// |      |                                                                                           |
// |      V (SeaORM 查询/命令)                                                                          |
// |  [SeaORM ORM 框架]                                                                                |
// |      |                                                                                           |
// |      V (SQL 生成与执行)                                                                            |
// |  [数据库 (SQLite)]                                                                                |
// |    - 表: `users`                                                                                  |
// |                                                                                                  |
// \--------------------------------------------------------------------------------------------------/
//
// 【模块核心职责】 (Module Core Responsibilities)
// 1. **数据访问抽象 (Data Access Abstraction)**: 提供一个清晰的接口（`UserRepository` 结构体及其方法）用于对 `User` 实体（用户数据）进行数据库操作。
//    它将底层的数据库交互细节（如具体的 SeaORM 查询语句）封装起来。
// 2. **与数据库交互 (Interact with the Database)**: 执行实际的数据库查询和命令，如查找用户、创建用户等。
//    所有与 `users` 表相关的 SQL 操作都应通过这个仓库模块进行。
// 3. **服务层依赖 (Dependency for Service Layer)**: `UserRepository` 被服务层（如 `AuthService`）调用，
//    为业务逻辑提供所需的数据支持，同时使服务层代码不直接依赖于具体的 ORM (SeaORM) 或数据库细节。
//    这有助于分层和解耦。
//
// 【关键技术点】 (Key Technologies)
// - **SeaORM**: Rust 的异步 ORM 框架，用于与数据库交互。
//   - `DatabaseConnection`: 代表数据库连接（通常是连接池），所有数据库操作都通过它执行。
//   - `DbErr`: SeaORM 操作可能返回的错误类型。
//   - `EntityTrait` (通过 `user_entity::Entity` 或别名 `UserEntity` 使用): 提供了如 `find()`, `find_by_id()` 等查询构建的起点。
//   - `ColumnTrait` (通过 `user_entity::Column` 使用): 用于在查询中引用表的列，例如 `user_entity::Column::Username`。
//   - `QueryFilter` (通过 `.filter()` 方法使用): 用于向查询添加 `WHERE` 条件。
//   - `ActiveModelTrait` (通过 `user_entity::ActiveModel` 的实例使用): 提供了如 `insert()`, `save()`, `update()` 等用于数据修改的方法。
//   - `user_entity::Model`: 由 SeaORM 根据实体定义生成的结构体，代表从数据库读取的一条记录。
//   - `user_entity::ActiveModel`: 用于创建新记录或更新现有记录的结构体。字段是 `ActiveValue` 类型 (`Set(value)` 或 `NotSet`)。
// - **异步编程 (`async/await`)**: 由于数据库操作本质上是 I/O 密集型的，`UserRepository` 中的所有方法都是异步的 (`async fn`)，
//   使用 `.await` 来等待数据库操作完成，而不会阻塞当前线程。
// - **`Result<T, DbErr>`**: 所有与数据库交互的方法都返回 `Result`，表示操作可能成功并返回值 `T`，或者失败并返回 SeaORM 的 `DbErr`。
// - **Rust 结构体和方法 (`struct UserRepository`, `impl UserRepository`)**: 用于组织相关的函数（方法）到一个逻辑单元中。
// - **引用传递 (`&DatabaseConnection`, `&str`)**: 为了效率和遵循 Rust 的所有权规则，数据库连接和查询参数通常通过引用传递。

// --- 导入依赖 ---
// `use sea_orm::{...};`
//   - 从 `sea_orm` crate 中导入一系列常用的 traits 和类型。
//   - `DatabaseConnection`: 代表一个活动的数据库连接（通常是连接池）。所有数据库操作都通过它异步执行。
//   - `DbErr`: SeaORM 定义的错误类型，用于表示数据库操作中可能发生的各种错误。
//   - `EntityTrait`: 一个核心 trait，由实体 (entity) 实现，提供了查询构建的入口点，如 `Entity::find()`。
//   - `ColumnTrait`: 用于在查询中引用表的列，并构建条件表达式 (如 `.eq()`, `.like()`, 等)。
//   - `QueryFilter`: 提供了 `.filter()` 方法，用于向查询添加 `WHERE` 子句。
//   - `ActiveModelTrait`: 由活动模型 (active model) 实现，提供了保存 (插入/更新) 数据的方法，如 `.insert()` 或 `.save()`。
//   - `IntoActiveModel`: 一个 trait，允许将 `Model` 转换为 `ActiveModel`，通常用于更新操作。
use sea_orm::{DatabaseConnection, DbErr, EntityTrait, ColumnTrait, QueryFilter, ActiveModelTrait, IntoActiveModel};
// `use crate::app::model::user_entity;`
//   - 导入在 `src/app/model/user_entity.rs` 中定义的 `user_entity` 模块。
//   - 这个模块包含了 `user_entity::Model` (实体模型), `user_entity::ActiveModel` (活动模型),
//     `user_entity::Entity` (实体定义，实现了 `EntityTrait`), 和 `user_entity::Column` (列定义，实现了 `ColumnTrait`)。
//   - 我们需要这些类型来与 `users` 表进行交互。
use crate::app::model::user_entity;

// `#[derive(Debug, Default)]`
// - `Debug`: 自动实现 `std::fmt::Debug` trait，允许使用 `{:?}` 打印 `UserRepository` 实例，方便调试。
// - `Default`: 自动实现 `std::default::Default` trait，允许通过 `UserRepository::default()` 创建一个默认实例。
//   对于空结构体，默认实例就是其本身。
#[derive(Debug, Default)]
// `pub struct UserRepository;`
//   - 定义一个公共的 (public) 结构体 `UserRepository`。
//   - **当前是空结构体**: 在这个实现中，`UserRepository` 是一个“无状态”的仓库。
//     它不持有任何数据（比如数据库连接池 `DatabaseConnection`）。
//     相反，它的所有方法都将 `&DatabaseConnection` 作为参数接收。
//   - **设计选择**:
//     - **无状态仓库**: 优点是结构体本身易于创建 (`UserRepository::new()` 或 `UserRepository::default()`)，
//       并且数据库连接的生命周期由调用者（通常是服务层或应用状态）管理。这使得仓库方法更易于测试（可以传入模拟的连接）。
//     - **有状态仓库 (Stateful Repository)**: 另一种设计是在 `UserRepository` 结构体中包含一个字段来存储 `DatabaseConnection` (通常是 `Arc` 包裹的，或者是一个引用，但这会引入生命周期参数)。
//       例如: `pub struct UserRepository { db: Arc<DatabaseConnection> }`。
//       这种情况下，方法就不需要显式接收 `db` 参数，而是通过 `&self.db` 访问。
//     本项目采用了无状态设计，将连接管理放在服务层或从应用状态传递。
pub struct UserRepository;

// `impl UserRepository { ... }`
//   - 为 `UserRepository` 结构体实现方法。
impl UserRepository {
    // `pub fn new() -> Self { Self }`
    //   - 定义一个公共的关联函数 `new`，用作 `UserRepository` 的构造函数。
    //   - `-> Self`: 返回类型是 `Self`，即 `UserRepository`。
    //   - `Self`: 在这个上下文中，`Self` (大写S) 也指代 `UserRepository` 类型本身。
    //     `Self {}` (如果结构体有字段) 或简写为 `Self` (对于空结构体或使用 `Default` 的情况) 用于创建实例。
    //     由于 `UserRepository` 是空结构体并且派生了 `Default`，`Self` 或 `UserRepository` 或 `UserRepository::default()` 都可以。
    pub fn new() -> Self {
        Self // 对于空结构体，`Self` 就足以创建一个实例。
    }

    // `pub async fn find_by_username(...) -> Result<Option<user_entity::Model>, DbErr>`
    //   - `pub async fn`: 定义一个公共的异步函数。数据库操作通常是 I/O 绑定的，因此设为异步以避免阻塞。
    //   - `db: &DatabaseConnection`: 第一个参数，是对数据库连接的【不可变引用】。
    //     `&`: 表示借用，此函数不获取 `DatabaseConnection` 的所有权。
    //   - `username: &str`: 第二个参数，是要查找的用户名，类型是 `&str` (字符串切片)。
    //     `&str` 是对字符串数据的不可变视图，通常比 `String` 更高效，因为它不涉及堆分配（除非从 `String` 创建）。
    //   - `-> Result<Option<user_entity::Model>, DbErr>`: 函数的返回类型。
    //     - `Result<T, E>`: 表示操作可能成功 (`Ok(T)`) 或失败 (`Err(E)`)。
    //       - `DbErr`: 失败时返回 SeaORM 的数据库错误类型。
    //     - `Option<user_entity::Model>`: 如果操作成功 (`Ok`)，其内部值是一个 `Option`。
    //       - `Some(user_entity::Model)`: 表示找到了用户，并返回该用户的 `Model` 实例。
    //       - `None`: 表示没有找到具有该用户名的用户。
    /// 根据用户名查找用户。
    ///
    /// # 参数
    /// * `db`: 数据库连接的引用。
    /// * `username`: 要查找的用户名。
    ///
    /// # 返回值
    /// * `Ok(Some(user_model))` 如果找到用户。
    /// * `Ok(None)` 如果没有找到具有该用户名的用户。
    /// * `Err(DbErr)` 如果数据库查询过程中发生错误。
    pub async fn find_by_username(
        db: &DatabaseConnection, // 数据库连接的引用
        username: &str,          // 要查找的用户名 (字符串切片)
    ) -> Result<Option<user_entity::Model>, DbErr> { // 返回 Result 包裹的 Option
        // `user_entity::Entity::find()`:
        //   - `user_entity::Entity`: 访问在 `user_entity.rs` 中定义的 `Entity` (它实现了 `EntityTrait`)。
        //   - `.find()`: `EntityTrait` 提供的方法，用于开始构建一个 SELECT 查询，目标是 `users` 表。
        //     返回一个 `Select<UserEntity>` 查询构造器。
        //     概念上类似于 SQL: `SELECT * FROM users ...`
        user_entity::Entity::find()
            // `.filter(user_entity::Column::Username.eq(username))`:
            //   - `.filter()`: `QueryFilter` trait (由 `Select` 实现) 提供的方法，用于向查询添加 `WHERE` 条件。
            //   - `user_entity::Column::Username`: 引用 `users` 表的 `username` 列。
            //     `user_entity::Column` 是 SeaORM 根据实体定义自动生成的枚举，其每个变体代表一个表列。
            //   - `.eq(username)`: `ColumnTrait` 提供的方法，用于构建一个 "等于" (`=`) 的条件。
            //     这里表示 `WHERE username = ?`，其中 `?` 会被 `username` 参数的值替换 (由 SeaORM 安全地处理以防止 SQL 注入)。
            .filter(user_entity::Column::Username.eq(username))
            // `.one(db)`: 执行构建好的查询，并期望最多返回一条记录。
            //   - `db`: 传入数据库连接。
            //   - 它返回 `Result<Option<Model>, DbErr>`。
            //     - `Ok(Some(model))` 如果找到唯一匹配的记录。
            //     - `Ok(None)` 如果没有找到匹配的记录。
            //     - `Err(DbErr)` 如果数据库操作出错。
            //     (如果查询可能返回多条记录但只想取第一条，可以使用 `.one()`；如果确定只有一条或零条，它也是合适的。)
            .one(db)
            // `.await`: 因为数据库查询是异步操作，所以需要 `.await` 来等待其完成。
            // `?` 操作符: 在这里不直接使用 `?`，因为 `one(db).await` 本身就返回 `Result<Option<Model>, DbErr>`，
            // 这与函数的签名完全匹配，所以可以直接返回这个 `Result`。
            // 如果我们想在出错时做一些额外处理或转换错误类型，才会用 `?` 或 `match`。
            .await
    }

    // `pub async fn create_user(...) -> Result<user_entity::Model, DbErr>`
    //   - `pub async fn`: 公共异步函数。
    //   - `user_data: user_entity::ActiveModel`: 参数，是一个 `user_entity::ActiveModel` 实例。
    //     - `ActiveModel` 是 SeaORM 用于插入和更新操作的特殊结构体。它的字段是 `ActiveValue<T>` 类型，
    //       可以是 `Set(value)` (表示要将字段设置为 `value`) 或 `NotSet` (表示此字段在操作中不被修改，或使用数据库默认值)。
    //     - 调用者 (例如 `AuthService`) 在调用此函数前，应已将用户名和哈希后的密码设置到 `user_data` 中。
    //   - `-> Result<user_entity::Model, DbErr>`: 返回类型。
    //     - 成功时 (`Ok`) 返回新创建的用户的完整 `user_entity::Model` 实例 (包含了数据库生成的 ID 和默认时间戳)。
    //     - 失败时 (`Err`) 返回 `DbErr`。
    /// 创建一个新用户。
    /// `user_data` (ActiveModel) 应已包含用户名和哈希后的密码。
    ///
    /// # 参数
    /// * `db`: 数据库连接的引用。
    /// * `user_data`: 包含新用户数据的 `ActiveModel`。
    ///
    /// # 返回值
    /// * `Ok(user_model)` 如果用户成功创建，返回包含所有字段 (包括数据库生成的 ID) 的用户模型。
    /// * `Err(DbErr)` 如果数据库插入操作失败。
    pub async fn create_user(
        db: &DatabaseConnection,
        user_data: user_entity::ActiveModel, // 包含待插入数据的活动模型
    ) -> Result<user_entity::Model, DbErr> {
        // --- 步骤 1: 执行插入操作 ---
        // `user_data.insert(db).await`:
        //   - `user_data` 是一个 `ActiveModel` 实例。
        //   - `.insert(db)`: `ActiveModelTrait` 提供的方法，用于将 `ActiveModel` 中的数据插入到数据库对应的表中。
        //     它会生成并执行一条 SQL `INSERT` 语句。
        //   - `.await`: 异步等待插入操作完成。
        //   - `insert` 方法通常返回 `Result<I, DbErr>`，其中 `I` 的类型取决于主键的设置和 SeaORM 版本。
        //     对于自动递增的主键，较新版本的 SeaORM (如 0.12+) 的 `insert` 方法可以直接返回 `Result<Model, DbErr>`。
        //     在此前的版本或某些配置下，它可能返回一个包含 `last_insert_id` 的 `InsertResult`，或者返回更新后的 `ActiveModel`。
        //     当前代码基于 `user_data.insert(db).await` 返回 `Result<user_entity::ActiveModel, DbErr>` 的假设，
        //     这个返回的 `ActiveModel` (即 `new_user_active_model`) 会包含数据库生成的主键 `id` (如果主键是自增的)。
        // `?` 操作符: 如果 `insert` 操作失败 (例如，违反了 `UNIQUE` 约束，如用户名已存在，尽管服务层应该先检查)，
        // 则 `?` 会使 `create_user` 函数立即返回 `Err(DbErr)`。
        let new_user_active_model: user_entity::ActiveModel = user_data.insert(db).await?;
        // `new_user_active_model` 现在是一个 `ActiveModel`，其 `id` 字段 (如果是自增主键) 应该已经被数据库填充了值。
        // 其他在 `user_data` 中未 `Set` 的字段，如果数据库有默认值 (如 `created_at`, `updated_at`)，也可能已被填充，
        // 但 `ActiveModel` 主要保证主键的回填。

        // --- 步骤 2: 从返回的 ActiveModel 中获取新生成的 ID 并重新获取完整的 Model ---
        // 虽然 `new_user_active_model` 可能包含了一些数据库生成的值 (如 ID)，但它仍然是一个 `ActiveModel`，
        // 其字段是 `ActiveValue` 类型。为了得到一个包含所有字段 (包括数据库默认值如 `created_at`, `updated_at`)
        // 且字段类型为其实际 Rust 类型 (如 `i32`, `String`, `DateTimeUtc`) 的 `user_entity::Model` 实例，
        // 最可靠的方法是使用新获得的 ID 从数据库中重新查询 (fetch) 这条记录。

        // `new_user_active_model.id`: 访问 `ActiveModel` 的 `id` 字段。这是一个 `ActiveValue<i32>`。
        // `.as_ref()`: 将 `ActiveValue<i32>` 转换为 `ActiveValue<&i32>`，允许我们借用内部的值而无需消耗 `ActiveValue`。
        //   - 如果 `id` 是 `Set(value)` 或 `Unchanged(value)`，它会变成 `Set(&value)` 或 `Unchanged(&value)`。
        //   - 如果是 `NotSet`，它仍然是 `NotSet`。
        // `if let Some(id_val) = ...`: `if let` 是一种模式匹配，用于检查 `Option` 是否为 `Some` 并解构其内部值。
        //   - `ActiveValue::as_ref()` 实际上返回 `ActiveValue<&T>`，但这里我们期望它是一个具体的值。
        //     更准确地说，`ActiveValue<T>` 有一个 `unwrap()` 方法 (如果确定它被设置了)，或者 `into_value()` (消耗 `ActiveValue`)。
        //     或者，我们可以直接匹配 `ActiveValue` 的变体。
        //     这里的 `id.as_ref()` 可能是指 `Option::as_ref` (如果 `id` 字段本身是 `Option<ActiveValue<i32>>`)
        //     或者更可能是对 `ActiveValue` 的某种处理，期望得到 `Option<&i32>`。
        //     假设 `new_user_active_model.id` 是 `ActiveValue::Set(val)` 或 `ActiveValue::Unchanged(val)`，
        //     我们需要从中提取出 `val`。
        //     一个更清晰的方式可能是:
        //     `let user_id = match new_user_active_model.id {`
        //     `    ActiveValue::Set(id) | ActiveValue::Unchanged(id) => id,`
        //     `    ActiveValue::NotSet => return Err(DbErr::Custom("Primary key not set after insert".into())),`
        //     `};`
        //     当前代码 `new_user_active_model.id.as_ref()` 在 `ActiveValue` 上下文中，
        //     其 `as_ref()` 方法返回 `Option<&T>` (如果 `ActiveValue` 是 `Set` 或 `Unchanged`) 或 `None` (如果 `NotSet`)。
        if let Some(id_val) = new_user_active_model.id.as_ref() { // `id_val` 现在是 `&i32`
             // `user_entity::Entity::find_by_id(*id_val)`:
             //   - `UserEntity::find_by_id()`: `EntityTrait` 提供的方法，用于根据主键 ID 查找记录。
             //   - `*id_val`: 解引用 `&i32` 得到 `i32` 类型的值作为 ID。
             user_entity::Entity::find_by_id(*id_val)
                .one(db) // 执行查询，期望一条或零条记录。
                .await?  // 异步等待，并通过 `?` 传播可能的 `DbErr`。
                // `.ok_or_else(|| ...)`: 将 `Option<Model>` 转换为 `Result<Model, DbErr>`。
                //   - 如果 `one(db).await?` 返回 `Some(model)` (即找到了记录)，则 `ok_or_else` 返回 `Ok(model)`。
                //   - 如果返回 `None` (即刚插入的记录却找不到了，这通常不应该发生)，则执行闭包 `|| ...`。
                //     闭包创建一个新的 `DbErr::RecordNotFound` 错误。
                .ok_or_else(|| DbErr::RecordNotFound("创建用户后未能从数据库取回该用户记录。".to_string()))
        } else {
            // 如果 `new_user_active_model.id.as_ref()` 返回 `None`，意味着 `id` 字段在插入后仍然是 `NotSet`。
            // 这对于一个自增主键来说是不正常的，表明插入操作可能没有按预期回填 ID，或者 `ActiveModel` 的行为与预期不符。
            // 返回一个自定义的数据库错误。
            Err(DbErr::Custom("插入用户后未能获取其主键ID。".to_string()))
        }
    }
}
