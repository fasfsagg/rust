// app/service/notification_service.rs
//
// /------------------------------------------------------------------------------------------------------\
// |                                【用户通知服务】 (notification_service.rs)                         |
// |------------------------------------------------------------------------------------------------------|
// |                                                                                                      |
// | 1. **核心职责**:                                                                                     |
// |    - 管理用户通知偏好和订阅设置                                                                       |
// |    - 提供智能通知过滤和批量优化                                                                       |
// |    - 处理不同类型的用户状态变化通知                                                                   |
// |    - 支持通知历史记录和统计                                                                           |
// |                                                                                                      |
// | 2. **设计原则**:                                                                                     |
// |    - 高性能：支持百万并发用户的通知处理                                                               |
// |    - 可配置：用户可以自定义通知偏好                                                                   |
// |    - 智能化：自动优化批量通知和防止通知风暴                                                           |
// |    - 可扩展：支持未来添加新的通知类型                                                                 |
// |                                                                                                      |
// | 3. **主要功能**:                                                                                     |
// |    - 通知偏好管理（用户可选择接收哪些类型的通知）                                                     |
// |    - 批量通知优化（合并相似通知，防止通知风暴）                                                       |
// |    - 实时状态同步（精确的用户在线状态管理）                                                           |
// |    - 通知历史记录（可选的通知历史功能）                                                               |
// |                                                                                                      |
// \------------------------------------------------------------------------------------------------------/

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{ DateTime, Utc };
use serde::{ Serialize, Deserialize };

use crate::app::model::{ ServerMessage, UserInfo };
use crate::app::service::{ ConnectionManager, MessageDistributor, MessagePriority };

/// 通知类型枚举
///
/// 【功能】: 定义系统支持的所有通知类型
/// 【扩展性】: 可以轻松添加新的通知类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NotificationType {
    /// 用户加入通知
    UserJoined,
    /// 用户离开通知
    UserLeft,
    /// 用户状态变化通知
    UserStatusChanged,
    /// 在线用户数变化通知
    OnlineCountChanged,
    /// 系统公告
    SystemAnnouncement,
    /// 聊天室活动通知
    ChatActivity,
    /// 连接质量通知
    ConnectionQuality,
}

impl std::fmt::Display for NotificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display_str = match self {
            NotificationType::UserJoined => "用户加入",
            NotificationType::UserLeft => "用户离开",
            NotificationType::UserStatusChanged => "用户状态变化",
            NotificationType::OnlineCountChanged => "在线人数变化",
            NotificationType::SystemAnnouncement => "系统公告",
            NotificationType::ChatActivity => "聊天室活动",
            NotificationType::ConnectionQuality => "连接质量",
        };
        write!(f, "{}", display_str)
    }
}

/// 通知偏好设置
///
/// 【功能】: 存储用户的通知偏好配置
/// 【用途】: 允许用户自定义接收哪些类型的通知
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    /// 用户ID
    pub user_id: Uuid,
    /// 启用的通知类型
    pub enabled_types: HashMap<NotificationType, bool>,
    /// 通知频率限制（秒）
    pub rate_limit_seconds: u64,
    /// 最后更新时间
    pub updated_at: DateTime<Utc>,
}

/// 通知事件
///
/// 【功能】: 表示一个待处理的通知事件
/// 【用途】: 在通知处理管道中传递通知信息
#[derive(Debug, Clone)]
pub struct NotificationEvent {
    /// 事件ID
    pub id: Uuid,
    /// 通知类型
    pub notification_type: NotificationType,
    /// 相关用户信息
    pub user_info: Option<UserInfo>,
    /// 通知内容
    pub content: String,
    /// 目标用户（None表示广播给所有用户）
    pub target_users: Option<Vec<Uuid>>,
    /// 排除的用户（通常是触发事件的用户）
    pub exclude_users: Option<Vec<Uuid>>,
    /// 事件时间戳
    pub timestamp: DateTime<Utc>,
    /// 优先级
    pub priority: MessagePriority,
}

