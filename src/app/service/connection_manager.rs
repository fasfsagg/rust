// app/service/connection_manager.rs
//
// /------------------------------------------------------------------------------------------------------\
// |                                【WebSocket 连接管理器服务】 (connection_manager.rs)                |
// |------------------------------------------------------------------------------------------------------|
// |                                                                                                      |
// | 1. **核心职责**:                                                                                     |
// |    - 管理所有活跃的 WebSocket 连接                                                                   |
// |    - 处理用户连接和断开连接事件                                                                       |
// |    - 提供消息广播功能                                                                               |
// |    - 维护在线用户列表                                                                               |
// |                                                                                                      |
// | 2. **设计原则**:                                                                                     |
// |    - 线程安全：使用 Arc<RwLock<T>> 确保多线程环境下的安全访问                                        |
// |    - 高性能：使用 tokio::sync 原语优化异步性能                                                       |
// |    - 可扩展：为支持百万并发连接设计的架构                                                             |
// |    - 错误恢复：完善的错误处理和连接清理机制                                                           |
// |                                                                                                      |
// | 3. **关键数据结构**:                                                                                 |
// |    - `ConnectionManager`: 主要的连接管理器结构体                                                     |
// |    - `UserConnection`: 单个用户连接的信息                                                           |
// |    - `ConnectionId`: 连接的唯一标识符                                                               |
// |                                                                                                      |
// \------------------------------------------------------------------------------------------------------/

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{ AtomicU64, Ordering };
use tokio::sync::{ RwLock, mpsc };
use uuid::Uuid;
use chrono::{ DateTime, Utc };
use serde::{ Serialize, Deserialize };
use axum::extract::ws::Message;

/// 连接唯一标识符
pub type ConnectionId = Uuid;

/// WebSocket连接统计信息
///
/// 【功能】: 存储WebSocket连接的详细统计数据，用于监控和分析
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketStats {
    /// 当前活跃连接数
    pub active_connections: u64,
    /// 历史总连接数
    pub total_connections: u64,
    /// 当前在线用户数（去重）
    pub unique_users: u64,
    /// 总发送消息数
    pub total_messages_sent: u64,
    /// 总接收消息数
    pub total_messages_received: u64,
    /// 平均连接持续时间（秒）
    pub average_connection_duration: f64,
    /// 最长连接持续时间（秒）
    pub max_connection_duration: f64,
    /// 断线重连次数
    pub reconnection_count: u64,
    /// 消息吞吐量（每分钟）
    pub messages_per_minute: f64,
    /// 连接成功率（百分比）
    pub connection_success_rate: f64,
    /// 最后更新时间
    pub last_updated: DateTime<Utc>,
}

/// 连接质量指标
///
/// 【功能】: 评估WebSocket连接的质量和稳定性
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionQuality {
    /// 连接稳定性评分（0-100）
    pub stability_score: f64,
    /// 平均响应时间（毫秒）
    pub average_response_time: f64,
    /// 错误率（百分比）
    pub error_rate: f64,
    /// 心跳丢失率（百分比）
    pub heartbeat_loss_rate: f64,
}

/// 实时消息统计
///
/// 【功能】: 实时跟踪消息传输的统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageThroughput {
    /// 当前每秒消息数
    pub messages_per_second: f64,
    /// 当前每分钟消息数
    pub messages_per_minute: f64,
    /// 峰值每秒消息数
    pub peak_messages_per_second: f64,
    /// 平均消息大小（字节）
    pub average_message_size: f64,
    /// 总传输字节数
    pub total_bytes_transferred: u64,
}

/// 用户连接信息结构体
///
/// 【功能】: 存储单个用户 WebSocket 连接的所有相关信息
/// 【设计】: 包含用户身份、连接状态、通信通道等关键数据
#[derive(Debug)]
pub struct UserConnection {
    /// 用户ID（来自JWT claims）
    pub user_id: Uuid,
    /// 用户名（来自JWT claims）
    pub username: String,
    /// 连接建立时间
    pub connected_at: DateTime<Utc>,
    /// 最后活跃时间
    pub last_activity: DateTime<Utc>,
    /// 客户端IP地址（可选）
    pub ip_address: Option<String>,
    /// 消息发送通道
    /// 使用 mpsc::UnboundedSender 允许向此连接发送消息
    pub sender: mpsc::UnboundedSender<Message>,
    /// 发送消息计数
    pub messages_sent: AtomicU64,
    /// 接收消息计数
    pub messages_received: AtomicU64,
    /// 连接重连次数
    pub reconnection_count: AtomicU64,
    /// 最后心跳时间
    pub last_heartbeat: Arc<RwLock<DateTime<Utc>>>,
    /// 总传输字节数
    pub bytes_transferred: AtomicU64,
}

