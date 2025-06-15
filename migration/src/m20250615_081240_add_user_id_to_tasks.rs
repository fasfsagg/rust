use sea_orm_migration::prelude::*;
use sea_orm::ConnectionTrait;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 检查 user_id 列是否已经存在
        let db = manager.get_connection();
        let check_query = "PRAGMA table_info(tasks)";
        let result = db.query_all(
            sea_orm::Statement::from_string(manager.get_database_backend(), check_query.to_string())
        ).await?;

        // 检查是否已经有 user_id 列
        let user_id_exists = result.iter().any(|row| {
            if let Some(name) = row.try_get::<String>("", "name").ok() {
                name == "user_id"
            } else {
                false
            }
        });

        // 只有当 user_id 列不存在时才添加
        if !user_id_exists {
            // 为 tasks 表添加 user_id 列，先设为可空
            manager.alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .add_column(ColumnDef::new(Tasks::UserId).uuid().null())
                    .to_owned()
            ).await?;
        }

        // SQLite 不支持向现有表添加外键约束
        // 我们只添加索引以优化查询性能
        // 外键约束将在应用层面通过 SeaORM 的关系定义来维护

        // 检查索引是否已经存在
        let index_check_query =
            "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_tasks_user_id'";
        let index_result = db.query_all(
            sea_orm::Statement::from_string(
                manager.get_database_backend(),
                index_check_query.to_string()
            )
        ).await?;

        // 只有当索引不存在时才创建
        if index_result.is_empty() {
            manager.create_index(
                Index::create()
                    .name("idx_tasks_user_id")
                    .table(Tasks::Table)
                    .col(Tasks::UserId)
                    .to_owned()
            ).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 删除索引
        manager.drop_index(
            Index::drop().name("idx_tasks_user_id").table(Tasks::Table).to_owned()
        ).await?;

        // 删除 user_id 列
        manager.alter_table(
            Table::alter().table(Tasks::Table).drop_column(Tasks::UserId).to_owned()
        ).await
    }
}

/// 定义 tasks 表的标识符
#[derive(DeriveIden)]
enum Tasks {
    Table,
    UserId,
}
