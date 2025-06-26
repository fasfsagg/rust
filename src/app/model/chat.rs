// app/model/chat.rs
//
// /------------------------------------------------------------------------------------------------------\
// |                                【聊天消息模型定义】 (chat.rs)                                      |
// |------------------------------------------------------------------------------------------------------|
// |                                                                                                      |
// | 1. **核心职责**:                                                                                     |
// |    - 定义聊天系统中的消息结构和类型                                                                   |
// |    - 提供客户端和服务器之间的消息协议                                                                 |
// |    - 支持不同类型的聊天消息（文本、系统通知、用户状态等）                                             |
// |                                                                                                      |
// | 2. **设计原则**:                                                                                     |
// |    - 类型安全：使用强类型确保消息格式正确                                                             |
// |    - 可扩展性：支持未来添加新的消息类型                                                               |
// |    - 序列化友好：所有类型都支持 JSON 序列化/反序列化                                                  |
// |    - 向后兼容：保持 API 的稳定性                                                                     |
// |                                                                                                      |
// | 3. **消息类型**:                                                                                     |
// |    - `ChatMessage`: 客户端发送的消息                                                                |
// |    - `ServerMessage`: 服务器发送的消息                                                              |
// |    - `MessageType`: 消息类型枚举                                                                    |
// |    - `UserInfo`: 用户信息结构                                                                       |
// |                                                                                                      |
// \------------------------------------------------------------------------------------------------------/

use serde::{ Deserialize, Serialize };
use uuid::Uuid;
use chrono::{ DateTime, Utc };

/// 客户端发送的聊天消息
///
/// 【功能】: 定义客户端向服务器发送的消息格式
/// 【用途】: WebSocket 消息的反序列化目标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// 消息类型
    pub message_type: MessageType,
    /// 消息内容
    pub content: String,
    /// 客户端时间戳（可选）
    pub timestamp: Option<DateTime<Utc>>,
}

/// 服务器发送的消息
///
/// 【功能】: 定义服务器向客户端发送的消息格式
/// 【用途】: 包含更多元数据的完整消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMessage {
    /// 消息ID
    pub id: Uuid,
    /// 消息类型
    pub message_type: MessageType,
    /// 消息内容
    pub content: String,
    /// 发送者信息（系统消息时为 None）
    pub sender: Option<UserInfo>,
    /// 服务器时间戳
    pub timestamp: DateTime<Utc>,
    /// 接收者（私聊时使用，群聊时为 None）
    pub recipient: Option<UserInfo>,
}

/// 消息类型枚举
///
/// 【功能】: 定义支持的消息类型
/// 【扩展性】: 可以轻松添加新的消息类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum MessageType {
    /// 普通文本消息
    Text,
    /// 系统通知消息
    System,
    /// 用户加入通知
    UserJoined,
    /// 用户离开通知
    UserLeft,
    /// 在线用户列表请求
    GetOnlineUsers,
    /// 在线用户列表响应
    OnlineUsersList,
    /// 心跳消息
    Ping,
    /// 心跳响应
    Pong,
    /// 错误消息
    Error,
    /// 消息已读通知
    MessageRead,
    /// 用户状态变更通知
    UserStatusChanged,
}

/// 用户信息结构
///
/// 【功能】: 在消息中携带用户基本信息
/// 【用途】: 显示发送者信息，支持用户识别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// 用户ID
    pub user_id: Uuid,
    /// 用户名
    pub username: String,
    /// 连接时间（可选）
    pub connected_at: Option<DateTime<Utc>>,
}

/// 在线用户列表响应
///
/// 【功能】: 包含当前在线用户列表的响应消息
/// 【用途】: 响应客户端的在线用户查询请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnlineUsersResponse {
    /// 在线用户列表
    pub users: Vec<UserInfo>,
    /// 总用户数
    pub total_count: usize,
    /// 响应时间戳
    pub timestamp: DateTime<Utc>,
}

