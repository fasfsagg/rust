//! `user_session_entity.rs`
//!
//! 用户会话实体定义，用于管理WebSocket连接状态和用户在线状态。
//! 为企业级聊天应用提供可靠的连接管理和状态跟踪。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 会话状态枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(Some(20))")]
pub enum SessionStatus {
    /// 在线状态
    #[sea_orm(string_value = "online")]
    Online,
    /// 离线状态
    #[sea_orm(string_value = "offline")]
    Offline,
    /// 忙碌状态
    #[sea_orm(string_value = "busy")]
    Busy,
    /// 离开状态
    #[sea_orm(string_value = "away")]
    Away,
    /// 隐身状态
    #[sea_orm(string_value = "invisible")]
    Invisible,
}

/// 设备类型枚举
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(Some(20))")]
pub enum DeviceType {
    /// 桌面端
    #[sea_orm(string_value = "desktop")]
    Desktop,
    /// 移动端
    #[sea_orm(string_value = "mobile")]
    Mobile,
    /// 平板端
    #[sea_orm(string_value = "tablet")]
    Tablet,
    /// Web端
    #[sea_orm(string_value = "web")]
    Web,
}

/// 用户会话实体模型
/// 
/// 设计考虑：
/// - 高并发支持：使用UUID主键，支持分布式环境下的会话管理
/// - 实时性：记录连接时间、心跳时间，支持连接状态监控
/// - 多设备支持：记录设备类型和信息，支持用户多设备同时在线
/// - 安全性：记录IP地址和用户代理，便于安全审计
/// - 扩展性：预留元数据字段用于存储额外的会话信息
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "user_sessions")]
pub struct Model {
    /// 会话唯一标识符
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,

    /// 关联的用户ID
    #[sea_orm(column_type = "Uuid")]
    pub user_id: Uuid,

    /// 会话令牌（用于WebSocket认证）
    #[sea_orm(column_type = "String(Some(255))", unique)]
    pub session_token: String,

    /// 会话状态
    pub status: SessionStatus,

    /// 设备类型
    pub device_type: DeviceType,

    /// 设备信息（如设备名称、操作系统等）
    #[sea_orm(column_type = "String(Some(255))", nullable)]
    pub device_info: Option<String>,

    /// 客户端IP地址
    #[sea_orm(column_type = "String(Some(45))")]
    pub ip_address: String,

    /// 用户代理字符串
    #[sea_orm(column_type = "Text", nullable)]
    pub user_agent: Option<String>,

    /// 当前所在聊天室ID（可选）
    #[sea_orm(column_type = "Uuid", nullable)]
    pub current_chat_room_id: Option<Uuid>,

    /// 最后活跃时间（用于心跳检测）
    pub last_activity_at: DateTimeUtc,

    /// 最后心跳时间
    pub last_heartbeat_at: DateTimeUtc,

    /// 会话元数据（JSON格式，存储额外的会话信息）
    #[sea_orm(column_type = "Text", nullable)]
    pub metadata: Option<String>,

    /// 会话过期时间
    #[sea_orm(nullable)]
    pub expires_at: Option<DateTimeUtc>,

    /// 创建时间（连接建立时间）
    pub created_at: DateTimeUtc,

    /// 更新时间
    pub updated_at: DateTimeUtc,
}

/// 定义用户会话与其他实体的关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 会话与用户的多对一关系（一个用户可以有多个会话）
    #[sea_orm(
        belongs_to = "super::user_entity::Entity",
        from = "Column::UserId",
        to = "super::user_entity::Column::Id"
    )]
    User,

    /// 会话与当前聊天室的多对一关系（可选）
    #[sea_orm(
        belongs_to = "super::chat_room_entity::Entity",
        from = "Column::CurrentChatRoomId",
        to = "super::chat_room_entity::Column::Id"
    )]
    CurrentChatRoom,
}

/// 实现与用户实体的关联关系
impl Related<super::user_entity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

/// 实现与聊天室实体的关联关系
impl Related<super::chat_room_entity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CurrentChatRoom.def()
    }
}

/// 活动模型行为实现
/// 将时间戳管理委托给数据库默认值
impl ActiveModelBehavior for ActiveModel {}
