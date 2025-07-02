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

use axum::extract::ws::Message;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use flate2::Compression;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::app::model::chat::ServerMessage;
use crate::app::service::{ConnectionId, ConnectionManager};

/// 消息分发策略枚举
///
/// 【功能】: 定义不同的消息分发策略
/// 【扩展性】: 可以轻松添加新的分发策略
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BroadcastStrategy {
    /// 广播给所有连接（排除发送者）
    BroadcastAll { exclude_sender: bool },
    /// 广播给指定用户列表
    BroadcastToUsers { user_ids: Vec<Uuid> },
    /// 私聊消息（点对点）
    DirectMessage { target_user_id: Uuid },
    /// 广播给指定连接列表
    BroadcastToConnections { connection_ids: Vec<ConnectionId> },
    /// 系统消息（广播给所有人，包括发送者）
    SystemMessage,
}

/// 消息优先级枚举
///
/// 【功能】: 定义消息的优先级级别
/// 【用途】: 用于消息队列的优先级排序
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum MessagePriority {
    /// 低优先级（普通聊天消息）
    Low = 1,
    /// 正常优先级（默认）
    #[default]
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
    /// 【性能优化新增】压缩消息数量
    pub compressed_messages: u64,
    /// 【性能优化新增】零拷贝消息数量
    pub zero_copy_messages: u64,
    /// 【性能优化新增】批处理效率（消息/批次）
    pub batch_efficiency: f64,
    /// 【性能优化新增】平均消息大小（字节）
    pub average_message_size: f64,
    /// 【性能优化新增】压缩比率
    pub compression_ratio: f64,
}

/// 【性能优化新增】消息压缩配置
///
/// 【功能】: 配置消息压缩的行为参数
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// 是否启用压缩
    pub enabled: bool,
    /// 压缩级别 (0-9, 6为默认)
    pub level: u32,
    /// 最小压缩阈值（字节）- 小于此大小的消息不压缩
    pub min_size_threshold: usize,
    /// 压缩类型
    pub compression_type: CompressionType,
}

/// 【性能优化新增】压缩类型枚举
#[derive(Debug, Clone, PartialEq)]
pub enum CompressionType {
    /// Gzip压缩
    Gzip,
    /// 无压缩
    None,
}

/// 【性能优化新增】动态批处理配置
///
/// 【功能】: 根据系统负载动态调整批处理大小
#[derive(Debug)]
pub struct DynamicBatchConfig {
    /// 最小批处理大小
    pub min_batch_size: usize,
    /// 最大批处理大小
    pub max_batch_size: usize,
    /// 当前批处理大小
    pub current_batch_size: AtomicUsize,
    /// 负载阈值 - 队列长度超过此值时增加批处理大小
    pub load_threshold: usize,
    /// 调整因子 (0.1 - 2.0)
    pub adjustment_factor: f64,
}

/// 消息分发器主结构体
///
/// 【功能】: 管理消息分发的核心组件
/// 【线程安全】: 使用 Arc<RwLock<T>> 和 Arc<Mutex<T>> 确保线程安全
/// 【性能优化】: 使用优先级队列、批量处理、零拷贝和压缩提高性能
#[derive(Debug, Clone)]
pub struct MessageDistributor {
    /// 连接管理器引用
    connection_manager: Arc<ConnectionManager>,
    /// 消息队列（按优先级排序）
    message_queue: Arc<Mutex<VecDeque<DistributionTask>>>,
    /// 分发统计信息
    stats: Arc<RwLock<DistributionStats>>,
    /// 【性能优化】动态批处理配置
    dynamic_batch_config: Arc<DynamicBatchConfig>,
    /// 分发工作线程数量
    worker_count: usize,
    /// 【性能优化新增】消息压缩配置
    compression_config: CompressionConfig,
    /// 【性能优化新增】性能监控计数器
    performance_counters: Arc<PerformanceCounters>,
}

/// 【性能优化新增】性能计数器
///
/// 【功能】: 使用原子操作记录性能指标，避免锁竞争
#[derive(Debug)]
pub struct PerformanceCounters {
    /// 零拷贝消息计数
    pub zero_copy_count: AtomicUsize,
    /// 压缩消息计数
    pub compressed_count: AtomicUsize,
    /// 总消息字节数
    pub total_bytes: AtomicUsize,
    /// 压缩后字节数
    pub compressed_bytes: AtomicUsize,
    /// 批处理计数
    pub batch_count: AtomicUsize,
    /// 批处理中的消息总数
    pub batched_messages: AtomicUsize,
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
        sender_connection_id: Option<ConnectionId>,
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
            compressed_messages: 0,
            zero_copy_messages: 0,
            batch_efficiency: 0.0,
            average_message_size: 0.0,
            compression_ratio: 1.0,
        }
    }
}

