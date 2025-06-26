//! `status_sync_service.rs`
//!
//! 实时状态同步服务模块，负责管理用户在线状态和消息状态的实时同步。
//! 为企业级聊天应用提供高性能的状态同步功能，支持百万并发用户。
//!
//! ## 设计原则
//! - **实时性**: 状态变更立即同步给相关用户
//! - **高性能**: 使用高效的数据结构和算法，支持大规模并发
//! - **可靠性**: 确保状态同步的准确性和一致性
//! - **扩展性**: 支持多种状态类型的同步
//!
//! ## 主要功能
//! - 用户在线状态同步
//! - 消息已读/未读状态同步
//! - 用户活跃状态更新
//! - 状态变更事件广播

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };

use crate::app::service::{ ConnectionManager, MessageDistributor, MessagePriority };
use crate::app::model::{ ServerMessage, MessageType };

/// 用户在线状态枚举
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserOnlineStatus {
    /// 在线
    Online,
    /// 离线
    Offline,
    /// 忙碌
    Busy,
    /// 离开
    Away,
}

/// 消息状态枚举
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageReadStatus {
    /// 未读
    Unread,
    /// 已读
    Read,
    /// 已送达
    Delivered,
}

/// 用户状态信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStatusInfo {
    /// 用户ID
    pub user_id: Uuid,
    /// 用户名
    pub username: String,
    /// 在线状态
    pub online_status: UserOnlineStatus,
    /// 最后活跃时间
    pub last_activity: DateTime<Utc>,
    /// 连接数量（支持多设备）
    pub connection_count: usize,
}

/// 消息状态信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStatusInfo {
    /// 消息ID
    pub message_id: Uuid,
    /// 用户ID
    pub user_id: Uuid,
    /// 读取状态
    pub read_status: MessageReadStatus,
    /// 状态更新时间
    pub updated_at: DateTime<Utc>,
}

/// 状态同步事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum StatusSyncEvent {
    /// 用户状态变更事件
    UserStatusChanged {
        user_status: UserStatusInfo,
    },
    /// 消息状态变更事件
    MessageStatusChanged {
        message_status: MessageStatusInfo,
    },
    /// 用户上线事件
    UserOnline {
        user_id: Uuid,
        username: String,
        timestamp: DateTime<Utc>,
    },
    /// 用户下线事件
    UserOffline {
        user_id: Uuid,
        username: String,
        timestamp: DateTime<Utc>,
    },
    /// 消息已读事件
    MessageRead {
        message_id: Uuid,
        user_id: Uuid,
        timestamp: DateTime<Utc>,
    },
}

/// 状态同步服务主结构体
///
/// 【功能】: 管理所有用户和消息的状态同步
/// 【线程安全】: 使用 Arc<RwLock<T>> 确保在多线程环境下的安全访问
/// 【性能考虑】: 为支持百万并发而设计，使用高效的数据结构
#[derive(Debug, Clone)]
pub struct StatusSyncService {
    /// 用户状态映射表
    /// Key: UserId, Value: UserStatusInfo
    user_statuses: Arc<RwLock<HashMap<Uuid, UserStatusInfo>>>,

    /// 消息状态映射表
    /// Key: (MessageId, UserId), Value: MessageStatusInfo
    message_statuses: Arc<RwLock<HashMap<(Uuid, Uuid), MessageStatusInfo>>>,

    /// 连接管理器引用
    connection_manager: Arc<ConnectionManager>,

    /// 消息分发器引用
    message_distributor: Arc<MessageDistributor>,
}

impl StatusSyncService {
    /// 创建新的状态同步服务实例
    ///
    /// 【功能】: 初始化状态同步服务
    /// 【参数】:
    /// * `connection_manager` - 连接管理器引用
    /// * `message_distributor` - 消息分发器引用
    ///
    /// 【返回值】: StatusSyncService 实例
    pub fn new(
        connection_manager: Arc<ConnectionManager>,
        message_distributor: Arc<MessageDistributor>
    ) -> Self {
        Self {
            user_statuses: Arc::new(RwLock::new(HashMap::new())),
            message_statuses: Arc::new(RwLock::new(HashMap::new())),
            connection_manager,
            message_distributor,
        }
    }

