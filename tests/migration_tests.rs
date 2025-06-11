//! tests/migration_tests.rs

use common::TestApp;
use migration::{ Migrator, MigratorTrait };
use sea_orm::{ ConnectionTrait, Statement, DatabaseConnection, DbErr };

// 引入我们精心准备的测试辅助模块
mod common;

/// 验证应用启动时，数据库迁移是否能在空数据库上成功运行。
///
/// ## 测试步骤:
/// 1. 调用 `spawn_app()`，该函数内部会:
///    - 创建一个全新的、基于文件的 SQLite 临时数据库。
///    - 运行 `init_app`，其中包含了数据库迁移逻辑。
/// 2. 获取 `TestApp` 实例，其中包含了到该临时数据库的连接。
/// 3. 使用原生 SQL 查询 `PRAGMA table_info(tasks);` 来检查 `tasks` 表的结构。
/// 4. 断言查询成功，证明 `tasks` 表已按预期被创建。
#[tokio::test]
async fn test_migrations_on_empty_database() {
    // 1. 启动应用，这将自动触发数据库迁移
    let app: TestApp = common::spawn_app().await;

    // 2. 使用返回的数据库连接来验证迁移结果
    // 我们直接检查 'tasks' 表是否存在，这是迁移脚本应该创建的。
    let result = check_table_exists(&app.db_connection, "tasks").await;

    // 3. 断言表存在
    assert!(result.is_ok(), "数据库迁移后，'tasks' 表应该存在, 但检查时出错: {:?}", result.err());
    assert!(result.unwrap(), "数据库迁移后，'tasks' 表应该存在, 但未找到");
}

/// 验证数据库迁移是"幂等"的，即重复运行不会产生错误或副作用。
///
/// ## 测试步骤:
/// 1. 第一次调用 `spawn_app()`。
///    - 这会在一个全新的数据库上运行迁移。
///    - 我们断言这次调用是成功的。
/// 2. 在同一个数据库上，再次模拟应用启动过程。
///    - 为了做到这一点，我们直接在第一次的数据库连接上运行迁移函数。
///    - (注意：这里我们不能再次调用 `spawn_app`，因为它会创建一个全新的数据库。
///      所以我们直接调用 `axum_tutorial::migration::run_migrations`)
/// 3. 断言第二次运行迁移也没有返回任何错误。
///    这证明了迁移脚本被设计为可以安全地重复执行。
#[tokio::test]
async fn test_migrations_are_idempotent() {
    // 1. 第一次启动应用并运行迁移
    let app = common::spawn_app().await;

    // 2. 直接在同一个数据库连接上再次运行迁移
    // 这一步模拟了应用重启时，迁移逻辑再次被触发的场景。
    let migration_result = Migrator::up(&app.db_connection, None).await;

    // 3. 断言第二次迁移没有产生错误
    assert!(
        migration_result.is_ok(),
        "重复运行数据库迁移不应该导致错误，但收到了: {:?}",
        migration_result.err()
    );
}

/// 辅助函数，用于检查指定的表是否存在于数据库中。
///
/// 在 SQLite 中，查询 `sqlite_master` 是检查表、索引等对象是否存在的标准方法。
///
/// ## 参数:
/// - `db`: 一个到数据库的 `DatabaseConnection` 连接。
/// - `table_name`: 要检查的表的名称。
///
/// ## 返回:
/// - `Ok(true)` 如果表存在。
/// - `Ok(false)` 如果表不存在。
/// - `Err(DbErr)` 如果查询执行失败。
async fn check_table_exists(db: &DatabaseConnection, table_name: &str) -> Result<bool, DbErr> {
    let query =
        format!("SELECT name FROM sqlite_master WHERE type='table' AND name='{}';", table_name);
    let result = db.query_one(Statement::from_string(db.get_database_backend(), query)).await?;
    Ok(result.is_some())
}
