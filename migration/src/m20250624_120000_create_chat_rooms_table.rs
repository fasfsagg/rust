use sea_orm_migration::prelude::*;

/// 创建聊天室表的迁移
///
/// 此迁移创建 `chat_rooms` 表，用于存储聊天室信息。
/// 表设计考虑了企业级应用的需求，支持不同类型的聊天室和状态管理。
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 创建聊天室表
        manager
            .create_table(
                Table::create()
                    .table(ChatRooms::Table)
                    .if_not_exists()
                    // 主键：UUID类型的聊天室ID
                    .col(
                        ColumnDef::new(ChatRooms::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    // 聊天室名称：唯一，最大100字符
                    .col(
                        ColumnDef::new(ChatRooms::Name)
                            .string_len(100)
                            .not_null()
                            .unique_key(),
                    )
                    // 聊天室描述：可选的文本字段
                    .col(ColumnDef::new(ChatRooms::Description).text().null())
                    // 聊天室类型：枚举值（public, private, group）
                    .col(
                        ColumnDef::new(ChatRooms::RoomType)
                            .string_len(20)
                            .not_null()
                            .default("public"),
                    )
                    // 聊天室状态：枚举值（active, archived, disabled）
                    .col(
                        ColumnDef::new(ChatRooms::Status)
                            .string_len(20)
                            .not_null()
                            .default("active"),
                    )
                    // 创建者用户ID：外键关联到users表
                    .col(ColumnDef::new(ChatRooms::CreatedBy).uuid().not_null())
                    // 最大成员数量：默认为0（无限制）
                    .col(
                        ColumnDef::new(ChatRooms::MaxMembers)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    // 当前成员数量：默认为0
                    .col(
                        ColumnDef::new(ChatRooms::CurrentMembers)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    // 聊天室设置：JSON格式的扩展配置
                    .col(ColumnDef::new(ChatRooms::Settings).text().null())
                    // 创建时间：自动设置为当前时间
                    .col(
                        ColumnDef::new(ChatRooms::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    // 更新时间：自动设置为当前时间
                    .col(
                        ColumnDef::new(ChatRooms::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    // 外键约束：创建者必须是有效用户
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_chat_rooms_created_by")
                            .from(ChatRooms::Table, ChatRooms::CreatedBy)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 创建索引以优化查询性能
        manager
            .create_index(
                Index::create()
                    .name("idx_chat_rooms_type_status")
                    .table(ChatRooms::Table)
                    .col(ChatRooms::RoomType)
                    .col(ChatRooms::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_chat_rooms_created_by")
                    .table(ChatRooms::Table)
                    .col(ChatRooms::CreatedBy)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_chat_rooms_created_at")
                    .table(ChatRooms::Table)
                    .col(ChatRooms::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 删除索引
        manager
            .drop_index(Index::drop().name("idx_chat_rooms_created_at").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("idx_chat_rooms_created_by").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("idx_chat_rooms_type_status").to_owned())
            .await?;

        // 删除表
        manager
            .drop_table(Table::drop().table(ChatRooms::Table).to_owned())
            .await
    }
}

/// 聊天室表的列定义枚举
#[derive(DeriveIden)]
enum ChatRooms {
    Table,
    Id,
    Name,
    Description,
    RoomType,
    Status,
    CreatedBy,
    MaxMembers,
    CurrentMembers,
    Settings,
    CreatedAt,
    UpdatedAt,
}

/// 用户表的列定义枚举（用于外键引用）
#[derive(DeriveIden)]
enum User {
    #[sea_orm(iden = "user")]
    Table,
    Id,
}