    /// 更新用户在线状态
    ///
    /// 【功能】: 更新用户的在线状态并同步给其他用户
    /// 【参数】:
    /// * `user_id` - 用户ID
    /// * `username` - 用户名
    /// * `status` - 新的在线状态
    ///
    /// 【返回值】: Result<(), String> - 成功返回 Ok(())，失败返回错误信息
    pub async fn update_user_status(
        &self,
        user_id: Uuid,
        username: String,
        status: UserOnlineStatus
    ) -> Result<(), String> {
        let now = Utc::now();

        // 获取连接数量（通过在线用户列表查找）
        let online_users = self.connection_manager.get_online_users().await;
        let connection_count = online_users
            .iter()
            .find(|u| u.user_id == user_id)
            .map(|u| u.connection_count)
            .unwrap_or(0);

        // 创建用户状态信息
        let user_status = UserStatusInfo {
            user_id,
            username: username.clone(),
            online_status: status.clone(),
            last_activity: now,
            connection_count,
        };

        // 更新内存中的状态
        {
            let mut statuses = self.user_statuses.write().await;
            statuses.insert(user_id, user_status.clone());
        }

        // 创建状态同步事件
        let sync_event = match status {
            UserOnlineStatus::Online =>
                StatusSyncEvent::UserOnline {
                    user_id,
                    username: username.clone(),
                    timestamp: now,
                },
            UserOnlineStatus::Offline =>
                StatusSyncEvent::UserOffline {
                    user_id,
                    username: username.clone(),
                    timestamp: now,
                },
            _ =>
                StatusSyncEvent::UserStatusChanged {
                    user_status: user_status.clone(),
                },
        };

        // 广播状态变更事件
        self.broadcast_status_event(sync_event).await?;

        println!("STATUS_SYNC: 用户 {} 状态更新为 {:?}", username, status);
        Ok(())
    }

    /// 更新消息读取状态
    ///
    /// 【功能】: 更新消息的读取状态并同步给相关用户
    /// 【参数】:
    /// * `message_id` - 消息ID
    /// * `user_id` - 用户ID
    /// * `read_status` - 新的读取状态
    ///
    /// 【返回值】: Result<(), String> - 成功返回 Ok(())，失败返回错误信息
    pub async fn update_message_status(
        &self,
        message_id: Uuid,
        user_id: Uuid,
        read_status: MessageReadStatus
    ) -> Result<(), String> {
        let now = Utc::now();

        // 创建消息状态信息
        let message_status = MessageStatusInfo {
            message_id,
            user_id,
            read_status: read_status.clone(),
            updated_at: now,
        };

        // 更新内存中的状态
        {
            let mut statuses = self.message_statuses.write().await;
            statuses.insert((message_id, user_id), message_status.clone());
        }

        // 创建状态同步事件
        let sync_event = match read_status {
            MessageReadStatus::Read =>
                StatusSyncEvent::MessageRead {
                    message_id,
                    user_id,
                    timestamp: now,
                },
            _ =>
                StatusSyncEvent::MessageStatusChanged {
                    message_status: message_status.clone(),
                },
        };

        // 广播状态变更事件
        self.broadcast_status_event(sync_event).await?;

        println!(
            "STATUS_SYNC: 消息 {} 状态更新为 {:?} (用户: {})",
            message_id,
            read_status,
            user_id
        );
        Ok(())
    }

    /// 获取用户状态
    ///
    /// 【功能】: 获取指定用户的当前状态
    /// 【参数】:
    /// * `user_id` - 用户ID
    ///
    /// 【返回值】: Option<UserStatusInfo> - 用户状态信息
    pub async fn get_user_status(&self, user_id: &Uuid) -> Option<UserStatusInfo> {
        let statuses = self.user_statuses.read().await;
        statuses.get(user_id).cloned()
    }

    /// 获取消息状态
    ///
    /// 【功能】: 获取指定消息和用户的读取状态
    /// 【参数】:
    /// * `message_id` - 消息ID
    /// * `user_id` - 用户ID
    ///
    /// 【返回值】: Option<MessageStatusInfo> - 消息状态信息
    pub async fn get_message_status(
        &self,
        message_id: &Uuid,
        user_id: &Uuid
    ) -> Option<MessageStatusInfo> {
        let statuses = self.message_statuses.read().await;
        statuses.get(&(*message_id, *user_id)).cloned()
    }

    /// 获取所有在线用户状态
    ///
    /// 【功能】: 获取当前所有在线用户的状态列表
    /// 【返回值】: Vec<UserStatusInfo> - 在线用户状态列表
    pub async fn get_online_users(&self) -> Vec<UserStatusInfo> {
        let statuses = self.user_statuses.read().await;
        statuses
            .values()
            .filter(|status| status.online_status == UserOnlineStatus::Online)
            .cloned()
            .collect()
    }

    /// 广播状态事件
    ///
    /// 【功能】: 将状态同步事件广播给所有相关用户
    /// 【参数】:
    /// * `event` - 状态同步事件
    ///
    /// 【返回值】: Result<(), String> - 成功返回 Ok(())，失败返回错误信息
    async fn broadcast_status_event(&self, event: StatusSyncEvent) -> Result<(), String> {
        // 创建状态同步消息
        let message_content = serde_json
            ::to_string(&event)
            .map_err(|e| format!("序列化状态事件失败: {}", e))?;

        let server_message = ServerMessage {
            id: Uuid::new_v4(),
            message_type: MessageType::System,
            content: message_content,
            sender: None,
            timestamp: Utc::now(),
            recipient: None,
        };

        // 广播给所有在线用户
        self.message_distributor.broadcast_to_all(
            server_message,
            false, // 不排除发送者
            None,
            Some(MessagePriority::High) // 状态同步使用高优先级
        ).await?;

        Ok(())
    }

