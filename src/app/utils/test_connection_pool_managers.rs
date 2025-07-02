//! 连接池管理器单元测试模块
//!
//! 【任务13.4】连接池优化 - 单元测试
//!
//! 本模块测试数据库连接池管理器和WebSocket连接池管理器的功能

#[cfg(test)]
mod tests {
    use super::super::{ DatabasePoolManager, WebSocketPoolManager };
    use crate::config::{ AppConfig, DatabasePoolConfig, WebSocketPoolConfig };
    use axum::extract::ws::Message;
    use tokio::sync::mpsc;
    use uuid::Uuid;

    /// 创建测试用的应用配置
    fn create_test_config() -> AppConfig {
        AppConfig {
            http_addr: "127.0.0.1:3000".parse().unwrap(),
            database_url: "sqlite::memory:".to_string(),
            jwt_secret: "test_secret".to_string(),
            database_pool: DatabasePoolConfig::development(),
            websocket_pool: WebSocketPoolConfig::development(),
        }
    }

    #[tokio::test]
    async fn test_database_pool_manager_creation() {
        // 测试数据库连接池管理器的创建
        let config = create_test_config();
        let pool_manager = DatabasePoolManager::new(&config).await;

        assert!(pool_manager.is_ok(), "数据库连接池管理器创建应该成功");

        let pool_manager = pool_manager.unwrap();
        let metrics = pool_manager.get_metrics();

        // 验证初始指标
        assert!(metrics.is_healthy(), "连接池应该是健康的");
        assert_eq!(metrics.total_connections.load(std::sync::atomic::Ordering::Relaxed), 20);
    }

    #[tokio::test]
    async fn test_database_pool_manager_connection() {
        // 测试数据库连接获取
        let config = create_test_config();
        let pool_manager = DatabasePoolManager::new(&config).await.unwrap();

        // 获取连接
        let connection = pool_manager.get_connection();

        // 验证连接可用性
        assert!(connection.ping().await.is_ok(), "数据库连接应该可用");

        // 验证指标更新
        let metrics = pool_manager.get_metrics();
        assert!(metrics.total_acquires.load(std::sync::atomic::Ordering::Relaxed) > 0);
    }

    #[tokio::test]
    async fn test_database_pool_manager_health_check() {
        // 测试数据库连接池健康检查
        let config = create_test_config();
        let pool_manager = DatabasePoolManager::new(&config).await.unwrap();

        // 执行健康检查
        let health_result = pool_manager.health_check().await;

        assert!(health_result.is_ok(), "健康检查应该成功");
        assert!(health_result.unwrap(), "连接池应该是健康的");
    }