/// 错误响应消息
///
/// 【功能】: 定义错误消息的结构
/// 【用途】: 向客户端报告错误信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// 错误代码
    pub error_code: String,
    /// 错误消息
    pub error_message: String,
    /// 错误详情（可选）
    pub details: Option<String>,
    /// 错误时间戳
    pub timestamp: DateTime<Utc>,
}

impl ChatMessage {
    /// 创建新的文本消息
    ///
    /// 【功能】: 便捷方法创建文本消息
    /// 【参数】:
    /// * `content` - 消息内容
    ///
    /// 【返回值】: ChatMessage 实例
    pub fn new_text(content: String) -> Self {
        Self {
            message_type: MessageType::Text,
            content,
            timestamp: Some(Utc::now()),
        }
    }

    /// 创建获取在线用户请求
    ///
    /// 【功能】: 便捷方法创建在线用户查询请求
    /// 【返回值】: ChatMessage 实例
    pub fn new_get_online_users() -> Self {
        Self {
            message_type: MessageType::GetOnlineUsers,
            content: String::new(),
            timestamp: Some(Utc::now()),
        }
    }

    /// 创建心跳消息
    ///
    /// 【功能】: 便捷方法创建心跳消息
    /// 【返回值】: ChatMessage 实例
    pub fn new_ping() -> Self {
        Self {
            message_type: MessageType::Ping,
            content: "ping".to_string(),
            timestamp: Some(Utc::now()),
        }
    }

    /// 创建消息已读通知
    ///
    /// 【功能】: 便捷方法创建消息已读通知
    /// 【参数】:
    /// * `message_id` - 已读的消息ID
    ///
    /// 【返回值】: ChatMessage 实例
    pub fn new_message_read(message_id: Uuid) -> Self {
        Self {
            message_type: MessageType::MessageRead,
            content: message_id.to_string(),
            timestamp: Some(Utc::now()),
        }
    }
}

impl ServerMessage {
    /// 创建新的文本消息
    ///
    /// 【功能】: 便捷方法创建服务器文本消息
    /// 【参数】:
    /// * `content` - 消息内容
    /// * `sender` - 发送者信息
    ///
    /// 【返回值】: ServerMessage 实例
    pub fn new_text(content: String, sender: UserInfo) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_type: MessageType::Text,
            content,
            sender: Some(sender),
            timestamp: Utc::now(),
            recipient: None,
        }
    }

    /// 创建系统消息
    ///
    /// 【功能】: 便捷方法创建系统通知消息
    /// 【参数】:
    /// * `content` - 消息内容
    ///
    /// 【返回值】: ServerMessage 实例
    pub fn new_system(content: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_type: MessageType::System,
            content,
            sender: None,
            timestamp: Utc::now(),
            recipient: None,
        }
    }

    /// 创建用户加入通知
    ///
    /// 【功能】: 便捷方法创建用户加入通知
    /// 【参数】:
    /// * `user` - 加入的用户信息
    ///
    /// 【返回值】: ServerMessage 实例
    pub fn new_user_joined(user: UserInfo) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_type: MessageType::UserJoined,
            content: format!("用户 {} 加入了聊天大厅", user.username),
            sender: None,
            timestamp: Utc::now(),
            recipient: None,
        }
    }

    /// 创建用户离开通知
    ///
    /// 【功能】: 便捷方法创建用户离开通知
    /// 【参数】:
    /// * `user` - 离开的用户信息
    ///
    /// 【返回值】: ServerMessage 实例
    pub fn new_user_left(user: UserInfo) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_type: MessageType::UserLeft,
            content: format!("用户 {} 离开了聊天大厅", user.username),
            sender: None,
            timestamp: Utc::now(),
            recipient: None,
        }
    }

    /// 创建在线用户列表响应
    ///
    /// 【功能】: 便捷方法创建在线用户列表响应
    /// 【参数】:
    /// * `users_response` - 在线用户响应数据
    ///
    /// 【返回值】: ServerMessage 实例
    pub fn new_online_users_list(users_response: OnlineUsersResponse) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_type: MessageType::OnlineUsersList,
            content: serde_json::to_string(&users_response).unwrap_or_default(),
            sender: None,
            timestamp: Utc::now(),
            recipient: None,
        }
    }

    /// 创建错误消息
    ///
    /// 【功能】: 便捷方法创建错误响应消息
    /// 【参数】:
    /// * `error_response` - 错误响应数据
    ///
    /// 【返回值】: ServerMessage 实例
    pub fn new_error(error_response: ErrorResponse) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_type: MessageType::Error,
            content: serde_json::to_string(&error_response).unwrap_or_default(),
            sender: None,
            timestamp: Utc::now(),
            recipient: None,
        }
    }

    /// 创建心跳响应
    ///
    /// 【功能】: 便捷方法创建心跳响应消息
    /// 【返回值】: ServerMessage 实例
    pub fn new_pong() -> Self {
        Self {
            id: Uuid::new_v4(),
            message_type: MessageType::Pong,
            content: "pong".to_string(),
            sender: None,
            timestamp: Utc::now(),
            recipient: None,
        }
    }
}

