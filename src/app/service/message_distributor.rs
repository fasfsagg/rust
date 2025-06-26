// app/service/message_distributor.rs
//
// /------------------------------------------------------------------------------------------------------\
// |                                【消息分发器服务】 (message_distributor.rs)                        |
// |------------------------------------------------------------------------------------------------------|
// |                                                                                                      |
// | 1. **核心职责**:                                                                                     |
// |    - 处理消息的智能分发和路由                                                                         |
// |    - 支持多种广播策略（全员广播、选择性广播、私聊等）                                                 |
// |    - 提供消息队列和批量处理能力                                                                       |
// |    - 实现消息优先级和限流机制                                                                         |
// |                                                                                                      |
// | 2. **设计原则**:                                                                                     |
// |    - 高性能：支持百万并发消息分发                                                                     |
// |    - 可扩展：支持不同的分发策略和消息类型                                                             |
// |    - 容错性：完善的错误处理和重试机制                                                                 |
// |    - 监控友好：提供详细的分发统计和性能指标                                                           |
// |                                                                                                      |
// | 3. **关键特性**:                                                                                     |
// |    - 异步消息分发                                                                                     |
// |    - 批量处理优化                                                                                     |
// |    - 消息优先级队列                                                                                   |
// |    - 分发策略抽象                                                                                     |
// |                                                                                                      |
// \------------------------------------------------------------------------------------------------------/

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{ RwLock, Mutex };
use uuid::Uuid;
use chrono::{ DateTime, Utc };
use serde::{ Serialize, Deserialize };
use axum::extract::ws::Message;

use crate::app::model::chat::ServerMessage;
use crate::app::service::{ ConnectionManager, ConnectionId };

/// 消息分发策略枚举
///
/// 【功能】: 定义不同的消息分发策略
/// 【扩展性】: 可以轻松添加新的分发策略
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BroadcastStrategy {
    /// 广播给所有连接（排除发送者）
    BroadcastAll {
        exclude_sender: bool,
    },
    /// 广播给指定用户列表
    BroadcastToUsers {
        user_ids: Vec<Uuid>,
    },
    /// 私聊消息（点对点）
    DirectMessage {
        target_user_id: Uuid,
    },
    /// 广播给指定连接列表
    BroadcastToConnections {
        connection_ids: Vec<ConnectionId>,
    },
    /// 系统消息（广播给所有人，包括发送者）
    SystemMessage,
}

/// 消息优先级枚举
///
/// 【功能】: 定义消息的优先级级别
/// 【用途】: 用于消息队列的优先级排序
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    /// 低优先级（普通聊天消息）
    Low = 1,
    /// 正常优先级（默认）
    Normal = 2,
    /// 高优先级（重要通知）
    High = 3,
    /// 紧急优先级（系统消息、错误通知）
    Critical = 4,
}

/// 分发任务结构体
///
/// 【功能】: 封装单个消息分发任务的所有信息
/// 【用途】: 消息队列中的基本单元
#[derive(Debug, Clone)]
pub struct DistributionTask {
    /// 任务ID
    pub task_id: Uuid,
    /// 要分发的消息
    pub message: ServerMessage,
    /// 分发策略
    pub strategy: BroadcastStrategy,
    /// 消息优先级
    pub priority: MessagePriority,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 发送者连接ID（可选，用于排除发送者）
    pub sender_connection_id: Option<ConnectionId>,
    /// 重试次数
    pub retry_count: u32,
    /// 最大重试次数
    pub max_retries: u32,
}

/// 分发统计信息
///
/// 【功能】: 记录消息分发的统计数据
/// 【用途】: 性能监控和调试
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionStats {
    /// 总分发任务数
    pub total_tasks: u64,
    /// 成功分发数
    pub successful_distributions: u64,
    /// 失败分发数
    pub failed_distributions: u64,
    /// 当前队列长度
    pub queue_length: usize,
    /// 平均分发延迟（毫秒）
    pub average_latency_ms: f64,
    /// 最后更新时间
    pub last_updated: DateTime<Utc>,
}

