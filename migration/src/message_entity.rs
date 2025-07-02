//! `message_entity.rs`
//!
//! 消息实体定义，用于存储聊天消息。
//! 支持多种消息类型，为企业级聊天应用提供丰富的消息功能。

use sea_orm::entity::prelude::*;
use serde::{ Deserialize, Serialize };

/// 消息类型枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum MessageType {
    /// 普通文本消息
    #[sea_orm(string_value = "text")]
    Text,
    /// 图片消息
    #[sea_orm(string_value = "image")]
    Image,
    /// 文件消息
    #[sea_orm(string_value = "file")]
    File,
    /// 系统消息（如用户加入/离开）
    #[sea_orm(string_value = "system")]
    System,
    /// 语音消息
    #[sea_orm(string_value = "voice")]
    Voice,
    /// 视频消息
    #[sea_orm(string_value = "video")]
    Video,
}

/// 消息状态枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum MessageStatus {
    /// 已发送
    #[sea_orm(string_value = "sent")]
    Sent,
    /// 已送达
    #[sea_orm(string_value = "delivered")]
    Delivered,
    /// 已读
    #[sea_orm(string_value = "read")]
    Read,
    /// 已删除
    #[sea_orm(string_value = "deleted")]
    Deleted,
    /// 已编辑
    #[sea_orm(string_value = "edited")]
    Edited,
}

/// 消息实体模型
///
/// 设计考虑：
/// - 高并发支持：使用UUID主键，支持分布式环境
/// - 消息完整性：记录发送者、接收聊天室、消息类型等完整信息
/// - 扩展性：支持多种消息类型和状态，预留元数据字段
/// - 性能优化：为常用查询字段建立索引（在迁移中实现）
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "messages")]
pub struct Model {
    /// 消息唯一标识符
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,

    /// 消息内容
    #[sea_orm(column_type = "Text")]
    pub content: String,

    /// 消息类型
    pub message_type: MessageType,

    /// 消息状态
    pub status: MessageStatus,

    /// 发送者用户ID
    #[sea_orm(column_type = "Uuid")]
    pub sender_id: Uuid,

    /// 目标聊天室ID
    #[sea_orm(column_type = "Uuid")]
    pub chat_room_id: Uuid,

    /// 回复的消息ID（可选，用于消息回复功能）
    #[sea_orm(column_type = "Uuid", nullable)]
    pub reply_to_id: Option<Uuid>,

    /// 消息元数据（JSON格式，存储文件信息、位置信息等）
    #[sea_orm(column_type = "Text", nullable)]
    pub metadata: Option<String>,

    /// 消息优先级（0-9，9为最高优先级）
    #[sea_orm(default_value = 5)]
    pub priority: i32,

    /// 是否置顶消息
    #[sea_orm(default_value = false)]
    pub is_pinned: bool,

    /// 消息过期时间（可选，用于临时消息）
    #[sea_orm(nullable)]
    pub expires_at: Option<DateTimeUtc>,

    /// 创建时间
    pub created_at: DateTimeUtc,

    /// 更新时间
    pub updated_at: DateTimeUtc,
}

/// 定义消息与其他实体的关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 消息与发送者（用户）的多对一关系
    #[sea_orm(
        belongs_to = "super::user_entity::Entity",
        from = "Column::SenderId",
        to = "super::user_entity::Column::Id"
    )]
    Sender,

    /// 消息与聊天室的多对一关系
    #[sea_orm(
        belongs_to = "super::chat_room_entity::Entity",
        from = "Column::ChatRoomId",
        to = "super::chat_room_entity::Column::Id"
    )]
    ChatRoom,

    /// 消息回复关系（自引用）
    #[sea_orm(belongs_to = "Entity", from = "Column::ReplyToId", to = "Column::Id")]
    ReplyTo,
}

/// 实现与用户实体的关联关系（发送者）
impl Related<super::user_entity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Sender.def()
    }
}

/// 实现与聊天室实体的关联关系
impl Related<super::chat_room_entity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ChatRoom.def()
    }
}

/// 活动模型行为实现
/// 将时间戳管理委托给数据库默认值
impl ActiveModelBehavior for ActiveModel {}