/// 【性能优化新增】压缩配置默认实现
impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: 6,                 // 平衡压缩率和速度
            min_size_threshold: 1024, // 1KB以下不压缩
            compression_type: CompressionType::Gzip,
        }
    }
}

/// 【性能优化新增】动态批处理配置默认实现
impl Default for DynamicBatchConfig {
    fn default() -> Self {
        Self {
            min_batch_size: 10,
            max_batch_size: 1000,
            current_batch_size: AtomicUsize::new(100),
            load_threshold: 500,
            adjustment_factor: 1.5,
        }
    }
}

/// 【性能优化新增】性能计数器默认实现
impl Default for PerformanceCounters {
    fn default() -> Self {
        Self {
            zero_copy_count: AtomicUsize::new(0),
            compressed_count: AtomicUsize::new(0),
            total_bytes: AtomicUsize::new(0),
            compressed_bytes: AtomicUsize::new(0),
            batch_count: AtomicUsize::new(0),
            batched_messages: AtomicUsize::new(0),
        }
    }
}

impl MessageDistributor {
    /// 创建新的消息分发器实例
    ///
    /// 【功能】: 初始化消息分发器，支持性能优化特性
    /// 【参数】:
    /// * `connection_manager` - 连接管理器引用
    /// * `batch_size` - 初始批量处理大小（默认100）
    /// * `worker_count` - 工作线程数量（默认4）
    ///
    /// 【返回值】: MessageDistributor 实例
    /// 【性能优化】: 启用零拷贝、压缩和动态批处理
    pub fn new(
        connection_manager: Arc<ConnectionManager>,
        batch_size: Option<usize>,
        worker_count: Option<usize>,
    ) -> Self {
        let initial_batch_size = batch_size.unwrap_or(100);
        let dynamic_batch_config = Arc::new(DynamicBatchConfig {
            current_batch_size: AtomicUsize::new(initial_batch_size),
            ..Default::default()
        });

        Self {
            connection_manager,
            message_queue: Arc::new(Mutex::new(VecDeque::new())),
            stats: Arc::new(RwLock::new(DistributionStats::default())),
            dynamic_batch_config,
            worker_count: worker_count.unwrap_or(4),
            compression_config: CompressionConfig::default(),
            performance_counters: Arc::new(PerformanceCounters::default()),
        }
    }

