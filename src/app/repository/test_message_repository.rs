//! `test_message_repository.rs`
//!
//! 消息仓库的单元测试模块
//! 测试消息搜索和过滤功能的正确性

#[cfg(test)]
mod tests {
    use super::super::message_repository::*;
    use migration::{ message_entity::{ MessageStatus, MessageType }, MigratorTrait };
    use sea_orm::{ Database, DatabaseConnection, DbErr };
    use uuid::Uuid;
    use chrono::Utc;

    /// 创建内存数据库连接用于测试
    async fn setup_test_db() -> Result<DatabaseConnection, DbErr> {
        let db = Database::connect("sqlite::memory:").await?;

        // 运行迁移
        migration::Migrator::up(&db, None).await?;

        Ok(db)
    }

    /// 创建测试消息数据
    async fn create_test_messages(repo: &MessageRepository) -> Result<Vec<Uuid>, DbErr> {
        let chat_room_id = Uuid::new_v4();
        let user1_id = Uuid::new_v4();
        let user2_id = Uuid::new_v4();
        let now = Utc::now();

        // 首先创建用户数据
        let user1_active = migration::user_entity::ActiveModel {
            id: sea_orm::Set(user1_id),
            username: sea_orm::Set("user1".to_string()),
            password_hash: sea_orm::Set("hash1".to_string()),
            created_at: sea_orm::Set(now),
            updated_at: sea_orm::Set(now),
        };

        let user2_active = migration::user_entity::ActiveModel {
            id: sea_orm::Set(user2_id),
            username: sea_orm::Set("user2".to_string()),
            password_hash: sea_orm::Set("hash2".to_string()),
            created_at: sea_orm::Set(now),
            updated_at: sea_orm::Set(now),
        };

        // 插入用户
        use sea_orm::EntityTrait;
        migration::user_entity::Entity::insert(user1_active).exec(repo.db()).await?;
        migration::user_entity::Entity::insert(user2_active).exec(repo.db()).await?;

        // 创建聊天室数据
        let chat_room_active = migration::chat_room_entity::ActiveModel {
            id: sea_orm::Set(chat_room_id),
            name: sea_orm::Set("Test Room".to_string()),
            description: sea_orm::Set(Some("Test chat room".to_string())),
            room_type: sea_orm::Set(migration::chat_room_entity::ChatRoomType::Public),
            status: sea_orm::Set(migration::chat_room_entity::ChatRoomStatus::Active),
            created_by: sea_orm::Set(user1_id),
            max_members: sea_orm::Set(100),
            current_members: sea_orm::Set(0),
            settings: sea_orm::Set(None),
            created_at: sea_orm::Set(now),
            updated_at: sea_orm::Set(now),
        };

        // 插入聊天室
        migration::chat_room_entity::Entity::insert(chat_room_active).exec(repo.db()).await?;

        let mut message_ids = Vec::new();

        // 创建测试消息
        let messages = vec![
            (
                "Hello world",
                MessageType::Text,
                MessageStatus::Sent,
                user1_id,
                chat_room_id,
                now - chrono::Duration::hours(2),
            ),
            (
                "How are you?",
                MessageType::Text,
                MessageStatus::Delivered,
                user2_id,
                chat_room_id,
                now - chrono::Duration::hours(1),
            ),
            (
                "I'm fine, thanks!",
                MessageType::Text,
                MessageStatus::Read,
                user1_id,
                chat_room_id,
                now - chrono::Duration::minutes(30),
            ),
            (
                "Check this image",
                MessageType::Image,
                MessageStatus::Sent,
                user2_id,
                chat_room_id,
                now - chrono::Duration::minutes(15),
            ),
            (
                "System notification",
                MessageType::System,
                MessageStatus::Sent,
                user1_id,
                chat_room_id,
                now - chrono::Duration::minutes(5),
            )
        ];

        for (content, msg_type, status, sender_id, room_id, created_at) in messages {
            let message_id = Uuid::new_v4();
            let active_model = migration::message_entity::ActiveModel {
                id: sea_orm::Set(message_id),
                content: sea_orm::Set(content.to_string()),
                message_type: sea_orm::Set(msg_type),
                status: sea_orm::Set(status),
                sender_id: sea_orm::Set(sender_id),
                chat_room_id: sea_orm::Set(room_id),
                reply_to_id: sea_orm::Set(None),
                metadata: sea_orm::Set(None),
                priority: sea_orm::Set(5),
                is_pinned: sea_orm::Set(false),
                expires_at: sea_orm::Set(None),
                created_at: sea_orm::Set(created_at),
                updated_at: sea_orm::Set(created_at),
            };

            repo.create(active_model).await?;
            message_ids.push(message_id);
        }

        Ok(message_ids)
    }

