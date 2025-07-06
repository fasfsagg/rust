use sea_orm_migration::prelude::*;

/// 创建用户会话表的迁移
///
/// 此迁移创建 `user_sessions` 表，用于管理WebSocket连接状态和用户在线状态。
/// 表设计支持多设备同时在线，为企业级聊天应用提供可靠的连接管理。
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 创建用户会话表
        manager.create_table(
            Table::create()
                .table(UserSessions::Table)
                .if_not_exists()
                // 主键：UUID类型的会话ID
                .col(ColumnDef::new(UserSessions::Id).uuid().not_null().primary_key())
                // 关联的用户ID：外键关联到users表
                .col(ColumnDef::new(UserSessions::UserId).uuid().not_null())
                // 会话令牌：用于WebSocket认证，唯一
                .col(
                    ColumnDef::new(UserSessions::SessionToken)
                        .string_len(255)
                        .not_null()
                        .unique_key()
                )
                // 会话状态：枚举值（online, offline, busy, away, invisible）
                .col(
                    ColumnDef::new(UserSessions::Status).string_len(20).not_null().default("online")
                )
                // 设备类型：枚举值（desktop, mobile, tablet, web）
                .col(
                    ColumnDef::new(UserSessions::DeviceType)
                        .string_len(20)
                        .not_null()
                        .default("web")
                )
                // 设备信息：可选，存储设备名称、操作系统等
                .col(ColumnDef::new(UserSessions::DeviceInfo).string_len(255).null())
                // 客户端IP地址：支持IPv4和IPv6
                .col(ColumnDef::new(UserSessions::IpAddress).string_len(45).not_null())
                // 用户代理字符串：可选
                .col(ColumnDef::new(UserSessions::UserAgent).text().null())
                // 当前所在聊天室ID：可选，外键关联到chat_rooms表
                .col(ColumnDef::new(UserSessions::CurrentChatRoomId).uuid().null())
                // 最后活跃时间：用于心跳检测
                .col(
                    ColumnDef::new(UserSessions::LastActivityAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp())
                )
                // 最后心跳时间
                .col(
                    ColumnDef::new(UserSessions::LastHeartbeatAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp())
                )
                // 会话元数据：JSON格式，存储额外的会话信息
                .col(ColumnDef::new(UserSessions::Metadata).text().null())
                // 会话过期时间：可选
                .col(ColumnDef::new(UserSessions::ExpiresAt).timestamp_with_time_zone().null())
                // 创建时间：连接建立时间
                .col(
                    ColumnDef::new(UserSessions::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp())
                )
                // 更新时间：自动设置为当前时间
                .col(
                    ColumnDef::new(UserSessions::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp())
                )
                // 外键约束：用户必须存在
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_user_sessions_user_id")
                        .from(UserSessions::Table, UserSessions::UserId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                // 外键约束：当前聊天室必须存在（如果指定）
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_user_sessions_current_chat_room_id")
                        .from(UserSessions::Table, UserSessions::CurrentChatRoomId)
                        .to(ChatRooms::Table, ChatRooms::Id)
                        .on_delete(ForeignKeyAction::SetNull)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .to_owned()
        ).await?;

        // 创建索引以优化查询性能
        manager.create_index(
            Index::create()
                .name("idx_user_sessions_user_id")
                .table(UserSessions::Table)
                .col(UserSessions::UserId)
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .name("idx_user_sessions_status")
                .table(UserSessions::Table)
                .col(UserSessions::Status)
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .name("idx_user_sessions_last_activity")
                .table(UserSessions::Table)
                .col(UserSessions::LastActivityAt)
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .name("idx_user_sessions_current_chat_room")
                .table(UserSessions::Table)
                .col(UserSessions::CurrentChatRoomId)
                .to_owned()
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 删除索引
        manager.drop_index(
            Index::drop().name("idx_user_sessions_current_chat_room").to_owned()
        ).await?;

        manager.drop_index(Index::drop().name("idx_user_sessions_last_activity").to_owned()).await?;

        manager.drop_index(Index::drop().name("idx_user_sessions_status").to_owned()).await?;

        manager.drop_index(Index::drop().name("idx_user_sessions_user_id").to_owned()).await?;

        // 删除表
        manager.drop_table(Table::drop().table(UserSessions::Table).to_owned()).await
    }
}

/// 用户会话表的列定义枚举
#[derive(DeriveIden)]
enum UserSessions {
    Table,
    Id,
    UserId,
    SessionToken,
    Status,
    DeviceType,
    DeviceInfo,
    IpAddress,
    UserAgent,
    CurrentChatRoomId,
    LastActivityAt,
    LastHeartbeatAt,
    Metadata,
    ExpiresAt,
    CreatedAt,
    UpdatedAt,
}

/// 用户表的列定义枚举（用于外键引用）
#[derive(DeriveIden)]
enum Users {
    #[sea_orm(iden = "users")]
    Table,
    Id,
}

/// 聊天室表的列定义枚举（用于外键引用）
#[derive(DeriveIden)]
enum ChatRooms {
    Table,
    Id,
}