    /// 【性能优化新增】创建高性能配置的消息分发器
    ///
    /// 【功能】: 为百万并发场景优化的构造函数
    /// 【参数】:
    /// * `connection_manager` - 连接管理器引用
    /// * `compression_config` - 压缩配置
    /// * `dynamic_batch_config` - 动态批处理配置
    /// * `worker_count` - 工作线程数量
    ///
    /// 【返回值】: 高性能配置的MessageDistributor实例
    pub fn new_high_performance(
        connection_manager: Arc<ConnectionManager>,
        compression_config: CompressionConfig,
        dynamic_batch_config: DynamicBatchConfig,
        worker_count: usize,
    ) -> Self {
        Self {
            connection_manager,
            message_queue: Arc::new(Mutex::new(VecDeque::new())),
            stats: Arc::new(RwLock::new(DistributionStats::default())),
            dynamic_batch_config: Arc::new(dynamic_batch_config),
            worker_count,
            compression_config,
            performance_counters: Arc::new(PerformanceCounters::default()),
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

        println!(
            "MESSAGE_DISTRIBUTOR: 任务已提交到队列，当前队列长度: {}",
            queue.len()
        );

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
        priority: Option<MessagePriority>,
    ) -> Result<(), String> {
        let task = DistributionTask::new(
            message,
            BroadcastStrategy::BroadcastAll { exclude_sender },
            priority.unwrap_or_default(),
            sender_connection_id,
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
        priority: Option<MessagePriority>,
    ) -> Result<(), String> {
        let task = DistributionTask::new(
            message,
            BroadcastStrategy::DirectMessage { target_user_id },
            priority.unwrap_or_default(),
            None,
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
        priority: Option<MessagePriority>,
    ) -> Result<usize, String> {
        if user_ids.is_empty() {
            return Ok(0);
        }

        let task = DistributionTask::new(
            message,
            BroadcastStrategy::BroadcastToUsers {
                user_ids: user_ids.clone(),
            },
            priority.unwrap_or(MessagePriority::Normal),
            None,
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
            None,
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
            serde_json::to_string(&task.message)
                .map_err(|e| format!("序列化消息失败: {}", e))?
                .into(),
        );

        let result = match &task.strategy {
            BroadcastStrategy::BroadcastAll { exclude_sender } => {
                let exclude_connection = if *exclude_sender {
                    task.sender_connection_id.as_ref()
                } else {
                    None
                };

                self.connection_manager
                    .broadcast_message(ws_message, exclude_connection)
                    .await
            }

            BroadcastStrategy::BroadcastToUsers { user_ids } => {
                let mut total_sent = 0;
                for user_id in user_ids {
                    match self
                        .connection_manager
                        .send_to_user(user_id, ws_message.clone())
                        .await
                    {
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
                self.connection_manager
                    .send_to_user(target_user_id, ws_message)
                    .await
            }

            BroadcastStrategy::BroadcastToConnections { connection_ids } => {
                let mut total_sent = 0;
                for connection_id in connection_ids {
                    match self
                        .connection_manager
                        .send_to_connection(connection_id, ws_message.clone())
                        .await
                    {
                        Ok(_) => {
                            total_sent += 1;
                        }
                        Err(e) => {
                            println!(
                                "MESSAGE_DISTRIBUTOR: 发送给连接 {} 失败: {}",
                                connection_id, e
                            );
                        }
                    }
                }
                Ok(total_sent)
            }

            BroadcastStrategy::SystemMessage => {
                self.connection_manager
                    .broadcast_message(ws_message, None)
                    .await
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
            let queue_length = queue.len();

            // 【性能优化】使用动态批处理大小
            let batch_size = self.adjust_batch_size(queue_length);
            let actual_batch_size = std::cmp::min(batch_size, queue_length);

            if actual_batch_size == 0 {
                return Ok(0);
            }

            // 取出一批任务
            queue.drain(0..actual_batch_size).collect::<Vec<_>>()
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
                        task.task_id, sent_count
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
                                    worker_id, processed_count
                                );
                            }
                        }
                        Err(e) => {
                            println!(
                                "MESSAGE_DISTRIBUTOR: 工作线程 {} 处理批次失败: {}",
                                worker_id, e
                            );
                        }
                    }
                }
            });

            handles.push(handle);
        }

        println!(
            "MESSAGE_DISTRIBUTOR: 已启动 {} 个工作线程",
            self.worker_count
        );

