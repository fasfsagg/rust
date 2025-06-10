use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// `up` 函数定义了当应用此迁移时数据库应该发生的变化。
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 使用 `SchemaManager` 的 `create_table` 方法来创建一个新表。
        manager.create_table(
            // `Table::create()` 是构建表定义的起点。
            Table::create()
                // `.table(Task::Table)` 指定了我们要创建的表的名称。
                // `Task::Table` 来自下面的 `Task` Iden 枚举。
                .table(Task::Table)
                // `.if_not_exists()` 确保如果表已存在，则不会抛出错误。
                .if_not_exists()
                // 定义 `id` 列。
                .col(
                    // `ColumnDef::new(Task::Id)` 创建一个新列定义。
                    // `uuid()` 设置列类型为 UUID。
                    // `.not_null()` 确保此列不能为 NULL。
                    // `.primary_key()` 将其设置为主键。
                    // `.default(Expr::cust("uuid_generate_v4()"))` 这在PostgreSQL中有效，
                    // 但对于 SQLite，SeaORM 会在插入时自动处理 Uuid::new_v4()。
                    // 或者我们可以在实体中定义默认值。这里我们依赖 SeaORM 的行为。
                    ColumnDef::new(Task::Id).uuid().not_null().primary_key()
                )
                // 定义 `title` 列。
                .col(ColumnDef::new(Task::Title).string().not_null())
                // 定义 `description` 列。
                // `.text()` 设置列类型为 TEXT，适合长文本。
                .col(ColumnDef::new(Task::Description).text())
                // 定义 `completed` 列。
                // `.default(false)` 为此列设置了默认值。
                .col(ColumnDef::new(Task::Completed).boolean().not_null().default(false))
                // 定义 `created_at` 列。
                // `.timestamp_with_time_zone()` 是存储带时区时间戳的推荐方式。
                // `Expr::current_timestamp()` 设置默认值为数据库的当前时间。
                .col(
                    ColumnDef::new(Task::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp())
                )
                // 定义 `updated_at` 列。
                .col(
                    ColumnDef::new(Task::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp())
                )
                // `.to_owned()` 完成表定义的构建。
                .to_owned()
        ).await
    }

    /// `down` 函数定义了当回滚此迁移时应该发生的变化。
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 使用 `SchemaManager` 的 `drop_table` 方法来删除表。
        manager.drop_table(Table::drop().table(Task::Table).to_owned()).await
    }
}

/// `Iden` 是 SeaORM 用于表示数据库标识符（如表名、列名）的通用 trait。
/// 我们创建一个 `Task` 枚举来实现 `Iden`，以便在迁移脚本中安全地引用这些名称。
#[derive(DeriveIden)]
enum Task {
    // `Table` 成员用于表示表名，在我们的例子中是 "tasks"。
    #[sea_orm(iden = "tasks")]
    Table,
    // 以下成员代表表中的列名。
    Id,
    Title,
    Description,
    Completed,
    CreatedAt,
    UpdatedAt,
}