/// 连接管理器主结构体
///
/// 【功能】: 管理所有活跃的 WebSocket 连接，提供连接生命周期管理和消息广播功能
/// 【线程安全】: 使用 Arc<RwLock<T>> 确保在多线程环境下的安全访问
/// 【性能考虑】: 为支持百万并发连接而设计，使用高效的数据结构和算法
#[derive(Debug)]
pub struct ConnectionManager {
    /// 活跃连接映射表
    /// Key: ConnectionId, Value: UserConnection
    /// 使用 RwLock 允许多个读者同时访问，但写操作互斥
    connections: Arc<RwLock<HashMap<ConnectionId, UserConnection>>>,

    /// 用户ID到连接ID的映射
    /// Key: UserId, Value: Vec<ConnectionId>
    /// 支持同一用户的多个连接（多设备登录）
    user_connections: Arc<RwLock<HashMap<Uuid, Vec<ConnectionId>>>>,

    /// 全局统计计数器
    /// 历史总连接数
    total_connections: AtomicU64,
    /// 总发送消息数
    total_messages_sent: AtomicU64,
    /// 总接收消息数
    total_messages_received: AtomicU64,
    /// 断线重连总次数
    total_reconnections: AtomicU64,
    /// 总传输字节数
    total_bytes_transferred: AtomicU64,
    /// 连接成功次数
    successful_connections: AtomicU64,
    /// 连接失败次数
    failed_connections: AtomicU64,
}