    #[tokio::test]
    async fn test_websocket_pool_manager_creation() {
        // 测试WebSocket连接池管理器的创建
        let config = create_test_config();
        let pool_manager = WebSocketPoolManager::new(config.websocket_pool);

        let metrics = pool_manager.get_metrics();

        // 验证初始状态
        assert!(metrics.is_healthy(), "WebSocket连接池应该是健康的");
        assert_eq!(metrics.active_connections.load(std::sync::atomic::Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn test_websocket_pool_manager_add_connection() {
        // 测试WebSocket连接添加
        let config = create_test_config();
        let pool_manager = WebSocketPoolManager::new(config.websocket_pool);

        // 创建测试连接
        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let (sender, _receiver) = mpsc::unbounded_channel();

        // 添加连接
        let result = pool_manager.add_connection(connection_id, user_id, sender).await;

        assert!(result.is_ok(), "添加连接应该成功");

        // 验证指标更新
        let metrics = pool_manager.get_metrics();
        assert_eq!(metrics.active_connections.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_websocket_pool_manager_remove_connection() {
        // 测试WebSocket连接移除
        let config = create_test_config();
        let pool_manager = WebSocketPoolManager::new(config.websocket_pool);

        // 添加连接
        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let (sender, _receiver) = mpsc::unbounded_channel();

        pool_manager.add_connection(connection_id, user_id, sender).await.unwrap();

        // 移除连接
        let removed = pool_manager.remove_connection(&connection_id).await;

        assert!(removed.is_some(), "应该成功移除连接");

        // 验证指标更新
        let metrics = pool_manager.get_metrics();
        assert_eq!(metrics.active_connections.load(std::sync::atomic::Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn test_websocket_pool_manager_send_message() {
        // 测试WebSocket消息发送
        let config = create_test_config();
        let pool_manager = WebSocketPoolManager::new(config.websocket_pool);

        // 添加连接
        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let (sender, mut receiver) = mpsc::unbounded_channel();

        pool_manager.add_connection(connection_id, user_id, sender).await.unwrap();

        // 发送消息
        let test_message = Message::Text("Hello, World!".to_string().into());
        let result = pool_manager.send_to_connection(&connection_id, test_message.clone()).await;

        assert!(result.is_ok(), "消息发送应该成功");

        // 验证消息接收
        let received_message = receiver.try_recv();
        assert!(received_message.is_ok(), "应该接收到消息");

        // 验证指标更新
        let metrics = pool_manager.get_metrics();
        assert!(metrics.total_messages_sent.load(std::sync::atomic::Ordering::Relaxed) > 0);
    }

    #[tokio::test]
    async fn test_websocket_pool_manager_broadcast_to_user() {
        // 测试用户广播功能
        let config = create_test_config();
        let pool_manager = WebSocketPoolManager::new(config.websocket_pool);

        let user_id = Uuid::new_v4();

        // 为同一用户添加多个连接
        let connection_id1 = Uuid::new_v4();
        let connection_id2 = Uuid::new_v4();
        let (sender1, mut receiver1) = mpsc::unbounded_channel();
        let (sender2, mut receiver2) = mpsc::unbounded_channel();

        pool_manager.add_connection(connection_id1, user_id, sender1).await.unwrap();
        pool_manager.add_connection(connection_id2, user_id, sender2).await.unwrap();

        // 广播消息
        let test_message = Message::Text("Broadcast message".to_string().into());
        let result = pool_manager.broadcast_to_user(&user_id, test_message).await;

        assert!(result.is_ok(), "广播应该成功");
        assert_eq!(result.unwrap(), 2, "应该发送到2个连接");

        // 验证两个连接都收到消息
        assert!(receiver1.try_recv().is_ok(), "连接1应该收到消息");
        assert!(receiver2.try_recv().is_ok(), "连接2应该收到消息");
    }

    #[tokio::test]
    async fn test_websocket_pool_manager_health_check() {
        // 测试WebSocket连接池健康检查
        let config = create_test_config();
        let pool_manager = WebSocketPoolManager::new(config.websocket_pool);

        // 执行健康检查
        let health_result = pool_manager.health_check().await;

        assert!(health_result.is_ok(), "健康检查应该成功");
        assert!(health_result.unwrap(), "连接池应该是健康的");
    }

    #[tokio::test]
    async fn test_websocket_pool_manager_connection_limit() {
        // 测试WebSocket连接数限制
        let mut config = create_test_config();
        config.websocket_pool.max_connections = 2; // 设置最大连接数为2

        let pool_manager = WebSocketPoolManager::new(config.websocket_pool);

        // 添加2个连接（应该成功）
        for i in 0..2 {
            let connection_id = Uuid::new_v4();
            let user_id = Uuid::new_v4();
            let (sender, _receiver) = mpsc::unbounded_channel();

            let result = pool_manager.add_connection(connection_id, user_id, sender).await;
            assert!(result.is_ok(), "前2个连接应该成功添加");
        }

        // 尝试添加第3个连接（应该失败）
        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let (sender, _receiver) = mpsc::unbounded_channel();

        let result = pool_manager.add_connection(connection_id, user_id, sender).await;
        assert!(result.is_err(), "第3个连接应该被拒绝");
        assert!(result.unwrap_err().contains("连接池已满"), "错误消息应该指示连接池已满");
    }

    #[tokio::test]
    async fn test_connection_pool_metrics() {
        // 测试连接池指标收集
        let config = create_test_config();
        let db_pool_manager = DatabasePoolManager::new(&config).await.unwrap();
        let ws_pool_manager = WebSocketPoolManager::new(config.websocket_pool);

        // 测试数据库连接池指标
        let db_metrics = db_pool_manager.get_metrics();
        assert!(db_metrics.is_healthy());
        assert_eq!(db_metrics.get_success_rate(), 100.0);

        // 获取一个连接来更新指标
        let _connection = db_pool_manager.get_connection();
        assert!(db_metrics.total_acquires.load(std::sync::atomic::Ordering::Relaxed) > 0);

        // 测试WebSocket连接池指标
        let ws_metrics = ws_pool_manager.get_metrics();
        assert!(ws_metrics.is_healthy());
        assert_eq!(ws_metrics.get_utilization_percentage(), 0.0);

        // 添加连接来更新指标
        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let (sender, _receiver) = mpsc::unbounded_channel();

        ws_pool_manager.add_connection(connection_id, user_id, sender).await.unwrap();
        assert!(ws_metrics.active_connections.load(std::sync::atomic::Ordering::Relaxed) > 0);
    }
}