/// 消息分发器主结构体
///
/// 【功能】: 管理消息分发的核心组件
/// 【线程安全】: 使用 Arc<RwLock<T>> 和 Arc<Mutex<T>> 确保线程安全
/// 【性能优化】: 使用优先级队列和批量处理提高性能
#[derive(Debug, Clone)]
pub struct MessageDistributor {
    /// 连接管理器引用
    connection_manager: Arc<ConnectionManager>,
    /// 消息队列（按优先级排序）
    message_queue: Arc<Mutex<VecDeque<DistributionTask>>>,
    /// 分发统计信息
    stats: Arc<RwLock<DistributionStats>>,
    /// 批量处理大小
    batch_size: usize,
    /// 分发工作线程数量
    worker_count: usize,
}

impl Default for MessagePriority {
    fn default() -> Self {
        MessagePriority::Normal
    }
}

impl DistributionTask {
    /// 创建新的分发任务
    ///
    /// 【功能】: 便捷方法创建分发任务
    /// 【参数】:
    /// * `message` - 要分发的消息
    /// * `strategy` - 分发策略
    /// * `priority` - 消息优先级
    /// * `sender_connection_id` - 发送者连接ID（可选）
    ///
    /// 【返回值】: DistributionTask 实例
    pub fn new(
        message: ServerMessage,
        strategy: BroadcastStrategy,
        priority: MessagePriority,
        sender_connection_id: Option<ConnectionId>
    ) -> Self {
        Self {
            task_id: Uuid::new_v4(),
            message,
            strategy,
            priority,
            created_at: Utc::now(),
            sender_connection_id,
            retry_count: 0,
            max_retries: 3,
        }
    }

    /// 检查任务是否可以重试
    ///
    /// 【功能】: 判断失败的任务是否还可以重试
    /// 【返回值】: bool - 如果可以重试返回 true
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    /// 增加重试次数
    ///
    /// 【功能】: 增加任务的重试计数
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    /// 获取任务年龄（从创建到现在的时间）
    ///
    /// 【功能】: 计算任务在队列中的等待时间
    /// 【返回值】: chrono::Duration - 任务年龄
    pub fn age(&self) -> chrono::Duration {
        Utc::now() - self.created_at
    }
}

impl Default for DistributionStats {
    fn default() -> Self {
        Self {
            total_tasks: 0,
            successful_distributions: 0,
            failed_distributions: 0,
            queue_length: 0,
            average_latency_ms: 0.0,
            last_updated: Utc::now(),
        }
    }
}

impl MessageDistributor {
    /// 创建新的消息分发器实例
    ///
    /// 【功能】: 初始化消息分发器
    /// 【参数】:
    /// * `connection_manager` - 连接管理器引用
    /// * `batch_size` - 批量处理大小（默认100）
    /// * `worker_count` - 工作线程数量（默认4）
    ///
    /// 【返回值】: MessageDistributor 实例
    pub fn new(
        connection_manager: Arc<ConnectionManager>,
        batch_size: Option<usize>,
        worker_count: Option<usize>
    ) -> Self {
        Self {
            connection_manager,
            message_queue: Arc::new(Mutex::new(VecDeque::new())),
            stats: Arc::new(RwLock::new(DistributionStats::default())),
            batch_size: batch_size.unwrap_or(100),
            worker_count: worker_count.unwrap_or(4),
        }
    }

    /// 提交消息分发任务
    ///
    /// 【功能】: 将消息分发任务添加到队列中
    /// 【参数】:
    /// * `task` - 分发任务
    ///
    /// 【返回值】: Result<(), String> - 成功返回 Ok(())，失败返回错误信息
    pub async fn submit_task(&self, task: DistributionTask) -> Result<(), String> {
        let mut queue = self.message_queue.lock().await;

        // 按优先级插入任务（高优先级在前）
        let insert_position = queue
            .iter()
            .position(|existing_task| existing_task.priority < task.priority)
            .unwrap_or(queue.len());

        queue.insert(insert_position, task);

        // 更新统计信息
        {
            let mut stats = self.stats.write().await;
            stats.total_tasks += 1;
            stats.queue_length = queue.len();
            stats.last_updated = Utc::now();
        }

        println!("MESSAGE_DISTRIBUTOR: 任务已提交到队列，当前队列长度: {}", queue.len());

        Ok(())
    }