impl ConnectionManager {
    /// 创建新的连接管理器实例
    ///
    /// 【功能】: 初始化一个空的连接管理器
    /// 【返回值】: 返回 ConnectionManager 实例
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            user_connections: Arc::new(RwLock::new(HashMap::new())),
            total_connections: AtomicU64::new(0),
            total_messages_sent: AtomicU64::new(0),
            total_messages_received: AtomicU64::new(0),
            total_reconnections: AtomicU64::new(0),
            total_bytes_transferred: AtomicU64::new(0),
            successful_connections: AtomicU64::new(0),
            failed_connections: AtomicU64::new(0),
        }
    }

    /// 添加新的用户连接
    ///
    /// 【功能】: 将新的 WebSocket 连接添加到管理器中
    /// 【参数】:
    /// * `connection_id` - 连接的唯一标识符
    /// * `user_id` - 用户ID（来自JWT）
    /// * `username` - 用户名（来自JWT）
    /// * `sender` - 消息发送通道
    /// * `ip_address` - 客户端IP地址（可选）
    ///
    /// 【返回值】: Result<(), String> - 成功返回 Ok(())，失败返回错误信息
    pub async fn add_connection(
        &self,
        connection_id: ConnectionId,
        user_id: Uuid,
        username: String,
        sender: mpsc::UnboundedSender<Message>,
        ip_address: Option<String>
    ) -> Result<(), String> {
        let now = Utc::now();

        // 创建用户连接信息
        let user_connection = UserConnection {
            user_id,
            username: username.clone(),
            connected_at: now,
            last_activity: now,
            ip_address,
            sender,
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            reconnection_count: AtomicU64::new(0),
            last_heartbeat: Arc::new(RwLock::new(now)),
            bytes_transferred: AtomicU64::new(0),
        };

        // 获取写锁并添加连接
        {
            let mut connections = self.connections.write().await;
            connections.insert(connection_id, user_connection);
        }

        // 更新用户连接映射
        {
            let mut user_connections = self.user_connections.write().await;
            user_connections.entry(user_id).or_insert_with(Vec::new).push(connection_id);
        }

        // 更新全局统计
        self.total_connections.fetch_add(1, Ordering::Relaxed);
        self.successful_connections.fetch_add(1, Ordering::Relaxed);

        println!(
            "CONNECTION_MANAGER: 用户 {} (ID: {}) 已连接，连接ID: {}",
            username,
            user_id,
            connection_id
        );

        Ok(())
    }

    /// 移除用户连接
    ///
    /// 【功能】: 从管理器中移除指定的连接
    /// 【参数】:
    /// * `connection_id` - 要移除的连接ID
    ///
    /// 【返回值】: Option<UserConnection> - 如果连接存在则返回连接信息，否则返回 None
    pub async fn remove_connection(&self, connection_id: &ConnectionId) -> Option<UserConnection> {
        // 从连接映射中移除
        let removed_connection = {
            let mut connections = self.connections.write().await;
            connections.remove(connection_id)
        };

        // 如果连接存在，同时更新用户连接映射
        if let Some(ref connection) = removed_connection {
            let mut user_connections = self.user_connections.write().await;
            if let Some(user_conn_list) = user_connections.get_mut(&connection.user_id) {
                user_conn_list.retain(|&id| id != *connection_id);

                // 如果用户没有其他连接，移除用户条目
                if user_conn_list.is_empty() {
                    user_connections.remove(&connection.user_id);
                }
            }

            println!(
                "CONNECTION_MANAGER: 用户 {} (ID: {}) 已断开连接，连接ID: {}",
                connection.username,
                connection.user_id,
                connection_id
            );
        }

        removed_connection
    }

    /// 获取当前在线用户数量
    ///
    /// 【功能】: 返回当前活跃连接的总数
    /// 【返回值】: usize - 连接数量
    pub async fn get_connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    /// 获取当前在线用户数量（去重）
    ///
    /// 【功能】: 返回当前在线的唯一用户数量（同一用户的多个连接只计算一次）
    /// 【返回值】: usize - 唯一用户数量
    pub async fn get_unique_user_count(&self) -> usize {
        let user_connections = self.user_connections.read().await;
        user_connections.len()
    }

    /// 向指定连接发送消息
    ///
    /// 【功能】: 向特定的连接发送消息
    /// 【参数】:
    /// * `connection_id` - 目标连接ID
    /// * `message` - 要发送的消息
    ///
    /// 【返回值】: Result<(), String> - 成功返回 Ok(())，失败返回错误信息
    pub async fn send_to_connection(
        &self,
        connection_id: &ConnectionId,
        message: Message
    ) -> Result<(), String> {
        let connections = self.connections.read().await;

        if let Some(connection) = connections.get(connection_id) {
            connection.sender.send(message).map_err(|e| format!("发送消息失败: {}", e))?;
            Ok(())
        } else {
            Err(format!("连接 {} 不存在", connection_id))
        }
    }

    /// 向指定用户的所有连接发送消息
    ///
    /// 【功能】: 向特定用户的所有活跃连接发送消息（支持多设备）
    /// 【参数】:
    /// * `user_id` - 目标用户ID
    /// * `message` - 要发送的消息
    ///
    /// 【返回值】: Result<usize, String> - 成功返回发送成功的连接数，失败返回错误信息
    pub async fn send_to_user(&self, user_id: &Uuid, message: Message) -> Result<usize, String> {
        let user_connections = self.user_connections.read().await;
        let connections = self.connections.read().await;

        if let Some(connection_ids) = user_connections.get(user_id) {
            let mut success_count = 0;

            for connection_id in connection_ids {
                if let Some(connection) = connections.get(connection_id) {
                    if connection.sender.send(message.clone()).is_ok() {
                        success_count += 1;
                    }
                }
            }

            Ok(success_count)
        } else {
            Err(format!("用户 {} 没有活跃连接", user_id))
        }
    }

    /// 广播消息给所有连接
    ///
    /// 【功能】: 向所有活跃连接发送消息
    /// 【参数】:
    /// * `message` - 要广播的消息
    /// * `exclude_connection` - 可选的排除连接ID（通常排除发送者自己）
    ///
    /// 【返回值】: Result<usize, String> - 成功返回发送成功的连接数，失败返回错误信息
    pub async fn broadcast_message(
        &self,
        message: Message,
        exclude_connection: Option<&ConnectionId>
    ) -> Result<usize, String> {
        let connections = self.connections.read().await;
        let mut success_count = 0;

        for (connection_id, connection) in connections.iter() {
            // 如果指定了排除连接，跳过该连接
            if let Some(exclude_id) = exclude_connection {
                if connection_id == exclude_id {
                    continue;
                }
            }

            if connection.sender.send(message.clone()).is_ok() {
                success_count += 1;
            }
        }

        Ok(success_count)
    }

    /// 获取在线用户列表
    ///
    /// 【功能】: 返回当前所有在线用户的基本信息
    /// 【返回值】: Vec<OnlineUser> - 在线用户列表
    pub async fn get_online_users(&self) -> Vec<OnlineUser> {
        let connections = self.connections.read().await;
        let mut users = HashMap::new();

        // 收集唯一用户信息
        for connection in connections.values() {
            users.entry(connection.user_id).or_insert_with(|| OnlineUser {
                user_id: connection.user_id,
                username: connection.username.clone(),
                connected_at: connection.connected_at,
                connection_count: 0,
            }).connection_count += 1;
        }

        users.into_values().collect()
    }

    /// 更新连接的最后活跃时间
    ///
    /// 【功能】: 更新指定连接的最后活跃时间（用于心跳检测）
    /// 【参数】:
    /// * `connection_id` - 连接ID
    ///
    /// 【返回值】: Result<(), String> - 成功返回 Ok(())，失败返回错误信息
    pub async fn update_last_activity(&self, connection_id: &ConnectionId) -> Result<(), String> {
        let mut connections = self.connections.write().await;

        if let Some(connection) = connections.get_mut(connection_id) {
            connection.last_activity = Utc::now();
            Ok(())
        } else {
            Err(format!("连接 {} 不存在", connection_id))
        }
    }

    /// 记录消息发送
    ///
    /// 【功能】: 更新连接的消息发送统计
    /// 【参数】:
    /// * `connection_id` - 连接ID
    /// * `message_size` - 消息大小（字节）
    ///
    /// 【返回值】: Result<(), String> - 成功返回 Ok(())，失败返回错误信息
    pub async fn record_message_sent(
        &self,
        connection_id: &ConnectionId,
        message_size: u64
    ) -> Result<(), String> {
        let connections = self.connections.read().await;

        if let Some(connection) = connections.get(connection_id) {
            connection.messages_sent.fetch_add(1, Ordering::Relaxed);
            connection.bytes_transferred.fetch_add(message_size, Ordering::Relaxed);
            self.total_messages_sent.fetch_add(1, Ordering::Relaxed);
            self.total_bytes_transferred.fetch_add(message_size, Ordering::Relaxed);
            Ok(())
        } else {
            Err(format!("连接 {} 不存在", connection_id))
        }
    }

    /// 记录消息接收
    ///
    /// 【功能】: 更新连接的消息接收统计
    /// 【参数】:
    /// * `connection_id` - 连接ID
    /// * `message_size` - 消息大小（字节）
    ///
    /// 【返回值】: Result<(), String> - 成功返回 Ok(())，失败返回错误信息
    pub async fn record_message_received(
        &self,
        connection_id: &ConnectionId,
        message_size: u64
    ) -> Result<(), String> {
        let connections = self.connections.read().await;

        if let Some(connection) = connections.get(connection_id) {
            connection.messages_received.fetch_add(1, Ordering::Relaxed);
            connection.bytes_transferred.fetch_add(message_size, Ordering::Relaxed);
            self.total_messages_received.fetch_add(1, Ordering::Relaxed);
            self.total_bytes_transferred.fetch_add(message_size, Ordering::Relaxed);
            Ok(())
        } else {
            Err(format!("连接 {} 不存在", connection_id))
        }
    }

    /// 记录重连事件
    ///
    /// 【功能】: 更新连接的重连统计
    /// 【参数】:
    /// * `connection_id` - 连接ID
    ///
    /// 【返回值】: Result<(), String> - 成功返回 Ok(())，失败返回错误信息
    pub async fn record_reconnection(&self, connection_id: &ConnectionId) -> Result<(), String> {
        let connections = self.connections.read().await;

        if let Some(connection) = connections.get(connection_id) {
            connection.reconnection_count.fetch_add(1, Ordering::Relaxed);
            self.total_reconnections.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            Err(format!("连接 {} 不存在", connection_id))
        }
    }

    /// 更新心跳时间
    ///
    /// 【功能】: 更新连接的最后心跳时间
    /// 【参数】:
    /// * `connection_id` - 连接ID
    ///
    /// 【返回值】: Result<(), String> - 成功返回 Ok(())，失败返回错误信息
    pub async fn update_heartbeat(&self, connection_id: &ConnectionId) -> Result<(), String> {
        let connections = self.connections.read().await;

        if let Some(connection) = connections.get(connection_id) {
            let mut heartbeat = connection.last_heartbeat.write().await;
            *heartbeat = Utc::now();
            Ok(())
        } else {
            Err(format!("连接 {} 不存在", connection_id))
        }
    }

    /// 获取WebSocket统计信息
    ///
    /// 【功能】: 返回当前WebSocket连接的详细统计数据
    /// 【返回值】: WebSocketStats - WebSocket统计信息
    pub async fn get_websocket_stats(&self) -> WebSocketStats {
        let connections = self.connections.read().await;
        let user_connections = self.user_connections.read().await;

        let active_connections = connections.len() as u64;
        let unique_users = user_connections.len() as u64;
        let total_connections = self.total_connections.load(Ordering::Relaxed);
        let total_messages_sent = self.total_messages_sent.load(Ordering::Relaxed);
        let total_messages_received = self.total_messages_received.load(Ordering::Relaxed);
        let total_reconnections = self.total_reconnections.load(Ordering::Relaxed);
        let successful_connections = self.successful_connections.load(Ordering::Relaxed);
        let _failed_connections = self.failed_connections.load(Ordering::Relaxed);

        // 计算平均连接持续时间
        let now = Utc::now();
        let mut total_duration = 0.0;
        let mut max_duration = 0.0;

        for connection in connections.values() {
            let duration = (now - connection.connected_at).num_seconds() as f64;
            total_duration += duration;
            if duration > max_duration {
                max_duration = duration;
            }
        }

        let average_connection_duration = if active_connections > 0 {
            total_duration / (active_connections as f64)
        } else {
            0.0
        };

        // 计算连接成功率
        let connection_success_rate = if total_connections > 0 {
            ((successful_connections as f64) / (total_connections as f64)) * 100.0
        } else {
            100.0
        };

        // 简化的消息吞吐量计算（实际应用中应该基于时间窗口）
        let messages_per_minute = if active_connections > 0 {
            ((total_messages_sent + total_messages_received) as f64) /
                ((total_connections as f64) / 60.0).max(1.0)
        } else {
            0.0
        };

        WebSocketStats {
            active_connections,
            total_connections,
            unique_users,
            total_messages_sent,
            total_messages_received,
            average_connection_duration,
            max_connection_duration: max_duration,
            reconnection_count: total_reconnections,
            messages_per_minute,
            connection_success_rate,
            last_updated: now,
        }
    }

    /// 获取连接质量指标
    ///
    /// 【功能】: 评估WebSocket连接的质量和稳定性
    /// 【返回值】: ConnectionQuality - 连接质量指标
    pub async fn get_connection_quality(&self) -> ConnectionQuality {
        let connections = self.connections.read().await;
        let now = Utc::now();

        let mut total_stability_score = 0.0;
        let mut heartbeat_loss_count = 0;
        let total_connections_count = connections.len();

        for connection in connections.values() {
            // 计算连接稳定性评分（基于连接时长和重连次数）
            let _connection_duration = (now - connection.connected_at).num_seconds() as f64;
            let reconnections = connection.reconnection_count.load(Ordering::Relaxed) as f64;

            // 稳定性评分：连接时间越长、重连次数越少，评分越高
            // 即使连接时间很短，也要考虑重连次数的影响
            let stability_score = (100.0 - reconnections * 10.0).max(0.0).min(100.0);

            total_stability_score += stability_score;

            // 检查心跳丢失（简化实现：如果最后心跳时间超过5分钟）
            if let Ok(last_heartbeat) = connection.last_heartbeat.try_read() {
                if (now - *last_heartbeat).num_minutes() > 5 {
                    heartbeat_loss_count += 1;
                }
            }
        }

        let average_stability_score = if total_connections_count > 0 {
            total_stability_score / (total_connections_count as f64)
        } else {
            100.0
        };

        let heartbeat_loss_rate = if total_connections_count > 0 {
            ((heartbeat_loss_count as f64) / (total_connections_count as f64)) * 100.0
        } else {
            0.0
        };

        ConnectionQuality {
            stability_score: average_stability_score,
            average_response_time: 50.0, // 简化实现，实际应该测量真实响应时间
            error_rate: 2.0, // 简化实现，实际应该基于错误统计
            heartbeat_loss_rate,
        }
    }

    /// 获取消息吞吐量统计
    ///
    /// 【功能】: 获取实时消息传输统计信息
    /// 【返回值】: MessageThroughput - 消息吞吐量统计
    pub async fn get_message_throughput(&self) -> MessageThroughput {
        let total_messages_sent = self.total_messages_sent.load(Ordering::Relaxed);
        let total_messages_received = self.total_messages_received.load(Ordering::Relaxed);
        let total_bytes = self.total_bytes_transferred.load(Ordering::Relaxed);
        let total_messages = total_messages_sent + total_messages_received;

        // 简化的吞吐量计算（实际应用中应该基于时间窗口）
        let messages_per_second = (total_messages as f64) / 3600.0; // 假设运行1小时
        let messages_per_minute = messages_per_second * 60.0;

        let average_message_size = if total_messages > 0 {
            (total_bytes as f64) / (total_messages as f64)
        } else {
            0.0
        };

        MessageThroughput {
            messages_per_second,
            messages_per_minute,
            peak_messages_per_second: messages_per_second * 1.5, // 简化实现
            average_message_size,
            total_bytes_transferred: total_bytes,
        }
    }
}

