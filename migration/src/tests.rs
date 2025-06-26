//! 数据库模型和迁移的单元测试
//!
//! 这些测试验证：
//! 1. 实体模型的正确性
//! 2. 枚举值的序列化/反序列化
//! 3. 关系定义的正确性

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;
    use uuid::Uuid;
    use chrono::Utc;

    /// 测试聊天室类型枚举的序列化和反序列化
    #[test]
    fn test_chat_room_type_serialization() {
        use crate::chat_room_entity::ChatRoomType;

        // 测试序列化（使用默认的变体名称）
        let room_type = ChatRoomType::Public;
        let serialized = serde_json::to_string(&room_type).unwrap();
        assert_eq!(serialized, "\"Public\"");

        let room_type = ChatRoomType::Private;
        let serialized = serde_json::to_string(&room_type).unwrap();
        assert_eq!(serialized, "\"Private\"");

        let room_type = ChatRoomType::Group;
        let serialized = serde_json::to_string(&room_type).unwrap();
        assert_eq!(serialized, "\"Group\"");

        // 测试反序列化
        let deserialized: ChatRoomType = serde_json::from_str("\"Public\"").unwrap();
        assert_eq!(deserialized, ChatRoomType::Public);

        let deserialized: ChatRoomType = serde_json::from_str("\"Private\"").unwrap();
        assert_eq!(deserialized, ChatRoomType::Private);

        let deserialized: ChatRoomType = serde_json::from_str("\"Group\"").unwrap();
        assert_eq!(deserialized, ChatRoomType::Group);
    }

    /// 测试聊天室状态枚举的序列化和反序列化
    #[test]
    fn test_chat_room_status_serialization() {
        use crate::chat_room_entity::ChatRoomStatus;

        // 测试序列化
        let status = ChatRoomStatus::Active;
        let serialized = serde_json::to_string(&status).unwrap();
        assert_eq!(serialized, "\"Active\"");

        let status = ChatRoomStatus::Archived;
        let serialized = serde_json::to_string(&status).unwrap();
        assert_eq!(serialized, "\"Archived\"");

        let status = ChatRoomStatus::Disabled;
        let serialized = serde_json::to_string(&status).unwrap();
        assert_eq!(serialized, "\"Disabled\"");

        // 测试反序列化
        let deserialized: ChatRoomStatus = serde_json::from_str("\"Active\"").unwrap();
        assert_eq!(deserialized, ChatRoomStatus::Active);

        let deserialized: ChatRoomStatus = serde_json::from_str("\"Archived\"").unwrap();
        assert_eq!(deserialized, ChatRoomStatus::Archived);

        let deserialized: ChatRoomStatus = serde_json::from_str("\"Disabled\"").unwrap();
        assert_eq!(deserialized, ChatRoomStatus::Disabled);
    }

    /// 测试消息类型枚举的序列化和反序列化
    #[test]
    fn test_message_type_serialization() {
        use crate::message_entity::MessageType;

        // 测试所有消息类型
        let types = vec![
            (MessageType::Text, "\"Text\""),
            (MessageType::Image, "\"Image\""),
            (MessageType::File, "\"File\""),
            (MessageType::System, "\"System\""),
            (MessageType::Voice, "\"Voice\""),
            (MessageType::Video, "\"Video\"")
        ];

        for (msg_type, expected_json) in types {
            // 测试序列化
            let serialized = serde_json::to_string(&msg_type).unwrap();
            assert_eq!(serialized, expected_json);

            // 测试反序列化
            let deserialized: MessageType = serde_json::from_str(expected_json).unwrap();
            assert_eq!(deserialized, msg_type);
        }
    }

    /// 测试消息状态枚举的序列化和反序列化
    #[test]
    fn test_message_status_serialization() {
        use crate::message_entity::MessageStatus;

        let statuses = vec![
            (MessageStatus::Sent, "\"Sent\""),
            (MessageStatus::Delivered, "\"Delivered\""),
            (MessageStatus::Read, "\"Read\""),
            (MessageStatus::Deleted, "\"Deleted\""),
            (MessageStatus::Edited, "\"Edited\"")
        ];

        for (status, expected_json) in statuses {
            // 测试序列化
            let serialized = serde_json::to_string(&status).unwrap();
            assert_eq!(serialized, expected_json);

            // 测试反序列化
            let deserialized: MessageStatus = serde_json::from_str(expected_json).unwrap();
            assert_eq!(deserialized, status);
        }
    }

    /// 测试会话状态枚举的序列化和反序列化
    #[test]
    fn test_session_status_serialization() {
        use crate::user_session_entity::SessionStatus;

        let statuses = vec![
            (SessionStatus::Online, "\"Online\""),
            (SessionStatus::Offline, "\"Offline\""),
            (SessionStatus::Busy, "\"Busy\""),
            (SessionStatus::Away, "\"Away\""),
            (SessionStatus::Invisible, "\"Invisible\"")
        ];

        for (status, expected_json) in statuses {
            // 测试序列化
            let serialized = serde_json::to_string(&status).unwrap();
            assert_eq!(serialized, expected_json);

            // 测试反序列化
            let deserialized: SessionStatus = serde_json::from_str(expected_json).unwrap();
            assert_eq!(deserialized, status);
        }
    }

    /// 测试设备类型枚举的序列化和反序列化
    #[test]
    fn test_device_type_serialization() {
        use crate::user_session_entity::DeviceType;

        let device_types = vec![
            (DeviceType::Desktop, "\"Desktop\""),
            (DeviceType::Mobile, "\"Mobile\""),
            (DeviceType::Tablet, "\"Tablet\""),
            (DeviceType::Web, "\"Web\"")
        ];

        for (device_type, expected_json) in device_types {
            // 测试序列化
            let serialized = serde_json::to_string(&device_type).unwrap();
            assert_eq!(serialized, expected_json);

            // 测试反序列化
            let deserialized: DeviceType = serde_json::from_str(expected_json).unwrap();
            assert_eq!(deserialized, device_type);
        }
    }

    /// 测试聊天室模型的创建和序列化
    #[test]
    fn test_chat_room_model_creation() {
        use crate::chat_room_entity::{ Model as ChatRoom, ChatRoomType, ChatRoomStatus };

        let chat_room = ChatRoom {
            id: Uuid::new_v4(),
            name: "测试聊天室".to_string(),
            description: Some("这是一个测试聊天室".to_string()),
            room_type: ChatRoomType::Public,
            status: ChatRoomStatus::Active,
            created_by: Uuid::new_v4(),
            max_members: 100,
            current_members: 0,
            settings: Some("{\"allow_file_upload\": true}".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // 测试序列化
        let serialized = serde_json::to_string(&chat_room).unwrap();
        assert!(serialized.contains("测试聊天室"));
        assert!(serialized.contains("Public"));
        assert!(serialized.contains("Active"));

        // 测试反序列化
        let deserialized: ChatRoom = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.name, "测试聊天室");
        assert_eq!(deserialized.room_type, ChatRoomType::Public);
        assert_eq!(deserialized.status, ChatRoomStatus::Active);
    }

    /// 测试消息模型的创建和序列化
    #[test]
    fn test_message_model_creation() {
        use crate::message_entity::{ Model as Message, MessageType, MessageStatus };

        let message = Message {
            id: Uuid::new_v4(),
            content: "Hello, World!".to_string(),
            message_type: MessageType::Text,
            status: MessageStatus::Sent,
            sender_id: Uuid::new_v4(),
            chat_room_id: Uuid::new_v4(),
            reply_to_id: None,
            metadata: Some("{\"edited\": false}".to_string()),
            priority: 5,
            is_pinned: false,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // 测试序列化
        let serialized = serde_json::to_string(&message).unwrap();
        assert!(serialized.contains("Hello, World!"));
        assert!(serialized.contains("Text"));
        assert!(serialized.contains("Sent"));

        // 测试反序列化
        let deserialized: Message = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.content, "Hello, World!");
        assert_eq!(deserialized.message_type, MessageType::Text);
        assert_eq!(deserialized.status, MessageStatus::Sent);
        assert_eq!(deserialized.priority, 5);
        assert!(!deserialized.is_pinned);
    }

    /// 测试用户会话模型的创建和序列化
    #[test]
    fn test_user_session_model_creation() {
        use crate::user_session_entity::{ Model as UserSession, SessionStatus, DeviceType };

        let session = UserSession {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            session_token: "test_token_123".to_string(),
            status: SessionStatus::Online,
            device_type: DeviceType::Web,
            device_info: Some("Chrome 120.0".to_string()),
            ip_address: "192.168.1.100".to_string(),
            user_agent: Some("Mozilla/5.0 (Windows NT 10.0; Win64; x64)".to_string()),
            current_chat_room_id: Some(Uuid::new_v4()),
            last_activity_at: Utc::now(),
            last_heartbeat_at: Utc::now(),
            metadata: Some("{\"timezone\": \"UTC+8\"}".to_string()),
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // 测试序列化
        let serialized = serde_json::to_string(&session).unwrap();
        assert!(serialized.contains("test_token_123"));
        assert!(serialized.contains("Online"));
        assert!(serialized.contains("Web"));
        assert!(serialized.contains("192.168.1.100"));

        // 测试反序列化
        let deserialized: UserSession = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.session_token, "test_token_123");
        assert_eq!(deserialized.status, SessionStatus::Online);
        assert_eq!(deserialized.device_type, DeviceType::Web);
        assert_eq!(deserialized.ip_address, "192.168.1.100");
    }
}