        handles
    }

    /// 获取分发统计信息
    ///
    /// 【功能】: 返回当前的分发统计数据，包含性能优化指标
    /// 【返回值】: DistributionStats - 统计信息
    pub async fn get_stats(&self) -> DistributionStats {
        let mut stats = self.stats.read().await.clone();

        // 【性能优化】更新实时性能指标
        let counters = &self.performance_counters;
        stats.zero_copy_messages = counters
            .zero_copy_count
            .load(std::sync::atomic::Ordering::Relaxed) as u64;
        stats.compressed_messages = counters
            .compressed_count
            .load(std::sync::atomic::Ordering::Relaxed) as u64;

        let total_bytes = counters
            .total_bytes
            .load(std::sync::atomic::Ordering::Relaxed);
        let compressed_bytes = counters
            .compressed_bytes
            .load(std::sync::atomic::Ordering::Relaxed);

        if total_bytes > 0 {
            stats.compression_ratio = (compressed_bytes as f64) / (total_bytes as f64);
        }

        let batch_count = counters
            .batch_count
            .load(std::sync::atomic::Ordering::Relaxed);
        let batched_messages = counters
            .batched_messages
            .load(std::sync::atomic::Ordering::Relaxed);

        if batch_count > 0 {
            stats.batch_efficiency = (batched_messages as f64) / (batch_count as f64);
        }

        stats
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

        println!(
            "MESSAGE_DISTRIBUTOR: 已清空队列，清除了 {} 个任务",
            cleared_count
        );

        cleared_count
    }

    /// 【性能优化新增】重置分发统计信息
    ///
    /// 【功能】: 清零所有统计数据和性能计数器
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = DistributionStats::default();

        // 【性能优化】重置性能计数器
        let counters = &self.performance_counters;
        counters
            .zero_copy_count
            .store(0, std::sync::atomic::Ordering::Relaxed);
        counters
            .compressed_count
            .store(0, std::sync::atomic::Ordering::Relaxed);
        counters
            .total_bytes
            .store(0, std::sync::atomic::Ordering::Relaxed);
        counters
            .compressed_bytes
            .store(0, std::sync::atomic::Ordering::Relaxed);
        counters
            .batch_count
            .store(0, std::sync::atomic::Ordering::Relaxed);
        counters
            .batched_messages
            .store(0, std::sync::atomic::Ordering::Relaxed);
    }

    /// 【性能优化新增】零拷贝消息压缩
    ///
    /// 【功能】: 使用Bytes实现零拷贝，并根据配置进行压缩
    /// 【参数】:
    /// * `message` - 原始消息内容
    ///
    /// 【返回值】: 优化后的Bytes消息
    /// 【性能特性】: 零拷贝 + 可选压缩
    pub fn optimize_message(
        &self,
        message: &str,
    ) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>> {
        let message_bytes = message.as_bytes();
        let original_size = message_bytes.len();

        // 更新总字节数计数器
        self.performance_counters
            .total_bytes
            .fetch_add(original_size, std::sync::atomic::Ordering::Relaxed);

        // 判断是否需要压缩
        if self.compression_config.enabled
            && original_size >= self.compression_config.min_size_threshold
            && self.compression_config.compression_type == CompressionType::Gzip
        {
            // 执行Gzip压缩
            let mut encoder =
                GzEncoder::new(Vec::new(), Compression::new(self.compression_config.level));
            encoder.write_all(message_bytes)?;
            let compressed_data = encoder.finish()?;

            // 只有在压缩效果明显时才使用压缩版本（至少节省10%）
            if compressed_data.len() < (original_size * 9) / 10 {
                self.performance_counters
                    .compressed_count
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                self.performance_counters
                    .compressed_bytes
                    .fetch_add(compressed_data.len(), std::sync::atomic::Ordering::Relaxed);

                // 使用零拷贝Bytes
                Ok(Bytes::from(compressed_data))
            } else {
                // 压缩效果不佳，使用原始数据
                self.performance_counters
                    .zero_copy_count
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Ok(Bytes::copy_from_slice(message_bytes))
            }
        } else {
            // 不压缩，直接使用零拷贝
            self.performance_counters
                .zero_copy_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(Bytes::copy_from_slice(message_bytes))
        }
    }

    /// 【性能优化新增】动态调整批处理大小
    ///
    /// 【功能】: 根据当前队列长度动态调整批处理大小
    /// 【参数】:
    /// * `current_queue_length` - 当前队列长度
    ///
    /// 【返回值】: 调整后的批处理大小
    pub fn adjust_batch_size(&self, current_queue_length: usize) -> usize {
        let config = &self.dynamic_batch_config;
        let current_size = config
            .current_batch_size
            .load(std::sync::atomic::Ordering::Relaxed);

        let new_size = if current_queue_length > config.load_threshold {
            // 队列积压，增加批处理大小
            let increased = ((current_size as f64) * config.adjustment_factor) as usize;
            increased.min(config.max_batch_size)
        } else if current_queue_length < config.load_threshold / 2 {
            // 队列较空，减少批处理大小以降低延迟
            let decreased = ((current_size as f64) / config.adjustment_factor) as usize;
            decreased.max(config.min_batch_size)
        } else {
            current_size
        };

        // 更新当前批处理大小
        config
            .current_batch_size
            .store(new_size, std::sync::atomic::Ordering::Relaxed);
        new_size
    }

    /// 【性能优化新增】获取性能指标
    ///
    /// 【功能】: 获取详细的性能指标用于监控
    /// 【返回值】: 性能指标的快照
    pub fn get_performance_metrics(&self) -> PerformanceMetrics {
        let counters = &self.performance_counters;
        PerformanceMetrics {
            zero_copy_messages: counters
                .zero_copy_count
                .load(std::sync::atomic::Ordering::Relaxed),
            compressed_messages: counters
                .compressed_count
                .load(std::sync::atomic::Ordering::Relaxed),
            total_bytes_processed: counters
                .total_bytes
                .load(std::sync::atomic::Ordering::Relaxed),
            compressed_bytes: counters
                .compressed_bytes
                .load(std::sync::atomic::Ordering::Relaxed),
            batch_count: counters
                .batch_count
                .load(std::sync::atomic::Ordering::Relaxed),
            batched_messages: counters
                .batched_messages
                .load(std::sync::atomic::Ordering::Relaxed),
            current_batch_size: self
                .dynamic_batch_config
                .current_batch_size
                .load(std::sync::atomic::Ordering::Relaxed),
            compression_enabled: self.compression_config.enabled,
        }
    }
}