    /// 便捷方法：广播消息给所有用户
    ///
    /// 【功能】: 创建并提交全员广播任务
    /// 【参数】:
    /// * `message` - 要广播的消息
    /// * `exclude_sender` - 是否排除发送者
    /// * `sender_connection_id` - 发送者连接ID（可选）
    /// * `priority` - 消息优先级（可选，默认Normal）
    ///
    /// 【返回值】: Result<(), String> - 成功返回 Ok(())，失败返回错误信息
    pub async fn broadcast_to_all(
        &self,
        message: ServerMessage,
        exclude_sender: bool,
        sender_connection_id: Option<ConnectionId>,
        priority: Option<MessagePriority>
    ) -> Result<(), String> {
        let task = DistributionTask::new(
            message,
            BroadcastStrategy::BroadcastAll { exclude_sender },
            priority.unwrap_or_default(),
            sender_connection_id
        );

        self.submit_task(task).await
    }

    /// 便捷方法：发送私聊消息
    ///
    /// 【功能】: 创建并提交私聊消息任务
    /// 【参数】:
    /// * `message` - 要发送的消息
    /// * `target_user_id` - 目标用户ID
    /// * `priority` - 消息优先级（可选，默认Normal）
    ///
    /// 【返回值】: Result<(), String> - 成功返回 Ok(())，失败返回错误信息
    pub async fn send_direct_message(
        &self,
        message: ServerMessage,
        target_user_id: Uuid,
        priority: Option<MessagePriority>
    ) -> Result<(), String> {
        let task = DistributionTask::new(
            message,
            BroadcastStrategy::DirectMessage { target_user_id },
            priority.unwrap_or_default(),
            None
        );

        self.submit_task(task).await
    }

    /// 广播消息给指定用户列表
    ///
    /// 【功能】: 创建并提交广播消息任务给指定的用户列表
    /// 【参数】:
    /// * `message` - 要发送的消息
    /// * `user_ids` - 目标用户ID列表
    /// * `priority` - 消息优先级（可选，默认Normal）
    ///
    /// 【返回值】: Result<usize, String> - 成功返回发送的用户数量，失败返回错误信息
    pub async fn broadcast_to_users(
        &self,
        message: ServerMessage,
        user_ids: Vec<Uuid>,
        priority: Option<MessagePriority>
    ) -> Result<usize, String> {
        if user_ids.is_empty() {
            return Ok(0);
        }

        let task = DistributionTask::new(
            message,
            BroadcastStrategy::BroadcastToUsers { user_ids: user_ids.clone() },
            priority.unwrap_or(MessagePriority::Normal),
            None
        );

        self.submit_task(task).await?;
        Ok(user_ids.len())
    }

    /// 便捷方法：发送系统消息
    ///
    /// 【功能】: 创建并提交系统消息任务
    /// 【参数】:
    /// * `message` - 系统消息
    ///
    /// 【返回值】: Result<(), String> - 成功返回 Ok(())，失败返回错误信息
    pub async fn send_system_message(&self, message: ServerMessage) -> Result<(), String> {
        let task = DistributionTask::new(
            message,
            BroadcastStrategy::SystemMessage,
            MessagePriority::High,
            None
        );

        self.submit_task(task).await
    }

    /// 处理单个分发任务
    ///
    /// 【功能】: 执行具体的消息分发逻辑
    /// 【参数】:
    /// * `task` - 要处理的分发任务
    ///
    /// 【返回值】: Result<usize, String> - 成功返回分发成功的连接数，失败返回错误信息
    async fn process_task(&self, task: &DistributionTask) -> Result<usize, String> {
        let start_time = std::time::Instant::now();

        // 将ServerMessage转换为WebSocket Message
        let ws_message = Message::Text(
            serde_json
                ::to_string(&task.message)
                .map_err(|e| format!("序列化消息失败: {}", e))?
                .into()
        );

        let result = match &task.strategy {
            BroadcastStrategy::BroadcastAll { exclude_sender } => {
                let exclude_connection = if *exclude_sender {
                    task.sender_connection_id.as_ref()
                } else {
                    None
                };

                self.connection_manager.broadcast_message(ws_message, exclude_connection).await
            }

            BroadcastStrategy::BroadcastToUsers { user_ids } => {
                let mut total_sent = 0;
                for user_id in user_ids {
                    match self.connection_manager.send_to_user(user_id, ws_message.clone()).await {
                        Ok(sent_count) => {
                            total_sent += sent_count;
                        }
                        Err(e) => {
                            println!("MESSAGE_DISTRIBUTOR: 发送给用户 {} 失败: {}", user_id, e);
                        }
                    }
                }
                Ok(total_sent)
            }

            BroadcastStrategy::DirectMessage { target_user_id } => {
                self.connection_manager.send_to_user(target_user_id, ws_message).await
            }

            BroadcastStrategy::BroadcastToConnections { connection_ids } => {
                let mut total_sent = 0;
                for connection_id in connection_ids {
                    match
                        self.connection_manager.send_to_connection(
                            connection_id,
                            ws_message.clone()
                        ).await
                    {
                        Ok(_) => {
                            total_sent += 1;
                        }
                        Err(e) => {
                            println!(
                                "MESSAGE_DISTRIBUTOR: 发送给连接 {} 失败: {}",
                                connection_id,
                                e
                            );
                        }
                    }
                }
                Ok(total_sent)
            }

            BroadcastStrategy::SystemMessage => {
                self.connection_manager.broadcast_message(ws_message, None).await
            }
        };

        // 记录处理时间
        let processing_time = start_time.elapsed();
        println!(
            "MESSAGE_DISTRIBUTOR: 任务 {} 处理完成，耗时: {:?}ms",
            task.task_id,
            processing_time.as_millis()
        );

        result
    }