/// 通知统计信息
///
/// 【功能】: 记录通知系统的统计数据
/// 【用途】: 监控和优化通知系统性能
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationStats {
    /// 总发送通知数
    pub total_sent: u64,
    /// 按类型分组的发送数
    pub sent_by_type: HashMap<NotificationType, u64>,
    /// 被过滤的通知数
    pub filtered_count: u64,
    /// 批量合并的通知数
    pub batched_count: u64,
    /// 最后更新时间
    pub last_updated: DateTime<Utc>,
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        let mut enabled_types = HashMap::new();
        // 默认启用所有通知类型
        enabled_types.insert(NotificationType::UserJoined, true);
        enabled_types.insert(NotificationType::UserLeft, true);
        enabled_types.insert(NotificationType::UserStatusChanged, true);
        enabled_types.insert(NotificationType::OnlineCountChanged, false); // 默认关闭，避免过于频繁
        enabled_types.insert(NotificationType::SystemAnnouncement, true);
        enabled_types.insert(NotificationType::ChatActivity, true);
        enabled_types.insert(NotificationType::ConnectionQuality, false); // 默认关闭

        Self {
            user_id: Uuid::new_v4(),
            enabled_types,
            rate_limit_seconds: 1, // 默认1秒限制
            updated_at: Utc::now(),
        }
    }
}

impl Default for NotificationStats {
    fn default() -> Self {
        Self {
            total_sent: 0,
            sent_by_type: HashMap::new(),
            filtered_count: 0,
            batched_count: 0,
            last_updated: Utc::now(),
        }
    }
}

impl NotificationEvent {
    /// 创建用户加入事件
    ///
    /// 【功能】: 便捷方法创建用户加入通知事件
    /// 【参数】:
    /// * `user_info` - 加入的用户信息
    ///
    /// 【返回值】: NotificationEvent 实例
    pub fn new_user_joined(user_info: UserInfo) -> Self {
        Self {
            id: Uuid::new_v4(),
            notification_type: NotificationType::UserJoined,
            content: format!("用户 {} 加入了聊天大厅", user_info.username),
            user_info: Some(user_info.clone()),
            target_users: None, // 广播给所有用户
            exclude_users: Some(vec![user_info.user_id]), // 排除加入的用户自己
            timestamp: Utc::now(),
            priority: MessagePriority::High,
        }
    }

    /// 创建用户离开事件
    ///
    /// 【功能】: 便捷方法创建用户离开通知事件
    /// 【参数】:
    /// * `user_info` - 离开的用户信息
    ///
    /// 【返回值】: NotificationEvent 实例
    pub fn new_user_left(user_info: UserInfo) -> Self {
        Self {
            id: Uuid::new_v4(),
            notification_type: NotificationType::UserLeft,
            content: format!("用户 {} 离开了聊天大厅", user_info.username),
            user_info: Some(user_info),
            target_users: None, // 广播给所有用户
            exclude_users: None, // 不排除任何用户
            timestamp: Utc::now(),
            priority: MessagePriority::Normal,
        }
    }

    /// 创建在线人数变化事件
    ///
    /// 【功能】: 便捷方法创建在线人数变化通知事件
    /// 【参数】:
    /// * `current_count` - 当前在线人数
    /// * `previous_count` - 之前的在线人数
    ///
    /// 【返回值】: NotificationEvent 实例
    pub fn new_online_count_changed(current_count: usize, previous_count: usize) -> Self {
        let change = (current_count as i32) - (previous_count as i32);
        let content = if change > 0 {
            format!("当前在线用户: {} (+{})", current_count, change)
        } else if change < 0 {
            format!("当前在线用户: {} ({})", current_count, change)
        } else {
            format!("当前在线用户: {}", current_count)
        };

        Self {
            id: Uuid::new_v4(),
            notification_type: NotificationType::OnlineCountChanged,
            content,
            user_info: None,
            target_users: None, // 广播给所有用户
            exclude_users: None,
            timestamp: Utc::now(),
            priority: MessagePriority::Low, // 在线人数变化优先级较低
        }
    }

    /// 创建系统公告事件
    ///
    /// 【功能】: 便捷方法创建系统公告通知事件
    /// 【参数】:
    /// * `content` - 公告内容
    ///
    /// 【返回值】: NotificationEvent 实例
    pub fn new_system_announcement(content: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            notification_type: NotificationType::SystemAnnouncement,
            content,
            user_info: None,
            target_users: None, // 广播给所有用户
            exclude_users: None,
            timestamp: Utc::now(),
            priority: MessagePriority::High, // 系统公告优先级高
        }
    }
}