impl From<UserInfo> for crate::app::service::OnlineUser {
    fn from(user_info: UserInfo) -> Self {
        Self {
            user_id: user_info.user_id,
            username: user_info.username,
            connected_at: user_info.connected_at.unwrap_or_else(Utc::now),
            connection_count: 1, // 默认值，实际值由连接管理器提供
        }
    }
}

impl From<crate::app::service::OnlineUser> for UserInfo {
    fn from(online_user: crate::app::service::OnlineUser) -> Self {
        Self {
            user_id: online_user.user_id,
            username: online_user.username,
            connected_at: Some(online_user.connected_at),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use chrono::Utc;

    #[test]
    fn test_chat_message_creation() {
        // 测试创建文本消息
        let text_msg = ChatMessage::new_text("Hello, World!".to_string());
        assert_eq!(text_msg.message_type, MessageType::Text);
        assert_eq!(text_msg.content, "Hello, World!");
        assert!(text_msg.timestamp.is_some());

        // 测试创建获取在线用户请求
        let online_users_msg = ChatMessage::new_get_online_users();
        assert_eq!(online_users_msg.message_type, MessageType::GetOnlineUsers);
        assert!(online_users_msg.timestamp.is_some());

        // 测试创建心跳消息
        let ping_msg = ChatMessage::new_ping();
        assert_eq!(ping_msg.message_type, MessageType::Ping);
        assert_eq!(ping_msg.content, "ping");
        assert!(ping_msg.timestamp.is_some());
    }

    #[test]
    fn test_server_message_creation() {
        let user_info = UserInfo {
            user_id: Uuid::new_v4(),
            username: "test_user".to_string(),
            connected_at: Some(Utc::now()),
        };

        // 测试创建文本消息
        let text_msg = ServerMessage::new_text("Hello".to_string(), user_info.clone());
        assert_eq!(text_msg.message_type, MessageType::Text);
        assert_eq!(text_msg.content, "Hello");
        assert!(text_msg.sender.is_some());
        assert_eq!(text_msg.sender.as_ref().unwrap().username, "test_user");

        // 测试创建系统消息
        let system_msg = ServerMessage::new_system("System notification".to_string());
        assert_eq!(system_msg.message_type, MessageType::System);
        assert_eq!(system_msg.content, "System notification");
        assert!(system_msg.sender.is_none());

        // 测试创建用户加入通知
        let join_msg = ServerMessage::new_user_joined(user_info.clone());
        assert_eq!(join_msg.message_type, MessageType::UserJoined);
        assert!(join_msg.content.contains("test_user"));
        assert!(join_msg.content.contains("加入了聊天大厅"));

        // 测试创建用户离开通知
        let leave_msg = ServerMessage::new_user_left(user_info.clone());
        assert_eq!(leave_msg.message_type, MessageType::UserLeft);
        assert!(leave_msg.content.contains("test_user"));
        assert!(leave_msg.content.contains("离开了聊天大厅"));

        // 测试创建心跳响应
        let pong_msg = ServerMessage::new_pong();
        assert_eq!(pong_msg.message_type, MessageType::Pong);
        assert_eq!(pong_msg.content, "pong");
        assert!(pong_msg.sender.is_none());
    }

    #[test]
    fn test_message_serialization() {
        // 测试ChatMessage序列化
        let chat_msg = ChatMessage::new_text("Test message".to_string());
        let serialized = serde_json::to_string(&chat_msg).unwrap();
        let deserialized: ChatMessage = serde_json::from_str(&serialized).unwrap();

        assert_eq!(chat_msg.message_type, deserialized.message_type);
        assert_eq!(chat_msg.content, deserialized.content);

        // 测试ServerMessage序列化
        let user_info = UserInfo {
            user_id: Uuid::new_v4(),
            username: "test_user".to_string(),
            connected_at: Some(Utc::now()),
        };
        let server_msg = ServerMessage::new_text("Test".to_string(), user_info);
        let serialized = serde_json::to_string(&server_msg).unwrap();
        let deserialized: ServerMessage = serde_json::from_str(&serialized).unwrap();

        assert_eq!(server_msg.message_type, deserialized.message_type);
        assert_eq!(server_msg.content, deserialized.content);
        assert_eq!(
            server_msg.sender.as_ref().unwrap().username,
            deserialized.sender.as_ref().unwrap().username
        );
    }

    #[test]
    fn test_online_users_response() {
        let users = vec![
            UserInfo {
                user_id: Uuid::new_v4(),
                username: "user1".to_string(),
                connected_at: Some(Utc::now()),
            },
            UserInfo {
                user_id: Uuid::new_v4(),
                username: "user2".to_string(),
                connected_at: Some(Utc::now()),
            }
        ];

        let response = OnlineUsersResponse {
            users: users.clone(),
            total_count: 2,
            timestamp: Utc::now(),
        };

        let server_msg = ServerMessage::new_online_users_list(response);
        assert_eq!(server_msg.message_type, MessageType::OnlineUsersList);

        // 验证内容可以反序列化
        let parsed_response: OnlineUsersResponse = serde_json
            ::from_str(&server_msg.content)
            .unwrap();
        assert_eq!(parsed_response.total_count, 2);
        assert_eq!(parsed_response.users.len(), 2);
        assert_eq!(parsed_response.users[0].username, "user1");
        assert_eq!(parsed_response.users[1].username, "user2");
    }

    #[test]
    fn test_error_response() {
        let error_response = ErrorResponse {
            error_code: "TEST_ERROR".to_string(),
            error_message: "Test error message".to_string(),
            details: Some("Additional details".to_string()),
            timestamp: Utc::now(),
        };

        let server_msg = ServerMessage::new_error(error_response);
        assert_eq!(server_msg.message_type, MessageType::Error);

        // 验证内容可以反序列化
        let parsed_error: ErrorResponse = serde_json::from_str(&server_msg.content).unwrap();
        assert_eq!(parsed_error.error_code, "TEST_ERROR");
        assert_eq!(parsed_error.error_message, "Test error message");
        assert_eq!(parsed_error.details, Some("Additional details".to_string()));
    }

    #[test]
    fn test_user_info_conversion() {
        use crate::app::service::OnlineUser;

        let online_user = OnlineUser {
            user_id: Uuid::new_v4(),
            username: "test_user".to_string(),
            connected_at: Utc::now(),
            connection_count: 2,
        };

        // 测试从OnlineUser转换为UserInfo
        let user_info: UserInfo = online_user.clone().into();
        assert_eq!(user_info.user_id, online_user.user_id);
        assert_eq!(user_info.username, online_user.username);
        assert_eq!(user_info.connected_at, Some(online_user.connected_at));

        // 测试从UserInfo转换为OnlineUser
        let converted_back: OnlineUser = user_info.into();
        assert_eq!(converted_back.user_id, online_user.user_id);
        assert_eq!(converted_back.username, online_user.username);
        assert_eq!(converted_back.connected_at, online_user.connected_at);
        assert_eq!(converted_back.connection_count, 1); // 默认值
    }
}
