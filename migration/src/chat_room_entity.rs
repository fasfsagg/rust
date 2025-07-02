//! `chat_room_entity.rs`
//!
//! 聊天室实体定义，用于管理不同的聊天频道。
//! 支持公共聊天室和私人聊天室，为企业级聊天应用提供基础架构。

use sea_orm::entity::prelude::*;
use sea_orm::sea_query::StringLen;
use serde::{ Deserialize, Serialize };

/// 聊天室类型枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum ChatRoomType {
    /// 公共聊天室 - 所有用户都可以加入
    #[sea_orm(string_value = "public")]
    Public,
    /// 私人聊天室 - 需要邀请才能加入
    #[sea_orm(string_value = "private")]
    Private,
    /// 群组聊天室 - 多人聊天群组
    #[sea_orm(string_value = "group")]
    Group,
}

/// 聊天室状态枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum ChatRoomStatus {
    /// 活跃状态 - 正常使用
    #[sea_orm(string_value = "active")]
    Active,
    /// 已归档 - 不再活跃但保留历史
    #[sea_orm(string_value = "archived")]
    Archived,
    /// 已禁用 - 管理员禁用
    #[sea_orm(string_value = "disabled")]
    Disabled,
}

/// 聊天室实体模型
///
/// 设计考虑：
/// - 支持百万并发：使用UUID主键，便于分布式扩展
/// - 企业级功能：支持不同类型的聊天室和状态管理
/// - 可扩展性：预留描述和设置字段用于未来功能扩展
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "chat_rooms")]
pub struct Model {
    /// 聊天室唯一标识符
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,

    /// 聊天室名称
    #[sea_orm(column_type = "String(StringLen::N(100))", unique)]
    pub name: String,

    /// 聊天室描述（可选）
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,

    /// 聊天室类型
    pub room_type: ChatRoomType,

    /// 聊天室状态
    pub status: ChatRoomStatus,

    /// 创建者用户ID
    #[sea_orm(column_type = "Uuid")]
    pub created_by: Uuid,

    /// 最大成员数量（0表示无限制）
    #[sea_orm(default_value = 0)]
    pub max_members: i32,

    /// 当前成员数量
    #[sea_orm(default_value = 0)]
    pub current_members: i32,

    /// 聊天室设置（JSON格式存储扩展配置）
    #[sea_orm(column_type = "Text", nullable)]
    pub settings: Option<String>,

    /// 创建时间
    pub created_at: DateTimeUtc,

    /// 更新时间
    pub updated_at: DateTimeUtc,
}

/// 定义聊天室与其他实体的关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 聊天室与创建者（用户）的多对一关系
    #[sea_orm(
        belongs_to = "super::user_entity::Entity",
        from = "Column::CreatedBy",
        to = "super::user_entity::Column::Id"
    )]
    Creator,

    /// 聊天室与消息的一对多关系
    #[sea_orm(has_many = "super::message_entity::Entity")]
    Messages,
}

/// 实现与用户实体的关联关系（创建者）
impl Related<super::user_entity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Creator.def()
    }
}

/// 实现与消息实体的关联关系
impl Related<super::message_entity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Messages.def()
    }
}

/// 活动模型行为实现
/// 将时间戳管理委托给数据库默认值
impl ActiveModelBehavior for ActiveModel {}