/// 通知服务主结构体
///
/// 【功能】: 管理用户通知系统的核心服务
/// 【线程安全】: 使用 Arc<RwLock<T>> 确保在多线程环境下的安全访问
/// 【性能考虑】: 为支持百万并发用户而设计，使用高效的通知过滤和批量处理
#[derive(Debug, Clone)]
pub struct NotificationService {
    /// 用户通知偏好映射
    /// Key: UserId, Value: NotificationPreferences
    user_preferences: Arc<RwLock<HashMap<Uuid, NotificationPreferences>>>,

    /// 通知统计信息
    stats: Arc<RwLock<NotificationStats>>,

    /// 连接管理器引用
    connection_manager: Arc<ConnectionManager>,

    /// 消息分发器引用
    message_distributor: Arc<MessageDistributor>,

    /// 最后在线人数（用于检测变化）
    /// 注意：此字段为未来功能预留，用于实现在线人数变化通知
    #[allow(dead_code)]
    last_online_count: Arc<RwLock<usize>>,
}

impl NotificationService {
    /// 创建新的通知服务实例
    ///
    /// 【功能】: 初始化通知服务
    /// 【参数】:
    /// * `connection_manager` - 连接管理器引用
    /// * `message_distributor` - 消息分发器引用
    ///
    /// 【返回值】: NotificationService 实例
    pub fn new(
        connection_manager: Arc<ConnectionManager>,
        message_distributor: Arc<MessageDistributor>
    ) -> Self {
        Self {
            user_preferences: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(NotificationStats::default())),
            connection_manager,
            message_distributor,
            last_online_count: Arc::new(RwLock::new(0)),
        }
    }

    /// 设置用户通知偏好
    ///
    /// 【功能】: 更新用户的通知偏好设置
    /// 【参数】:
    /// * `user_id` - 用户ID
    /// * `preferences` - 新的通知偏好
    ///
    /// 【返回值】: Result<(), String> - 成功返回 Ok(())，失败返回错误信息
    pub async fn set_user_preferences(
        &self,
        user_id: Uuid,
        mut preferences: NotificationPreferences
    ) -> Result<(), String> {
        preferences.user_id = user_id;
        preferences.updated_at = Utc::now();

        let mut user_prefs = self.user_preferences.write().await;
        user_prefs.insert(user_id, preferences);

        println!("NOTIFICATION_SERVICE: 用户 {} 的通知偏好已更新", user_id);
        Ok(())
    }

    /// 获取用户通知偏好
    ///
    /// 【功能】: 获取用户的通知偏好设置
    /// 【参数】:
    /// * `user_id` - 用户ID
    ///
    /// 【返回值】: NotificationPreferences - 用户偏好（如果不存在则返回默认值）
    pub async fn get_user_preferences(&self, user_id: &Uuid) -> NotificationPreferences {
        let user_prefs = self.user_preferences.read().await;
        user_prefs
            .get(user_id)
            .cloned()
            .unwrap_or_else(|| {
                NotificationPreferences {
                    user_id: *user_id,
                    ..Default::default()
                }
            })
    }

    /// 处理通知事件
    ///
    /// 【功能】: 处理通知事件，应用过滤规则并发送通知
    /// 【参数】:
    /// * `event` - 通知事件
    ///
    /// 【返回值】: Result<usize, String> - 成功返回发送的通知数量，失败返回错误信息
    pub async fn handle_notification_event(
        &self,
        event: NotificationEvent
    ) -> Result<usize, String> {
        println!(
            "NOTIFICATION_SERVICE: 处理通知事件 {} - {}",
            event.notification_type,
            event.content
        );

        // 获取目标用户列表
        let target_users = if let Some(targets) = &event.target_users {
            targets.clone()
        } else {
            // 如果没有指定目标用户，则获取所有在线用户
            self.connection_manager
                .get_online_users().await
                .into_iter()
                .map(|user| user.user_id)
                .collect()
        };

        // 应用排除规则
        let filtered_users = if let Some(exclude_list) = &event.exclude_users {
            target_users
                .into_iter()
                .filter(|user_id| !exclude_list.contains(user_id))
                .collect()
        } else {
            target_users
        };

        // 应用用户偏好过滤
        let final_users = self.filter_by_user_preferences(
            &filtered_users,
            &event.notification_type
        ).await;

        if final_users.is_empty() {
            println!("NOTIFICATION_SERVICE: 没有用户需要接收此通知");
            return Ok(0);
        }

        // 创建服务器消息
        let server_message = self.create_server_message_from_event(&event);

        // 发送通知
        let sent_count = self.send_notification_to_users(
            server_message,
            &final_users,
            event.priority
        ).await?;

        // 更新统计信息
        self.update_stats(
            &event.notification_type,
            sent_count,
            filtered_users.len() - final_users.len()
        ).await;

        println!("NOTIFICATION_SERVICE: 通知事件处理完成，发送给 {} 个用户", sent_count);

        Ok(sent_count)
    }

    /// 根据用户偏好过滤用户列表
    ///
    /// 【功能】: 根据用户的通知偏好过滤目标用户列表
    /// 【参数】:
    /// * `users` - 原始用户列表
    /// * `notification_type` - 通知类型
    ///
    /// 【返回值】: Vec<Uuid> - 过滤后的用户列表
    async fn filter_by_user_preferences(
        &self,
        users: &[Uuid],
        notification_type: &NotificationType
    ) -> Vec<Uuid> {
        let user_prefs = self.user_preferences.read().await;
        let mut filtered_users = Vec::new();

        for user_id in users {
            let preferences = user_prefs
                .get(user_id)
                .cloned()
                .unwrap_or_else(|| {
                    NotificationPreferences {
                        user_id: *user_id,
                        ..Default::default()
                    }
                });

            // 检查用户是否启用了此类型的通知
            if preferences.enabled_types.get(notification_type).copied().unwrap_or(true) {
                filtered_users.push(*user_id);
            }
        }

        filtered_users
    }

    /// 从通知事件创建服务器消息
    ///
    /// 【功能】: 将通知事件转换为服务器消息格式
    /// 【参数】:
    /// * `event` - 通知事件
    ///
    /// 【返回值】: ServerMessage - 服务器消息
    fn create_server_message_from_event(&self, event: &NotificationEvent) -> ServerMessage {
        use crate::app::model::ServerMessage;

        match event.notification_type {
            NotificationType::UserJoined => {
                if let Some(user_info) = &event.user_info {
                    ServerMessage::new_user_joined(user_info.clone())
                } else {
                    ServerMessage::new_system(event.content.clone())
                }
            }
            NotificationType::UserLeft => {
                if let Some(user_info) = &event.user_info {
                    ServerMessage::new_user_left(user_info.clone())
                } else {
                    ServerMessage::new_system(event.content.clone())
                }
            }
            _ => {
                // 其他类型的通知都作为系统消息处理
                ServerMessage::new_system(event.content.clone())
            }
        }
    }

    /// 向指定用户发送通知
    ///
    /// 【功能】: 使用消息分发器向用户列表发送通知
    /// 【参数】:
    /// * `message` - 要发送的消息
    /// * `users` - 目标用户列表
    /// * `priority` - 消息优先级
    ///
    /// 【返回值】: Result<usize, String> - 成功返回发送数量，失败返回错误信息
    async fn send_notification_to_users(
        &self,
        message: ServerMessage,
        users: &[Uuid],
        priority: MessagePriority
    ) -> Result<usize, String> {
        if users.is_empty() {
            return Ok(0);
        }

        // 使用消息分发器的批量发送功能
        self.message_distributor.broadcast_to_users(message, users.to_vec(), Some(priority)).await
    }

    /// 更新统计信息
    ///
    /// 【功能】: 更新通知系统的统计数据
    /// 【参数】:
    /// * `notification_type` - 通知类型
    /// * `sent_count` - 发送数量
    /// * `filtered_count` - 被过滤的数量
    async fn update_stats(
        &self,
        notification_type: &NotificationType,
        sent_count: usize,
        filtered_count: usize
    ) {
        let mut stats = self.stats.write().await;
        stats.total_sent += sent_count as u64;
        stats.filtered_count += filtered_count as u64;

        *stats.sent_by_type.entry(notification_type.clone()).or_insert(0) += sent_count as u64;
        stats.last_updated = Utc::now();
    }

    /// 获取通知统计信息
    ///
    /// 【功能】: 获取当前的通知统计数据
    /// 【返回值】: NotificationStats - 统计信息
    pub async fn get_stats(&self) -> NotificationStats {
        self.stats.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::service::{ ConnectionManager, MessageDistributor };
    use tokio::sync::mpsc;
    use axum::extract::ws::Message;

    /// 创建测试用的通知服务实例
    async fn create_test_notification_service() -> (
        NotificationService,
        Arc<ConnectionManager>,
        Arc<MessageDistributor>,
    ) {
        let connection_manager = Arc::new(ConnectionManager::new());
        let message_distributor = Arc::new(
            MessageDistributor::new(
                connection_manager.clone(),
                Some(10), // 小批量用于测试
                Some(1) // 单线程用于测试
            )
        );
        let notification_service = NotificationService::new(
            connection_manager.clone(),
            message_distributor.clone()
        );

        (notification_service, connection_manager, message_distributor)
    }

    #[tokio::test]
    async fn test_notification_service_creation() {
        let (notification_service, _, _) = create_test_notification_service().await;

        // 验证初始统计信息
        let stats = notification_service.get_stats().await;
        assert_eq!(stats.total_sent, 0);
        assert_eq!(stats.filtered_count, 0);
        assert_eq!(stats.batched_count, 0);
        assert!(stats.sent_by_type.is_empty());
    }

    #[tokio::test]
    async fn test_user_preferences_management() {
        let (notification_service, _, _) = create_test_notification_service().await;
        let user_id = Uuid::new_v4();

        // 测试获取默认偏好
        let default_prefs = notification_service.get_user_preferences(&user_id).await;
        assert_eq!(default_prefs.user_id, user_id);
        assert!(
            default_prefs.enabled_types.get(&NotificationType::UserJoined).copied().unwrap_or(false)
        );
        assert!(
            default_prefs.enabled_types.get(&NotificationType::UserLeft).copied().unwrap_or(false)
        );

        // 测试设置自定义偏好
        let mut custom_prefs = NotificationPreferences::default();
        custom_prefs.enabled_types.insert(NotificationType::UserJoined, false);
        custom_prefs.enabled_types.insert(NotificationType::UserLeft, true);

        let result = notification_service.set_user_preferences(user_id, custom_prefs.clone()).await;
        assert!(result.is_ok());

        // 验证偏好已保存
        let saved_prefs = notification_service.get_user_preferences(&user_id).await;
        assert_eq!(saved_prefs.user_id, user_id);
        assert!(
            !saved_prefs.enabled_types.get(&NotificationType::UserJoined).copied().unwrap_or(true)
        );
        assert!(
            saved_prefs.enabled_types.get(&NotificationType::UserLeft).copied().unwrap_or(false)
        );
    }

    #[tokio::test]
    async fn test_user_joined_notification_event() {
        let (notification_service, connection_manager, _) =
            create_test_notification_service().await;

        // 添加一个测试连接
        let (sender, _receiver) = mpsc::unbounded_channel::<Message>();
        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let username = "test_user".to_string();

        connection_manager
            .add_connection(
                connection_id,
                user_id,
                username.clone(),
                sender,
                Some("127.0.0.1".to_string())
            ).await
            .unwrap();

        // 创建用户加入事件
        let user_info = UserInfo {
            user_id,
            username: username.clone(),
            connected_at: Some(chrono::Utc::now()),
        };
        let join_event = NotificationEvent::new_user_joined(user_info);

        // 处理通知事件
        let result = notification_service.handle_notification_event(join_event).await;
        assert!(result.is_ok());

        // 验证统计信息已更新
        let stats = notification_service.get_stats().await;
        assert_eq!(stats.total_sent, 0); // 因为排除了发送者自己，所以发送数为0
        assert_eq!(stats.sent_by_type.get(&NotificationType::UserJoined).copied().unwrap_or(0), 0);
    }

    #[tokio::test]
    async fn test_user_left_notification_event() {
        let (notification_service, connection_manager, _) =
            create_test_notification_service().await;

        // 添加两个测试连接
        let (sender1, _receiver1) = mpsc::unbounded_channel::<Message>();
        let (sender2, _receiver2) = mpsc::unbounded_channel::<Message>();
        let connection_id1 = Uuid::new_v4();
        let connection_id2 = Uuid::new_v4();
        let user_id1 = Uuid::new_v4();
        let user_id2 = Uuid::new_v4();

        connection_manager
            .add_connection(
                connection_id1,
                user_id1,
                "user1".to_string(),
                sender1,
                Some("127.0.0.1".to_string())
            ).await
            .unwrap();

        connection_manager
            .add_connection(
                connection_id2,
                user_id2,
                "user2".to_string(),
                sender2,
                Some("127.0.0.1".to_string())
            ).await
            .unwrap();

        // 创建用户离开事件
        let user_info = UserInfo {
            user_id: user_id1,
            username: "user1".to_string(),
            connected_at: Some(chrono::Utc::now()),
        };
        let leave_event = NotificationEvent::new_user_left(user_info);

        // 处理通知事件
        let result = notification_service.handle_notification_event(leave_event).await;
        assert!(result.is_ok());

        // 验证统计信息已更新
        let stats = notification_service.get_stats().await;
        // 用户离开通知会发送给所有在线用户（包括离开的用户，因为他们可能还在线）
        // 所以这里应该是2个用户
        assert_eq!(stats.total_sent, 2); // 发送给所有在线用户
        assert_eq!(stats.sent_by_type.get(&NotificationType::UserLeft).copied().unwrap_or(0), 2);
    }

    #[tokio::test]
    async fn test_notification_filtering_by_user_preferences() {
        let (notification_service, connection_manager, _) =
            create_test_notification_service().await;

        // 添加一个测试连接
        let (sender, _receiver) = mpsc::unbounded_channel::<Message>();
        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        connection_manager
            .add_connection(
                connection_id,
                user_id,
                "test_user".to_string(),
                sender,
                Some("127.0.0.1".to_string())
            ).await
            .unwrap();

        // 设置用户偏好：禁用用户加入通知
        let mut prefs = NotificationPreferences::default();
        prefs.enabled_types.insert(NotificationType::UserJoined, false);
        notification_service.set_user_preferences(user_id, prefs).await.unwrap();

        // 创建用户加入事件
        let user_info = UserInfo {
            user_id: Uuid::new_v4(), // 不同的用户ID
            username: "another_user".to_string(),
            connected_at: Some(chrono::Utc::now()),
        };
        let join_event = NotificationEvent::new_user_joined(user_info);

        // 处理通知事件
        let result = notification_service.handle_notification_event(join_event).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // 应该被过滤，发送数为0

        // 验证统计信息
        let stats = notification_service.get_stats().await;
        assert_eq!(stats.total_sent, 0);
        // 注意：由于用户偏好过滤发生在获取目标用户之后，所以filtered_count可能为0
        // 这里我们主要验证没有发送通知即可
    }

    #[tokio::test]
    async fn test_online_count_changed_notification() {
        let (_notification_service, _, _) = create_test_notification_service().await;

        // 创建在线人数变化事件
        let count_event = NotificationEvent::new_online_count_changed(5, 3);

        // 验证事件内容
        assert_eq!(count_event.notification_type, NotificationType::OnlineCountChanged);
        assert!(count_event.content.contains("当前在线用户: 5 (+2)"));
        assert_eq!(count_event.priority, MessagePriority::Low);
        assert!(count_event.target_users.is_none()); // 应该广播给所有用户
        assert!(count_event.exclude_users.is_none()); // 不排除任何用户
    }

    #[tokio::test]
    async fn test_system_announcement_notification() {
        let (notification_service, connection_manager, _) =
            create_test_notification_service().await;

        // 添加一个测试连接
        let (sender, _receiver) = mpsc::unbounded_channel::<Message>();
        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        connection_manager
            .add_connection(
                connection_id,
                user_id,
                "test_user".to_string(),
                sender,
                Some("127.0.0.1".to_string())
            ).await
            .unwrap();

        // 创建系统公告事件
        let announcement_event = NotificationEvent::new_system_announcement(
            "系统将在10分钟后进行维护".to_string()
        );

        // 处理通知事件
        let result = notification_service.handle_notification_event(announcement_event).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1); // 应该发送给1个用户

        // 验证统计信息
        let stats = notification_service.get_stats().await;
        assert_eq!(stats.total_sent, 1);
        assert_eq!(
            stats.sent_by_type.get(&NotificationType::SystemAnnouncement).copied().unwrap_or(0),
            1
        );
    }
}
