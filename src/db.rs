// src/db.rs

// /--------------------------------------------------------------------------------------------------\
// |                                      【模块功能图示】                                        |
// |--------------------------------------------------------------------------------------------------|
// |                                                                                                  |
// |   +-----------------+      +---------------------------------------------------------------+   |
// |   | 服务层 (Service Layer) | ---> |                     db.rs (本模块)                              |   |
// |   | (调用者)        |      |                                                               |   |
// |   +-----------------+      |   +---------------------------------------------------------+   |   |
// |                            |   | DatabaseConnection (SeaORM)                               |   |   |
// |                            |   |  - 代表到数据库的活动连接池。                             |   |   |
// |                            |   |  - 通过它执行所有数据库操作。                             |   |   |
// |                            |   +---------------------------------------------------------+   |   |
// |                            |                  /|\       |                                |   |   |
// |                            |                   |        |                                |   |   |
// |                            |  +----------------+--------+------------------------------+   |   |
// |                            |  | 公共函数 (Public Functions):                             |   |   |
// |                            |  |  - establish_connection(db_url: &str) -> Result<DatabaseConnection, DbErr> |   |   |
// |                            |  |  - run_migrations(db: &DatabaseConnection) -> Result<(), DbErr> |   |   |
// |                            |  |    - Schema::create_table_from_entity(user_entity::Entity) |   |   |
// |                            |  +---------------------------------------------------------+   |   |
// |                                                                                                  |
// \--------------------------------------------------------------------------------------------------/
//
// 文件路径: src/db.rs
//
// 【模块核心职责】
// 这个模块是应用程序的【数据访问层 (Data Access Layer, DAL)】，负责与数据库的交互。
// 它使用 SeaORM 来建立连接、管理数据库模式（通过迁移），并提供执行数据库操作的连接实例。
//
// 【主要内容】
// 1.  **`establish_connection()`**: 异步函数，根据提供的数据库 URL 创建并返回一个 `DatabaseConnection`。
//     这个连接通常是一个连接池，由 SeaORM 管理。
// 2.  **`run_migrations()`**: 异步函数，用于在应用程序启动时自动创建数据库表。
//     - 对于简单场景（如本项目只有一个 User 表），它直接使用 `Schema::create_table_from_entity` 来从实体定义创建表。
//     - 【注意】: 对于更复杂的应用程序，通常会使用 SeaORM CLI 生成和管理迁移文件 (e.g., `sea-orm-cli migrate init`, `sea-orm-cli migrate generate <migration_name>`)。
//       这些迁移文件提供了对数据库模式进行版本控制和更精细操作（如修改表、添加索引等）的能力。
//       `create_table_from_entity` 适用于快速原型开发或表结构非常简单且不常变动的场景。
//
// 【关键技术点】
// - **`sea_orm`**: 一个现代的、动态的 Rust ORM (对象关系映射器)。
//   - `Database::connect(db_url)`: 用于根据连接字符串建立到数据库的连接。
//   - `DatabaseConnection`: 代表一个数据库连接（通常是池化的）。所有数据库操作都通过它进行。
//   - `Schema`: 用于数据库模式操作，如创建、删除、修改表。
//   - `EntityTrait`: SeaORM 实体（如 `user_entity::Entity`）实现了这个 trait，提供了与数据库表交互的方法。
//   - `DbErr`: SeaORM 操作可能返回的错误类型。
// - **异步 (`async/await`)**: 数据库操作通常是 I/O 密集型的，使用 `async/await` 可以非阻塞地执行这些操作，提高应用程序的并发能力和响应性。

// --- 导入依赖 ---
use sea_orm::{Database, DatabaseConnection, DbErr, EntityTrait, Schema}; // SeaORM核心组件
use crate::app::model::user_entity; // 导入我们定义的 User 实体

// --- 数据库连接与迁移函数 ---

/// 建立数据库连接 (Establish Database Connection)
///
/// 【功能】: 根据提供的数据库 URL 异步连接到数据库。
/// 【参数】: `db_url: &str` - 数据库连接字符串 (例如, "sqlite:./db/app.db").
/// 【返回值】: `Result<DatabaseConnection, DbErr>`
///    - `Ok(DatabaseConnection)`: 成功连接到数据库，返回连接实例。
///    - `Err(DbErr)`: 连接失败，返回 SeaORM 数据库错误。
pub async fn establish_connection(db_url: &str) -> Result<DatabaseConnection, DbErr> {
    println!("DB: 正在尝试连接到数据库: {}", db_url);
    // `Database::connect` 是 SeaORM提供的异步函数，用于创建数据库连接。
    // 它会根据 URL 中的协议 (sqlite, postgres, mysql) 选择合适的驱动。
    let db_conn = Database::connect(db_url).await?;
    println!("DB: 数据库连接已成功建立。");
    Ok(db_conn)
}

/// 运行数据库迁移 (Run Database Migrations)
///
/// 【功能】: 异步执行数据库模式迁移。在此项目中，它会创建 `users` 表 (如果尚不存在)。
/// 【参数】: `db: &DatabaseConnection` - 活动的数据库连接实例。
/// 【返回值】: `Result<(), DbErr>`
///    - `Ok(())`: 迁移（或表创建）成功。
///    - `Err(DbErr)`: 操作失败。
///
/// 【迁移策略说明】:
///   - 本函数使用 `Schema::create_table_from_entity`，这是一种声明式的方式，
///     SeaORM 会检查表是否存在，如果不存在则根据 `user_entity::Entity` 的定义创建它。
///     如果表已存在且结构不同，此方法可能不会更新它（具体行为取决于数据库和SeaORM版本）。
///   - **生产环境推荐**: 使用 SeaORM CLI 管理的迁移文件。迁移文件提供了更强大和可控的
///     数据库模式演进方案，包括版本控制、数据迁移、复杂模式更改等。
///     例如:
///     1. `sea-orm-cli migrate init` - 初始化迁移目录。
///     2. `sea-orm-cli migrate generate create_users_table` - 生成一个新的迁移文件。
///     3. 在迁移文件中定义 `up` 和 `down` SQL 或使用 SeaORM 的 Schema API。
///     4. `sea-orm-cli migrate up` - 应用迁移。
///     在应用启动时，可以调用 `MigratorTrait::up(&db, None).await` 来应用所有待处理的迁移。
pub async fn run_migrations(db: &DatabaseConnection) -> Result<(), DbErr> {
    println!("DB: 准备运行数据库迁移 (创建 users 表)...");

    // 获取数据库后端类型 (e.g., Sqlite, Postgres, MySql)
    let db_backend = db.get_database_backend();
    // 创建一个新的 Schema 构建器实例
    let schema = Schema::new(db_backend);

    // 从 user_entity::Entity 定义创建 users 表
    // `create_table_from_entity` 会生成相应的 `CREATE TABLE IF NOT EXISTS ...` SQL语句。
    // 它会检查表是否存在，如果不存在，则根据实体定义创建它。
    // 这个方法是幂等的，多次运行通常是安全的。
    db.execute(
        db_backend.build(
            schema
                .create_table_from_entity(user_entity::Entity) // 定义要创建的表
                .if_not_exists(), // 确保只在表不存在时创建
        )
    ).await?;

    println!("DB: 'users' 表已成功创建或已存在。数据库迁移完成。");
    Ok(())
}