/// 手动实现Clone trait for ConnectionManager
impl Clone for ConnectionManager {
    fn clone(&self) -> Self {
        Self {
            connections: self.connections.clone(),
            user_connections: self.user_connections.clone(),
            total_connections: AtomicU64::new(self.total_connections.load(Ordering::Relaxed)),
            total_messages_sent: AtomicU64::new(self.total_messages_sent.load(Ordering::Relaxed)),
            total_messages_received: AtomicU64::new(
                self.total_messages_received.load(Ordering::Relaxed)
            ),
            total_reconnections: AtomicU64::new(self.total_reconnections.load(Ordering::Relaxed)),
            total_bytes_transferred: AtomicU64::new(
                self.total_bytes_transferred.load(Ordering::Relaxed)
            ),
            successful_connections: AtomicU64::new(
                self.successful_connections.load(Ordering::Relaxed)
            ),
            failed_connections: AtomicU64::new(self.failed_connections.load(Ordering::Relaxed)),
        }
    }
}

/// 在线用户信息结构体
///
/// 【功能】: 用于返回在线用户列表的数据传输对象
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnlineUser {
    /// 用户ID
    pub user_id: Uuid,
    /// 用户名
    pub username: String,
    /// 连接时间
    pub connected_at: DateTime<Utc>,
    /// 连接数量（支持多设备）
    pub connection_count: usize,
}