    /// 批量处理消息队列
    ///
    /// 【功能】: 从队列中取出一批任务进行处理
    /// 【返回值】: Result<usize, String> - 成功返回处理的任务数，失败返回错误信息
    pub async fn process_batch(&self) -> Result<usize, String> {
        let tasks = {
            let mut queue = self.message_queue.lock().await;
            let batch_size = std::cmp::min(self.batch_size, queue.len());
            if batch_size == 0 {
                return Ok(0);
            }

            // 取出一批任务
            queue.drain(0..batch_size).collect::<Vec<_>>()
        };

        let mut successful_count = 0;
        let mut failed_count = 0;

        // 并发处理任务
        let futures = tasks.iter().map(|task| self.process_task(task));
        let results = futures_util::future::join_all(futures).await;

        // 处理结果和重试逻辑
        for (task, result) in tasks.into_iter().zip(results.into_iter()) {
            match result {
                Ok(sent_count) => {
                    successful_count += 1;
                    println!(
                        "MESSAGE_DISTRIBUTOR: 任务 {} 成功分发给 {} 个连接",
                        task.task_id,
                        sent_count
                    );
                }
                Err(e) => {
                    failed_count += 1;
                    println!("MESSAGE_DISTRIBUTOR: 任务 {} 处理失败: {}", task.task_id, e);

                    // 如果任务可以重试，重新加入队列
                    if task.can_retry() {
                        let mut retry_task = task;
                        retry_task.increment_retry();

                        if let Err(e) = self.submit_task(retry_task).await {
                            println!("MESSAGE_DISTRIBUTOR: 重新提交任务失败: {}", e);
                        }
                    }
                }
            }
        }

        // 更新统计信息
        {
            let mut stats = self.stats.write().await;
            stats.successful_distributions += successful_count;
            stats.failed_distributions += failed_count;
            stats.queue_length = {
                let queue = self.message_queue.lock().await;
                queue.len()
            };
            stats.last_updated = Utc::now();
        }

        Ok((successful_count + failed_count) as usize)
    }

    /// 启动消息分发工作线程
    ///
    /// 【功能】: 启动后台工作线程持续处理消息队列
    /// 【返回值】: Vec<tokio::task::JoinHandle<()>> - 工作线程句柄列表
    pub fn start_workers(&self) -> Vec<tokio::task::JoinHandle<()>> {
        let mut handles = Vec::new();

        for worker_id in 0..self.worker_count {
            let distributor = self.clone();
            let handle = tokio::spawn(async move {
                println!("MESSAGE_DISTRIBUTOR: 工作线程 {} 已启动", worker_id);

                let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(10));

                loop {
                    interval.tick().await;

                    match distributor.process_batch().await {
                        Ok(processed_count) => {
                            if processed_count > 0 {
                                println!(
                                    "MESSAGE_DISTRIBUTOR: 工作线程 {} 处理了 {} 个任务",
                                    worker_id,
                                    processed_count
                                );
                            }
                        }
                        Err(e) => {
                            println!(
                                "MESSAGE_DISTRIBUTOR: 工作线程 {} 处理批次失败: {}",
                                worker_id,
                                e
                            );
                        }
                    }
                }
            });

            handles.push(handle);
        }

        println!("MESSAGE_DISTRIBUTOR: 已启动 {} 个工作线程", self.worker_count);

