use sea_orm_migration::prelude::*;

/// 创建消息表的迁移
///
/// 此迁移创建 `messages` 表，用于存储聊天消息。
/// 表设计支持多种消息类型和状态，为企业级聊天应用提供完整的消息管理功能。
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 创建消息表
        manager
            .create_table(
                Table::create()
                    .table(Messages::Table)
                    .if_not_exists()
                    // 主键：UUID类型的消息ID
                    .col(ColumnDef::new(Messages::Id).uuid().not_null().primary_key())
                    // 消息内容：文本类型
                    .col(ColumnDef::new(Messages::Content).text().not_null())
                    // 消息类型：枚举值（text, image, file, system, voice, video）
                    .col(
                        ColumnDef::new(Messages::MessageType)
                            .string_len(20)
                            .not_null()
                            .default("text"),
                    )
                    // 消息状态：枚举值（sent, delivered, read, deleted, edited）
                    .col(
                        ColumnDef::new(Messages::Status)
                            .string_len(20)
                            .not_null()
                            .default("sent"),
                    )
                    // 发送者用户ID：外键关联到users表
                    .col(ColumnDef::new(Messages::SenderId).uuid().not_null())
                    // 目标聊天室ID：外键关联到chat_rooms表
                    .col(ColumnDef::new(Messages::ChatRoomId).uuid().not_null())
                    // 回复的消息ID：可选，用于消息回复功能
                    .col(ColumnDef::new(Messages::ReplyToId).uuid().null())
                    // 消息元数据：JSON格式，存储文件信息、位置信息等
                    .col(ColumnDef::new(Messages::Metadata).text().null())
                    // 消息优先级：0-9，默认为5
                    .col(
                        ColumnDef::new(Messages::Priority)
                            .integer()
                            .not_null()
                            .default(5),
                    )
                    // 是否置顶消息：默认为false
                    .col(
                        ColumnDef::new(Messages::IsPinned)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    // 消息过期时间：可选，用于临时消息
                    .col(
                        ColumnDef::new(Messages::ExpiresAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    // 创建时间：自动设置为当前时间
                    .col(
                        ColumnDef::new(Messages::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    // 更新时间：自动设置为当前时间
                    .col(
                        ColumnDef::new(Messages::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    // 外键约束：发送者必须是有效用户
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_messages_sender_id")
                            .from(Messages::Table, Messages::SenderId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    // 外键约束：聊天室必须存在
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_messages_chat_room_id")
                            .from(Messages::Table, Messages::ChatRoomId)
                            .to(ChatRooms::Table, ChatRooms::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    // 外键约束：回复消息必须存在（如果指定）
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_messages_reply_to_id")
                            .from(Messages::Table, Messages::ReplyToId)
                            .to(Messages::Table, Messages::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 创建索引以优化查询性能
        manager
            .create_index(
                Index::create()
                    .name("idx_messages_chat_room_created_at")
                    .table(Messages::Table)
                    .col(Messages::ChatRoomId)
                    .col(Messages::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_messages_sender_id")
                    .table(Messages::Table)
                    .col(Messages::SenderId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_messages_type_status")
                    .table(Messages::Table)
                    .col(Messages::MessageType)
                    .col(Messages::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_messages_reply_to_id")
                    .table(Messages::Table)
                    .col(Messages::ReplyToId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 删除索引
        manager
            .drop_index(Index::drop().name("idx_messages_reply_to_id").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("idx_messages_type_status").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("idx_messages_sender_id").to_owned())
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_messages_chat_room_created_at")
                    .to_owned(),
            )
            .await?;

        // 删除表
        manager
            .drop_table(Table::drop().table(Messages::Table).to_owned())
            .await
    }
}

/// 消息表的列定义枚举
#[derive(DeriveIden)]
enum Messages {
    Table,
    Id,
    Content,
    MessageType,
    Status,
    SenderId,
    ChatRoomId,
    ReplyToId,
    Metadata,
    Priority,
    IsPinned,
    ExpiresAt,
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

/// 聊天室表的列定义枚举（用于外键引用）
#[derive(DeriveIden)]
enum ChatRooms {
    Table,
    Id,
}