    /// 清理离线用户状态
    ///
    /// 【功能】: 清理已离线用户的状态信息，释放内存
    /// 【参数】:
    /// * `user_id` - 用户ID
    ///
    /// 【返回值】: Result<(), String> - 成功返回 Ok(())，失败返回错误信息
    pub async fn cleanup_user_status(&self, user_id: &Uuid) -> Result<(), String> {
        let mut statuses = self.user_statuses.write().await;
        if let Some(removed_status) = statuses.remove(user_id) {
            println!("STATUS_SYNC: 清理用户 {} 的状态信息", removed_status.username);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::service::{ ConnectionManager, MessageDistributor };
    use std::sync::Arc;
    use uuid::Uuid;

    /// 创建测试用的状态同步服务
    async fn create_test_status_sync_service() -> StatusSyncService {
        let connection_manager = Arc::new(ConnectionManager::new());
        let message_distributor = Arc::new(
            MessageDistributor::new(
                connection_manager.clone(),
                Some(10), // 小批量用于测试
                Some(1) // 单线程用于测试
            )
        );

        StatusSyncService::new(connection_manager, message_distributor)
    }

    #[tokio::test]
    async fn test_update_user_status() {
        let service = create_test_status_sync_service().await;
        let user_id = Uuid::new_v4();
        let username = "test_user".to_string();

        // 测试更新用户在线状态
        let result = service.update_user_status(
            user_id,
            username.clone(),
            UserOnlineStatus::Online
        ).await;

        assert!(result.is_ok());

        // 验证状态已保存
        let status = service.get_user_status(&user_id).await;
        assert!(status.is_some());
        let status = status.unwrap();
        assert_eq!(status.user_id, user_id);
        assert_eq!(status.username, username);
        assert_eq!(status.online_status, UserOnlineStatus::Online);
    }

    #[tokio::test]
    async fn test_update_message_status() {
        let service = create_test_status_sync_service().await;
        let message_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        // 测试更新消息已读状态
        let result = service.update_message_status(
            message_id,
            user_id,
            MessageReadStatus::Read
        ).await;

        assert!(result.is_ok());

        // 验证状态已保存
        let status = service.get_message_status(&message_id, &user_id).await;
        assert!(status.is_some());
        let status = status.unwrap();
        assert_eq!(status.message_id, message_id);
        assert_eq!(status.user_id, user_id);
        assert_eq!(status.read_status, MessageReadStatus::Read);
    }

    #[tokio::test]
    async fn test_get_online_users() {
        let service = create_test_status_sync_service().await;

        // 添加几个在线用户
        let user1_id = Uuid::new_v4();
        let user2_id = Uuid::new_v4();
        let user3_id = Uuid::new_v4();

        service
            .update_user_status(user1_id, "user1".to_string(), UserOnlineStatus::Online).await
            .unwrap();
        service
            .update_user_status(user2_id, "user2".to_string(), UserOnlineStatus::Online).await
            .unwrap();
        service
            .update_user_status(user3_id, "user3".to_string(), UserOnlineStatus::Offline).await
            .unwrap();

        // 获取在线用户列表
        let online_users = service.get_online_users().await;

        // 应该只有2个在线用户
        assert_eq!(online_users.len(), 2);

        let online_user_ids: Vec<Uuid> = online_users
            .iter()
            .map(|u| u.user_id)
            .collect();
        assert!(online_user_ids.contains(&user1_id));
        assert!(online_user_ids.contains(&user2_id));
        assert!(!online_user_ids.contains(&user3_id));
    }

    #[tokio::test]
    async fn test_cleanup_user_status() {
        let service = create_test_status_sync_service().await;
        let user_id = Uuid::new_v4();

        // 添加用户状态
        service
            .update_user_status(user_id, "test_user".to_string(), UserOnlineStatus::Online).await
            .unwrap();

        // 验证状态存在
        assert!(service.get_user_status(&user_id).await.is_some());

        // 清理状态
        let result = service.cleanup_user_status(&user_id).await;
        assert!(result.is_ok());

        // 验证状态已清理
        assert!(service.get_user_status(&user_id).await.is_none());
    }

    #[tokio::test]
    async fn test_multiple_message_statuses() {
        let service = create_test_status_sync_service().await;
        let message_id = Uuid::new_v4();
        let user1_id = Uuid::new_v4();
        let user2_id = Uuid::new_v4();

        // 不同用户对同一消息的不同状态
        service.update_message_status(message_id, user1_id, MessageReadStatus::Read).await.unwrap();
        service
            .update_message_status(message_id, user2_id, MessageReadStatus::Delivered).await
            .unwrap();

        // 验证状态独立存储
        let user1_status = service.get_message_status(&message_id, &user1_id).await.unwrap();
        let user2_status = service.get_message_status(&message_id, &user2_id).await.unwrap();

        assert_eq!(user1_status.read_status, MessageReadStatus::Read);
        assert_eq!(user2_status.read_status, MessageReadStatus::Delivered);
    }
}