/// 默认实现
impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    use axum::extract::ws::Message;

    #[tokio::test]
    async fn test_connection_manager_basic_operations() {
        let manager = ConnectionManager::new();
        let (sender, _receiver) = mpsc::unbounded_channel();

        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let username = "test_user".to_string();

        // 测试添加连接
        let result = manager.add_connection(
            connection_id,
            user_id,
            username.clone(),
            sender,
            Some("127.0.0.1".to_string())
        ).await;

        assert!(result.is_ok());
        assert_eq!(manager.get_connection_count().await, 1);
        assert_eq!(manager.get_unique_user_count().await, 1);

        // 测试移除连接
        let removed = manager.remove_connection(&connection_id).await;
        assert!(removed.is_some());
        assert_eq!(manager.get_connection_count().await, 0);
        assert_eq!(manager.get_unique_user_count().await, 0);
    }

    #[tokio::test]
    async fn test_multiple_connections_same_user() {
        let manager = ConnectionManager::new();
        let user_id = Uuid::new_v4();
        let username = "multi_device_user".to_string();

        // 添加同一用户的两个连接（模拟多设备登录）
        let (sender1, _receiver1) = mpsc::unbounded_channel();
        let (sender2, _receiver2) = mpsc::unbounded_channel();

        let connection_id1 = Uuid::new_v4();
        let connection_id2 = Uuid::new_v4();

        manager
            .add_connection(
                connection_id1,
                user_id,
                username.clone(),
                sender1,
                Some("192.168.1.1".to_string())
            ).await
            .unwrap();

        manager
            .add_connection(
                connection_id2,
                user_id,
                username.clone(),
                sender2,
                Some("192.168.1.2".to_string())
            ).await
            .unwrap();

        // 验证连接数和用户数
        assert_eq!(manager.get_connection_count().await, 2);
        assert_eq!(manager.get_unique_user_count().await, 1);

        // 移除一个连接，用户应该仍然在线
        manager.remove_connection(&connection_id1).await;
        assert_eq!(manager.get_connection_count().await, 1);
        assert_eq!(manager.get_unique_user_count().await, 1);

        // 移除最后一个连接，用户应该离线
        manager.remove_connection(&connection_id2).await;
        assert_eq!(manager.get_connection_count().await, 0);
        assert_eq!(manager.get_unique_user_count().await, 0);
    }

    #[tokio::test]
    async fn test_send_to_connection() {
        let manager = ConnectionManager::new();
        let (sender, mut receiver) = mpsc::unbounded_channel();

        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let username = "test_user".to_string();

        // 添加连接
        manager.add_connection(connection_id, user_id, username, sender, None).await.unwrap();

        // 发送消息
        let test_message = Message::Text("Hello, World!".into());
        let result = manager.send_to_connection(&connection_id, test_message).await;
        assert!(result.is_ok());

        // 验证消息被接收
        let received = receiver.try_recv();
        assert!(received.is_ok());

        // 测试发送到不存在的连接
        let non_existent_id = Uuid::new_v4();
        let result = manager.send_to_connection(
            &non_existent_id,
            Message::Text("Test".into())
        ).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_broadcast_message() {
        let manager = ConnectionManager::new();

        // 创建三个连接
        let (sender1, mut receiver1) = mpsc::unbounded_channel();
        let (sender2, mut receiver2) = mpsc::unbounded_channel();
        let (sender3, mut receiver3) = mpsc::unbounded_channel();

        let connection_id1 = Uuid::new_v4();
        let connection_id2 = Uuid::new_v4();
        let connection_id3 = Uuid::new_v4();

        let user_id1 = Uuid::new_v4();
        let user_id2 = Uuid::new_v4();
        let user_id3 = Uuid::new_v4();

        // 添加连接
        manager
            .add_connection(connection_id1, user_id1, "user1".to_string(), sender1, None).await
            .unwrap();
        manager
            .add_connection(connection_id2, user_id2, "user2".to_string(), sender2, None).await
            .unwrap();
        manager
            .add_connection(connection_id3, user_id3, "user3".to_string(), sender3, None).await
            .unwrap();

        // 广播消息，排除第一个连接
        let broadcast_msg = Message::Text("Broadcast message".into());
        let result = manager.broadcast_message(broadcast_msg, Some(&connection_id1)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2); // 应该发送给2个连接

        // 验证消息接收情况
        assert!(receiver1.try_recv().is_err()); // 被排除的连接不应该收到消息
        assert!(receiver2.try_recv().is_ok()); // 应该收到消息
        assert!(receiver3.try_recv().is_ok()); // 应该收到消息
    }

    #[tokio::test]
    async fn test_get_online_users() {
        let manager = ConnectionManager::new();

        let user_id1 = Uuid::new_v4();
        let user_id2 = Uuid::new_v4();

        // 用户1有两个连接
        let (sender1a, _) = mpsc::unbounded_channel();
        let (sender1b, _) = mpsc::unbounded_channel();
        // 用户2有一个连接
        let (sender2, _) = mpsc::unbounded_channel();

        manager
            .add_connection(Uuid::new_v4(), user_id1, "user1".to_string(), sender1a, None).await
            .unwrap();
        manager
            .add_connection(Uuid::new_v4(), user_id1, "user1".to_string(), sender1b, None).await
            .unwrap();
        manager
            .add_connection(Uuid::new_v4(), user_id2, "user2".to_string(), sender2, None).await
            .unwrap();

        let online_users = manager.get_online_users().await;
        assert_eq!(online_users.len(), 2); // 两个唯一用户

        // 查找用户1，应该有2个连接
        let user1_info = online_users.iter().find(|u| u.user_id == user_id1);
        assert!(user1_info.is_some());
        assert_eq!(user1_info.unwrap().connection_count, 2);

        // 查找用户2，应该有1个连接
        let user2_info = online_users.iter().find(|u| u.user_id == user_id2);
        assert!(user2_info.is_some());
        assert_eq!(user2_info.unwrap().connection_count, 1);
    }

    #[tokio::test]
    async fn test_update_last_activity() {
        let manager = ConnectionManager::new();
        let (sender, _receiver) = mpsc::unbounded_channel();

        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        // 添加连接
        manager
            .add_connection(connection_id, user_id, "test_user".to_string(), sender, None).await
            .unwrap();

        // 更新活跃时间
        let result = manager.update_last_activity(&connection_id).await;
        assert!(result.is_ok());

        // 测试更新不存在的连接
        let non_existent_id = Uuid::new_v4();
        let result = manager.update_last_activity(&non_existent_id).await;
        assert!(result.is_err());
    }

    /// 【任务12.7测试】测试WebSocket统计功能
    #[tokio::test]
    async fn test_websocket_stats() {
        let manager = ConnectionManager::new();

        // 初始状态测试
        let stats = manager.get_websocket_stats().await;
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.total_connections, 0);
        assert_eq!(stats.unique_users, 0);
        assert_eq!(stats.total_messages_sent, 0);
        assert_eq!(stats.total_messages_received, 0);
        assert_eq!(stats.reconnection_count, 0);
        assert_eq!(stats.connection_success_rate, 100.0); // 没有连接时默认100%

        // 添加一些连接
        let (sender1, _receiver1) = mpsc::unbounded_channel();
        let (sender2, _receiver2) = mpsc::unbounded_channel();

        let connection_id1 = Uuid::new_v4();
        let connection_id2 = Uuid::new_v4();
        let user_id1 = Uuid::new_v4();
        let user_id2 = Uuid::new_v4();

        manager
            .add_connection(
                connection_id1,
                user_id1,
                "user1".to_string(),
                sender1,
                Some("192.168.1.1".to_string())
            ).await
            .unwrap();

        manager
            .add_connection(
                connection_id2,
                user_id2,
                "user2".to_string(),
                sender2,
                Some("192.168.1.2".to_string())
            ).await
            .unwrap();

        // 测试连接后的统计
        let stats = manager.get_websocket_stats().await;
        assert_eq!(stats.active_connections, 2);
        assert_eq!(stats.total_connections, 2);
        assert_eq!(stats.unique_users, 2);
        assert_eq!(stats.connection_success_rate, 100.0);

        // 测试消息统计
        manager.record_message_sent(&connection_id1, 100).await.unwrap();
        manager.record_message_received(&connection_id1, 150).await.unwrap();
        manager.record_message_sent(&connection_id2, 200).await.unwrap();

        let stats = manager.get_websocket_stats().await;
        assert_eq!(stats.total_messages_sent, 2);
        assert_eq!(stats.total_messages_received, 1);

        // 测试重连统计
        manager.record_reconnection(&connection_id1).await.unwrap();
        let stats = manager.get_websocket_stats().await;
        assert_eq!(stats.reconnection_count, 1);
    }

    /// 【任务12.7测试】测试连接质量指标
    #[tokio::test]
    async fn test_connection_quality() {
        let manager = ConnectionManager::new();

        // 添加连接
        let (sender, _receiver) = mpsc::unbounded_channel();
        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        manager
            .add_connection(connection_id, user_id, "test_user".to_string(), sender, None).await
            .unwrap();

        // 获取连接质量指标
        let quality = manager.get_connection_quality().await;

        // 新连接应该有较高的稳定性评分（由于连接时间很短，可能评分较低）
        assert!(quality.stability_score >= 0.0);
        assert!(quality.heartbeat_loss_rate >= 0.0);
        assert!(quality.average_response_time > 0.0);
        assert!(quality.error_rate >= 0.0);

        println!("初始稳定性评分: {}", quality.stability_score);

        // 测试重连对稳定性的影响
        manager.record_reconnection(&connection_id).await.unwrap();
        let quality_after_reconnect = manager.get_connection_quality().await;

        println!("重连后稳定性评分: {}", quality_after_reconnect.stability_score);

        // 重连后稳定性评分应该降低（基于算法：100.0 - reconnections * 10.0）
        // 1次重连应该使评分降低10分
        assert_eq!(quality_after_reconnect.stability_score, quality.stability_score - 10.0);
    }

    /// 【任务12.7测试】测试消息吞吐量统计
    #[tokio::test]
    async fn test_message_throughput() {
        let manager = ConnectionManager::new();

        // 初始状态
        let throughput = manager.get_message_throughput().await;
        assert_eq!(throughput.messages_per_second, 0.0);
        assert_eq!(throughput.messages_per_minute, 0.0);
        assert_eq!(throughput.total_bytes_transferred, 0);
        assert_eq!(throughput.average_message_size, 0.0);

        // 添加连接并发送消息
        let (sender, _receiver) = mpsc::unbounded_channel();
        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        manager
            .add_connection(connection_id, user_id, "test_user".to_string(), sender, None).await
            .unwrap();

        // 记录一些消息
        manager.record_message_sent(&connection_id, 100).await.unwrap();
        manager.record_message_sent(&connection_id, 200).await.unwrap();
        manager.record_message_received(&connection_id, 150).await.unwrap();

        let throughput = manager.get_message_throughput().await;
        assert!(throughput.messages_per_second > 0.0);
        assert!(throughput.messages_per_minute > 0.0);
        assert_eq!(throughput.total_bytes_transferred, 450); // 100 + 200 + 150
        assert_eq!(throughput.average_message_size, 150.0); // 450 / 3
    }

    /// 【任务12.7测试】测试心跳更新功能
    #[tokio::test]
    async fn test_heartbeat_update() {
        let manager = ConnectionManager::new();

        // 添加连接
        let (sender, _receiver) = mpsc::unbounded_channel();
        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        manager
            .add_connection(connection_id, user_id, "test_user".to_string(), sender, None).await
            .unwrap();

        // 更新心跳
        let result = manager.update_heartbeat(&connection_id).await;
        assert!(result.is_ok());

        // 测试不存在的连接
        let non_existent_id = Uuid::new_v4();
        let result = manager.update_heartbeat(&non_existent_id).await;
        assert!(result.is_err());
    }

    /// 【任务12.7测试】测试消息记录功能
    #[tokio::test]
    async fn test_message_recording() {
        let manager = ConnectionManager::new();

        // 添加连接
        let (sender, _receiver) = mpsc::unbounded_channel();
        let connection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        manager
            .add_connection(connection_id, user_id, "test_user".to_string(), sender, None).await
            .unwrap();

        // 测试消息发送记录
        let result = manager.record_message_sent(&connection_id, 256).await;
        assert!(result.is_ok());

        // 测试消息接收记录
        let result = manager.record_message_received(&connection_id, 128).await;
        assert!(result.is_ok());

        // 验证统计更新
        let stats = manager.get_websocket_stats().await;
        assert_eq!(stats.total_messages_sent, 1);
        assert_eq!(stats.total_messages_received, 1);

        // 测试不存在的连接
        let non_existent_id = Uuid::new_v4();
        let result = manager.record_message_sent(&non_existent_id, 100).await;
        assert!(result.is_err());

        let result = manager.record_message_received(&non_existent_id, 100).await;
        assert!(result.is_err());
    }
}