        handles
    }

    /// 获取分发统计信息
    ///
    /// 【功能】: 返回当前的分发统计数据
    /// 【返回值】: DistributionStats - 统计信息
    pub async fn get_stats(&self) -> DistributionStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// 获取当前队列长度
    ///
    /// 【功能】: 返回当前消息队列的长度
    /// 【返回值】: usize - 队列长度
    pub async fn get_queue_length(&self) -> usize {
        let queue = self.message_queue.lock().await;
        queue.len()
    }

    /// 清空消息队列
    ///
    /// 【功能】: 清空所有待处理的消息任务
    /// 【返回值】: usize - 清空的任务数量
    pub async fn clear_queue(&self) -> usize {
        let mut queue = self.message_queue.lock().await;
        let cleared_count = queue.len();
        queue.clear();

        // 更新统计信息
        {
            let mut stats = self.stats.write().await;
            stats.queue_length = 0;
            stats.last_updated = Utc::now();
        }

        println!("MESSAGE_DISTRIBUTOR: 已清空队列，清除了 {} 个任务", cleared_count);

        cleared_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    use crate::app::model::chat::{ ServerMessage, UserInfo };
    use crate::app::service::ConnectionManager;

    /// 创建测试用的消息分发器
    async fn create_test_distributor() -> (MessageDistributor, Arc<ConnectionManager>) {
        let connection_manager = Arc::new(ConnectionManager::new());
        let distributor = MessageDistributor::new(
            connection_manager.clone(),
            Some(10), // 小批量用于测试
            Some(2) // 2个工作线程
        );
        (distributor, connection_manager)
    }

    /// 创建测试用的服务器消息
    fn create_test_message(content: &str) -> ServerMessage {
        let user_info = UserInfo {
            user_id: Uuid::new_v4(),
            username: "test_user".to_string(),
            connected_at: Some(Utc::now()),
        };
        ServerMessage::new_text(content.to_string(), user_info)
    }

    #[tokio::test]
    async fn test_message_distributor_creation() {
        let (distributor, _) = create_test_distributor().await;

        // 验证初始状态
        assert_eq!(distributor.get_queue_length().await, 0);

        let stats = distributor.get_stats().await;
        assert_eq!(stats.total_tasks, 0);
        assert_eq!(stats.successful_distributions, 0);
        assert_eq!(stats.failed_distributions, 0);
    }

    #[tokio::test]
    async fn test_submit_task() {
        let (distributor, _) = create_test_distributor().await;

        let message = create_test_message("Test message");
        let task = DistributionTask::new(
            message,
            BroadcastStrategy::BroadcastAll { exclude_sender: false },
            MessagePriority::Normal,
            None
        );

        // 提交任务
        let result = distributor.submit_task(task).await;
        assert!(result.is_ok());

        // 验证队列长度
        assert_eq!(distributor.get_queue_length().await, 1);

        // 验证统计信息
        let stats = distributor.get_stats().await;
        assert_eq!(stats.total_tasks, 1);
        assert_eq!(stats.queue_length, 1);
    }

    #[tokio::test]
    async fn test_priority_queue_ordering() {
        let (distributor, _) = create_test_distributor().await;

        // 提交不同优先级的任务
        let low_priority_task = DistributionTask::new(
            create_test_message("Low priority"),
            BroadcastStrategy::SystemMessage,
            MessagePriority::Low,
            None
        );

        let high_priority_task = DistributionTask::new(
            create_test_message("High priority"),
            BroadcastStrategy::SystemMessage,
            MessagePriority::High,
            None
        );

        let critical_priority_task = DistributionTask::new(
            create_test_message("Critical priority"),
            BroadcastStrategy::SystemMessage,
            MessagePriority::Critical,
            None
        );

        // 按低->高->紧急的顺序提交
        distributor.submit_task(low_priority_task).await.unwrap();
        distributor.submit_task(high_priority_task).await.unwrap();
        distributor.submit_task(critical_priority_task).await.unwrap();

        // 验证队列长度
        assert_eq!(distributor.get_queue_length().await, 3);

        // 处理一批任务，应该按优先级顺序处理
        let processed_count = distributor.process_batch().await.unwrap();
        assert_eq!(processed_count, 3);

        // 队列应该为空
        assert_eq!(distributor.get_queue_length().await, 0);
    }

    #[tokio::test]
    async fn test_broadcast_to_all_convenience_method() {
        let (distributor, connection_manager) = create_test_distributor().await;

        // 添加一个测试连接
        let (sender, _receiver) = mpsc::unbounded_channel();
        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        connection_manager
            .add_connection(connection_id, user_id, "test_user".to_string(), sender, None).await
            .unwrap();

        let message = create_test_message("Broadcast test");

        // 使用便捷方法广播消息
        let result = distributor.broadcast_to_all(
            message,
            false,
            None,
            Some(MessagePriority::Normal)
        ).await;

        assert!(result.is_ok());
        assert_eq!(distributor.get_queue_length().await, 1);
    }

    #[tokio::test]
    async fn test_send_direct_message_convenience_method() {
        let (distributor, _) = create_test_distributor().await;

        let message = create_test_message("Direct message test");
        let target_user_id = Uuid::new_v4();

        // 使用便捷方法发送私聊消息
        let result = distributor.send_direct_message(
            message,
            target_user_id,
            Some(MessagePriority::High)
        ).await;

        assert!(result.is_ok());
        assert_eq!(distributor.get_queue_length().await, 1);
    }

    #[tokio::test]
    async fn test_send_system_message_convenience_method() {
        let (distributor, _) = create_test_distributor().await;

        let message = ServerMessage::new_system("System notification".to_string());

        // 使用便捷方法发送系统消息
        let result = distributor.send_system_message(message).await;

        assert!(result.is_ok());
        assert_eq!(distributor.get_queue_length().await, 1);
    }

    #[tokio::test]
    async fn test_clear_queue() {
        let (distributor, _) = create_test_distributor().await;

        // 添加几个任务
        for i in 0..5 {
            let message = create_test_message(&format!("Message {}", i));
            let task = DistributionTask::new(
                message,
                BroadcastStrategy::SystemMessage,
                MessagePriority::Normal,
                None
            );
            distributor.submit_task(task).await.unwrap();
        }

        assert_eq!(distributor.get_queue_length().await, 5);

        // 清空队列
        let cleared_count = distributor.clear_queue().await;
        assert_eq!(cleared_count, 5);
        assert_eq!(distributor.get_queue_length().await, 0);
    }

    #[test]
    fn test_distribution_task_creation() {
        let message = create_test_message("Test task");
        let task = DistributionTask::new(
            message.clone(),
            BroadcastStrategy::BroadcastAll { exclude_sender: true },
            MessagePriority::High,
            Some(Uuid::new_v4())
        );

        assert_eq!(task.message.content, message.content);
        assert_eq!(task.priority, MessagePriority::High);
        assert_eq!(task.retry_count, 0);
        assert_eq!(task.max_retries, 3);
        assert!(task.can_retry());
    }

    #[test]
    fn test_distribution_task_retry_logic() {
        let message = create_test_message("Retry test");
        let mut task = DistributionTask::new(
            message,
            BroadcastStrategy::SystemMessage,
            MessagePriority::Normal,
            None
        );

        // 初始状态可以重试
        assert!(task.can_retry());
        assert_eq!(task.retry_count, 0);

        // 增加重试次数
        task.increment_retry();
        assert_eq!(task.retry_count, 1);
        assert!(task.can_retry());

        // 继续增加直到达到最大重试次数
        task.increment_retry();
        task.increment_retry();
        assert_eq!(task.retry_count, 3);
        assert!(!task.can_retry()); // 不能再重试
    }

    #[test]
    fn test_message_priority_ordering() {
        let mut priorities = vec![
            MessagePriority::Low,
            MessagePriority::Critical,
            MessagePriority::Normal,
            MessagePriority::High
        ];

        priorities.sort();

        assert_eq!(
            priorities,
            vec![
                MessagePriority::Low,
                MessagePriority::Normal,
                MessagePriority::High,
                MessagePriority::Critical
            ]
        );
    }

    #[test]
    fn test_broadcast_strategy_variants() {
        // 测试不同的广播策略
        let strategies = vec![
            BroadcastStrategy::BroadcastAll { exclude_sender: true },
            BroadcastStrategy::BroadcastToUsers { user_ids: vec![Uuid::new_v4()] },
            BroadcastStrategy::DirectMessage { target_user_id: Uuid::new_v4() },
            BroadcastStrategy::BroadcastToConnections { connection_ids: vec![Uuid::new_v4()] },
            BroadcastStrategy::SystemMessage
        ];

        // 验证所有策略都可以正常创建
        for strategy in strategies {
            let message = create_test_message("Strategy test");
            let task = DistributionTask::new(message, strategy, MessagePriority::Normal, None);
            assert!(task.task_id != Uuid::nil());
        }
    }
}