    #[tokio::test]
    async fn test_search_messages_by_keyword() {
        let db = setup_test_db().await.expect("Failed to setup test database");
        let repo = MessageRepository::new(db);

        // 创建测试数据
        create_test_messages(&repo).await.expect("Failed to create test messages");

        // 测试关键词搜索
        let params = MessagePaginationParams {
            page: 1,
            page_size: 10,
            desc_order: true,
        };

        let result = repo
            .search_messages("Hello".to_string(), params, None).await
            .expect("Failed to search messages");

        assert_eq!(result.messages.len(), 1);
        assert!(result.messages[0].content.contains("Hello"));
        assert_eq!(result.total_count, 1);
    }

    #[tokio::test]
    async fn test_search_messages_with_filter() {
        let db = setup_test_db().await.expect("Failed to setup test database");
        let repo = MessageRepository::new(db);

        // 创建测试数据
        create_test_messages(&repo).await.expect("Failed to create test messages");

        // 测试带过滤器的搜索
        let filter = MessageFilter {
            message_type: Some(MessageType::Text),
            status: Some(MessageStatus::Sent),
            ..Default::default()
        };

        let params = MessagePaginationParams {
            page: 1,
            page_size: 10,
            desc_order: true,
        };

        let result = repo
            .search_messages("Hello".to_string(), params, Some(filter)).await
            .expect("Failed to search messages with filter");

        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.messages[0].message_type, MessageType::Text);
        assert_eq!(result.messages[0].status, MessageStatus::Sent);
    }

    #[tokio::test]
    async fn test_find_by_chat_room_with_keyword_filter() {
        let db = setup_test_db().await.expect("Failed to setup test database");
        let repo = MessageRepository::new(db);

        // 创建测试数据
        let message_ids = create_test_messages(&repo).await.expect(
            "Failed to create test messages"
        );

        // 获取第一条消息的聊天室ID
        let first_message = repo
            .find_by_id(message_ids[0]).await
            .expect("Failed to find message")
            .expect("Message not found");
        let chat_room_id = first_message.chat_room_id;

        // 测试聊天室消息查询带关键词过滤
        let filter = MessageFilter {
            keyword: Some("are".to_string()),
            ..Default::default()
        };

        let params = MessagePaginationParams {
            page: 1,
            page_size: 10,
            desc_order: true,
        };

        let result = repo
            .find_by_chat_room(chat_room_id, params, Some(filter)).await
            .expect("Failed to find messages by chat room");

        assert_eq!(result.messages.len(), 1);
        assert!(result.messages[0].content.contains("are"));
    }

    #[tokio::test]
    async fn test_pagination() {
        let db = setup_test_db().await.expect("Failed to setup test database");
        let repo = MessageRepository::new(db);

        // 创建测试数据
        create_test_messages(&repo).await.expect("Failed to create test messages");

        // 测试分页
        let params = MessagePaginationParams {
            page: 1,
            page_size: 2,
            desc_order: true,
        };

        let result = repo.search_messages("".to_string(), params, None).await;

        // 空关键词应该返回错误
        assert!(result.is_err());

        // 测试有效的分页查询
        let params = MessagePaginationParams {
            page: 1,
            page_size: 2,
            desc_order: true,
        };

        let result = repo
            .search_messages("e".to_string(), params, None)
            .await // 搜索包含'e'的消息
            .expect("Failed to search with pagination");

        assert!(result.messages.len() <= 2);
        assert_eq!(result.page_size, 2);
        assert_eq!(result.current_page, 1);
        assert!(result.total_count > 0);
    }

    #[tokio::test]
    async fn test_date_range_filter() {
        let db = setup_test_db().await.expect("Failed to setup test database");
        let repo = MessageRepository::new(db);

        // 创建测试数据
        create_test_messages(&repo).await.expect("Failed to create test messages");

        let now = Utc::now();
        let filter = MessageFilter {
            start_time: Some(now - chrono::Duration::hours(1)),
            end_time: Some(now),
            ..Default::default()
        };

        let params = MessagePaginationParams {
            page: 1,
            page_size: 10,
            desc_order: true,
        };

        let result = repo
            .search_messages("e".to_string(), params, Some(filter)).await
            .expect("Failed to search with date filter");

        // 应该只返回最近1小时内的消息
        assert!(result.messages.len() > 0);
        for message in &result.messages {
            assert!(message.created_at >= now - chrono::Duration::hours(1));
            assert!(message.created_at <= now);
        }
    }

    #[tokio::test]
    async fn test_message_type_filter() {
        let db = setup_test_db().await.expect("Failed to setup test database");
        let repo = MessageRepository::new(db);

        // 创建测试数据
        create_test_messages(&repo).await.expect("Failed to create test messages");

        let filter = MessageFilter {
            message_type: Some(MessageType::Image),
            ..Default::default()
        };

        let params = MessagePaginationParams {
            page: 1,
            page_size: 10,
            desc_order: true,
        };

        let result = repo
            .search_messages("image".to_string(), params, Some(filter)).await
            .expect("Failed to search with message type filter");

        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.messages[0].message_type, MessageType::Image);
    }
}