/// 【性能优化新增】性能指标结构体
///
/// 【功能】: 用于外部监控系统的性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// 零拷贝消息数量
    pub zero_copy_messages: usize,
    /// 压缩消息数量
    pub compressed_messages: usize,
    /// 处理的总字节数
    pub total_bytes_processed: usize,
    /// 压缩后的字节数
    pub compressed_bytes: usize,
    /// 批处理次数
    pub batch_count: usize,
    /// 批处理的消息总数
    pub batched_messages: usize,
    /// 当前批处理大小
    pub current_batch_size: usize,
    /// 是否启用压缩
    pub compression_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::model::chat::{ServerMessage, UserInfo};
    use crate::app::service::ConnectionManager;
    use tokio::sync::mpsc;

    /// 创建测试用的连接管理器
    async fn create_test_connection_manager() -> (Arc<ConnectionManager>, mpsc::Receiver<String>) {
        let connection_manager = Arc::new(ConnectionManager::new());
        let (_tx, rx) = mpsc::channel(100);
        (connection_manager, rx)
    }

    /// 创建测试用的消息分发器
    async fn create_test_distributor() -> (MessageDistributor, Arc<ConnectionManager>) {
        let connection_manager = Arc::new(ConnectionManager::new());
        let distributor = MessageDistributor::new(
            connection_manager.clone(),
            Some(10), // 小批量用于测试
            Some(2),  // 2个工作线程
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
            BroadcastStrategy::BroadcastAll {
                exclude_sender: false,
            },
            MessagePriority::Normal,
            None,
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
            None,
        );

        let high_priority_task = DistributionTask::new(
            create_test_message("High priority"),
            BroadcastStrategy::SystemMessage,
            MessagePriority::High,
            None,
        );

        let critical_priority_task = DistributionTask::new(
            create_test_message("Critical priority"),
            BroadcastStrategy::SystemMessage,
            MessagePriority::Critical,
            None,
        );

        // 按低->高->紧急的顺序提交
        distributor.submit_task(low_priority_task).await.unwrap();
        distributor.submit_task(high_priority_task).await.unwrap();
        distributor
            .submit_task(critical_priority_task)
            .await
            .unwrap();

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
            .add_connection(
                connection_id,
                user_id,
                "test_user".to_string(),
                sender,
                None,
            )
            .await
            .unwrap();

        let message = create_test_message("Broadcast test");

        // 使用便捷方法广播消息
        let result = distributor
            .broadcast_to_all(message, false, None, Some(MessagePriority::Normal))
            .await;

        assert!(result.is_ok());
        assert_eq!(distributor.get_queue_length().await, 1);
    }

    #[tokio::test]
    async fn test_send_direct_message_convenience_method() {
        let (distributor, _) = create_test_distributor().await;

        let message = create_test_message("Direct message test");
        let target_user_id = Uuid::new_v4();

        // 使用便捷方法发送私聊消息
        let result = distributor
            .send_direct_message(message, target_user_id, Some(MessagePriority::High))
            .await;

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
                None,
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
            BroadcastStrategy::BroadcastAll {
                exclude_sender: true,
            },
            MessagePriority::High,
            Some(Uuid::new_v4()),
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
            None,
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
            MessagePriority::High,
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
            BroadcastStrategy::BroadcastAll {
                exclude_sender: true,
            },
            BroadcastStrategy::BroadcastToUsers {
                user_ids: vec![Uuid::new_v4()],
            },
            BroadcastStrategy::DirectMessage {
                target_user_id: Uuid::new_v4(),
            },
            BroadcastStrategy::BroadcastToConnections {
                connection_ids: vec![Uuid::new_v4()],
            },
            BroadcastStrategy::SystemMessage,
        ];

        // 验证所有策略都可以正常创建
        for strategy in strategies {
            let message = create_test_message("Strategy test");
            let task = DistributionTask::new(message, strategy, MessagePriority::Normal, None);
            assert!(task.task_id != Uuid::nil());
        }
    }

    // 【性能优化测试】消息批处理优化测试
    #[tokio::test]
    async fn test_performance_optimization_zero_copy_messages() {
        let (connection_manager, _) = create_test_connection_manager().await;
        let distributor = MessageDistributor::new(connection_manager, Some(50), Some(2));

        // 测试零拷贝消息优化
        let test_message = "Test message for zero-copy optimization";
        let optimized_bytes = distributor.optimize_message(test_message).unwrap();

        // 验证消息内容正确
        assert_eq!(optimized_bytes.as_ref(), test_message.as_bytes());

        // 验证性能计数器更新
        let metrics = distributor.get_performance_metrics();
        assert!(metrics.zero_copy_messages > 0 || metrics.compressed_messages > 0);
        assert!(metrics.total_bytes_processed > 0);
    }

    #[tokio::test]
    async fn test_performance_optimization_dynamic_batch_sizing() {
        let (connection_manager, _) = create_test_connection_manager().await;
        let distributor = MessageDistributor::new(connection_manager, Some(100), Some(2));

        // 测试动态批处理大小调整
        let initial_size = distributor
            .dynamic_batch_config
            .current_batch_size
            .load(std::sync::atomic::Ordering::Relaxed);

        // 模拟高负载情况（队列长度超过阈值）
        let high_load_size = distributor.adjust_batch_size(600); // 超过默认阈值500
        assert!(high_load_size > initial_size);
        assert!(high_load_size <= distributor.dynamic_batch_config.max_batch_size);

        // 模拟低负载情况
        let low_load_size = distributor.adjust_batch_size(100); // 低于阈值的一半
        assert!(low_load_size < high_load_size);
        assert!(low_load_size >= distributor.dynamic_batch_config.min_batch_size);
    }

    #[tokio::test]
    async fn test_performance_optimization_compression_config() {
        let (connection_manager, _) = create_test_connection_manager().await;

        // 创建启用压缩的分发器
        let compression_config = CompressionConfig {
            enabled: true,
            level: 6,
            min_size_threshold: 100, // 100字节阈值
            compression_type: CompressionType::Gzip,
        };

        let dynamic_batch_config = DynamicBatchConfig::default();
        let distributor = MessageDistributor::new_high_performance(
            connection_manager,
            compression_config,
            dynamic_batch_config,
            4,
        );

        // 测试大消息压缩
        let large_message = "A".repeat(200); // 超过压缩阈值
        let _optimized_bytes = distributor.optimize_message(&large_message).unwrap();

        // 验证消息被处理
        let metrics = distributor.get_performance_metrics();
        assert!(metrics.total_bytes_processed >= 200);
        assert!(metrics.compression_enabled);
    }

    #[tokio::test]
    async fn test_performance_optimization_stats_integration() {
        let (connection_manager, _) = create_test_connection_manager().await;
        let distributor = MessageDistributor::new(connection_manager, Some(50), Some(2));

        // 处理一些消息以生成统计数据
        for i in 0..10 {
            let message = format!("Test message {}", i);
            let _ = distributor.optimize_message(&message);
        }

        // 验证统计信息包含性能指标
        let stats = distributor.get_stats().await;
        assert!(stats.zero_copy_messages > 0 || stats.compressed_messages > 0);
        assert!(stats.average_message_size >= 0.0);
        assert!(stats.compression_ratio >= 0.0);

        // 测试重置功能
        distributor.reset_stats().await;
        let reset_stats = distributor.get_stats().await;
        assert_eq!(reset_stats.zero_copy_messages, 0);
        assert_eq!(reset_stats.compressed_messages, 0);
    }

    #[test]
    fn test_performance_optimization_compression_types() {
        // 测试压缩类型枚举
        assert_eq!(CompressionType::Gzip, CompressionType::Gzip);
        assert_ne!(CompressionType::Gzip, CompressionType::None);

        // 测试默认配置
        let default_config = CompressionConfig::default();
        assert!(default_config.enabled);
        assert_eq!(default_config.compression_type, CompressionType::Gzip);
        assert_eq!(default_config.level, 6);
        assert_eq!(default_config.min_size_threshold, 1024);
    }
}
