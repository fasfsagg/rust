//! WebSocket连接池管理器模块
//!
//! 【任务13.4】连接池优化 - WebSocket连接池管理器
//!
//! 本模块实现了企业级WebSocket连接池管理功能，包括：
//! 1. WebSocket连接池和复用
//! 2. 连接负载均衡和故障转移
//! 3. 连接生命周期管理
//! 4. 性能监控和指标收集
//! 5. 自动重连和健康检查

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{ Duration, Instant };
use tokio::sync::{ RwLock, mpsc };
use tokio::time::interval;
use uuid::Uuid;
use axum::extract::ws::Message;
use tracing::{ info, warn, error, debug };
use crate::config::WebSocketPoolConfig;
use std::sync::atomic::{ AtomicU64, AtomicBool, AtomicUsize, Ordering };

/// WebSocket连接池统计指标
///
/// 【功能】: 收集和监控WebSocket连接池的性能指标
/// 【用途】: 用于性能监控、负载均衡决策和故障诊断
#[derive(Debug, Default)]
pub struct WebSocketPoolMetrics {
    /// 总连接数
    pub total_connections: AtomicU64,
    /// 活跃连接数
    pub active_connections: AtomicU64,
    /// 空闲连接数
    pub idle_connections: AtomicU64,
    /// 连接池利用率（百分比）
    pub pool_utilization: AtomicU64,
    /// 总消息发送数
    pub total_messages_sent: AtomicU64,
    /// 总消息接收数
    pub total_messages_received: AtomicU64,
    /// 连接失败次数
    pub connection_failures: AtomicU64,
    /// 重连成功次数
    pub reconnection_successes: AtomicU64,
    /// 重连失败次数
    pub reconnection_failures: AtomicU64,
    /// 平均消息延迟（微秒）
    pub avg_message_latency_us: AtomicU64,
    /// 连接池是否健康
    pub is_healthy: AtomicBool,
    /// 负载均衡器状态
    pub load_balancer_active: AtomicBool,
    /// 故障转移器状态
    pub failover_active: AtomicBool,
}

/// WebSocket连接信息
///
/// 【功能】: 存储单个WebSocket连接的详细信息和状态
#[derive(Debug)]
pub struct WebSocketConnectionInfo {
    /// 连接ID
    pub connection_id: Uuid,
    /// 用户ID
    pub user_id: Uuid,
    /// 连接建立时间
    pub connected_at: Instant,
    /// 最后活跃时间
    pub last_activity: Instant,
    /// 消息发送通道
    pub sender: mpsc::UnboundedSender<Message>,
    /// 连接状态
    pub is_active: AtomicBool,
    /// 重连次数
    pub reconnect_count: AtomicU64,
    /// 发送消息计数
    pub messages_sent: AtomicU64,
    /// 接收消息计数
    pub messages_received: AtomicU64,
    /// 连接延迟（微秒）
    pub latency_us: AtomicU64,
}

/// WebSocket连接池管理器
///
/// 【功能】: 管理WebSocket连接池，提供企业级连接管理功能
/// 【特性】: 支持连接复用、负载均衡、故障转移和自动恢复
pub struct WebSocketPoolManager {
    /// 连接池配置
    config: WebSocketPoolConfig,
    /// 活跃连接映射
    connections: Arc<RwLock<HashMap<Uuid, WebSocketConnectionInfo>>>,
    /// 用户连接映射（支持多设备）
    user_connections: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>,
    /// 连接池指标
    metrics: Arc<WebSocketPoolMetrics>,
    /// 负载均衡器
    load_balancer: Arc<RwLock<LoadBalancer>>,
    /// 故障转移管理器
    failover_manager: Arc<RwLock<FailoverManager>>,
    /// 监控任务句柄
    monitoring_handle: Option<tokio::task::JoinHandle<()>>,
}

/// 负载均衡器
///
/// 【功能】: 实现WebSocket连接的负载均衡算法
/// 【算法】: 支持轮询、最少连接数等负载均衡策略
#[derive(Debug)]
pub struct LoadBalancer {
    /// 当前轮询索引
    round_robin_index: AtomicUsize,
    /// 负载均衡策略
    strategy: LoadBalancingStrategy,
    /// 是否启用
    enabled: bool,
}

/// 负载均衡策略枚举
#[derive(Debug, Clone)]
pub enum LoadBalancingStrategy {
    /// 轮询策略
    RoundRobin,
    /// 最少连接数策略
    LeastConnections,
    /// 随机策略
    Random,
}

/// 故障转移管理器
///
/// 【功能】: 处理连接故障和自动恢复
/// 【特性】: 自动检测故障、触发重连、维护备用连接
#[derive(Debug)]
pub struct FailoverManager {
    /// 故障检测阈值
    failure_threshold: u32,
    /// 恢复检测间隔
    recovery_interval: Duration,
    /// 当前故障计数
    current_failures: AtomicU64,
    /// 是否处于故障转移状态
    in_failover: AtomicBool,
    /// 最后故障时间
    last_failure_time: Arc<RwLock<Option<Instant>>>,
}

impl WebSocketPoolMetrics {
    /// 创建新的WebSocket连接池指标实例
    pub fn new() -> Self {
        Self {
            total_connections: AtomicU64::new(0),
            active_connections: AtomicU64::new(0),
            idle_connections: AtomicU64::new(0),
            pool_utilization: AtomicU64::new(0),
            total_messages_sent: AtomicU64::new(0),
            total_messages_received: AtomicU64::new(0),
            connection_failures: AtomicU64::new(0),
            reconnection_successes: AtomicU64::new(0),
            reconnection_failures: AtomicU64::new(0),
            avg_message_latency_us: AtomicU64::new(0),
            is_healthy: AtomicBool::new(true),
            load_balancer_active: AtomicBool::new(false),
            failover_active: AtomicBool::new(false),
        }
    }

    /// 记录新连接
    pub fn record_new_connection(&self) {
        self.total_connections.fetch_add(1, Ordering::Relaxed);
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        self.update_utilization();
    }

    /// 记录连接断开
    pub fn record_disconnection(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
        self.update_utilization();
    }

    /// 记录消息发送
    pub fn record_message_sent(&self, latency: Duration) {
        self.total_messages_sent.fetch_add(1, Ordering::Relaxed);
        let latency_us = latency.as_micros() as u64;
        self.avg_message_latency_us.store(latency_us, Ordering::Relaxed);
    }

    /// 记录消息接收
    pub fn record_message_received(&self) {
        self.total_messages_received.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录连接失败
    pub fn record_connection_failure(&self) {
        self.connection_failures.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录重连成功
    pub fn record_reconnection_success(&self) {
        self.reconnection_successes.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录重连失败
    pub fn record_reconnection_failure(&self) {
        self.reconnection_failures.fetch_add(1, Ordering::Relaxed);
    }

    /// 更新连接池利用率
    fn update_utilization(&self) {
        let active = self.active_connections.load(Ordering::Relaxed);
        let total = self.total_connections.load(Ordering::Relaxed);

        let utilization = if total > 0 {
            (((active as f64) / (total as f64)) * 100.0) as u64
        } else {
            0
        };

        self.pool_utilization.store(utilization, Ordering::Relaxed);
    }

    /// 获取连接池健康状态
    pub fn is_healthy(&self) -> bool {
        self.is_healthy.load(Ordering::Relaxed)
    }

    /// 设置连接池健康状态
    pub fn set_healthy(&self, healthy: bool) {
        self.is_healthy.store(healthy, Ordering::Relaxed);
    }

    /// 获取连接池利用率（百分比）
    pub fn get_utilization_percentage(&self) -> f64 {
        self.pool_utilization.load(Ordering::Relaxed) as f64
    }
}

impl LoadBalancer {
    /// 创建新的负载均衡器
    pub fn new(strategy: LoadBalancingStrategy, enabled: bool) -> Self {
        Self {
            round_robin_index: AtomicUsize::new(0),
            strategy,
            enabled,
        }
    }

    /// 选择最佳连接
    ///
    /// 【功能】: 根据负载均衡策略选择最佳的连接
    /// 【参数】: 可用连接列表
    /// 【返回值】: 选中的连接索引（如果有）
    pub fn select_connection(&self, available_connections: &[Uuid]) -> Option<usize> {
        if !self.enabled || available_connections.is_empty() {
            return None;
        }

        match self.strategy {
            LoadBalancingStrategy::RoundRobin => {
                let index =
                    self.round_robin_index.fetch_add(1, Ordering::Relaxed) %
                    available_connections.len();
                Some(index)
            }
            LoadBalancingStrategy::LeastConnections => {
                // 简化实现：返回第一个连接
                // 在实际实现中，这里应该根据连接的负载情况选择
                Some(0)
            }
            LoadBalancingStrategy::Random => {
                // 使用简单的伪随机数生成器（基于时间戳）
                use std::time::{ SystemTime, UNIX_EPOCH };
                let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
                let index = (timestamp as usize) % available_connections.len();
                Some(index)
            }
        }
    }

    /// 启用负载均衡
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// 禁用负载均衡
    pub fn disable(&mut self) {
        self.enabled = false;
    }
}

impl FailoverManager {
    /// 创建新的故障转移管理器
    pub fn new(failure_threshold: u32, recovery_interval: Duration) -> Self {
        Self {
            failure_threshold,
            recovery_interval,
            current_failures: AtomicU64::new(0),
            in_failover: AtomicBool::new(false),
            last_failure_time: Arc::new(RwLock::new(None)),
        }
    }

    /// 记录连接失败
    pub async fn record_failure(&self) {
        let failures = self.current_failures.fetch_add(1, Ordering::Relaxed) + 1;

        {
            let mut last_failure = self.last_failure_time.write().await;
            *last_failure = Some(Instant::now());
        }

        if failures >= (self.failure_threshold as u64) {
            self.trigger_failover().await;
        }
    }

    /// 触发故障转移
    async fn trigger_failover(&self) {
        if !self.in_failover.load(Ordering::Relaxed) {
            self.in_failover.store(true, Ordering::Relaxed);
            warn!(
                "WEBSOCKET_POOL: 触发故障转移 - 失败次数: {}",
                self.current_failures.load(Ordering::Relaxed)
            );
        }
    }

    /// 检查是否可以恢复
    pub async fn can_recover(&self) -> bool {
        if !self.in_failover.load(Ordering::Relaxed) {
            return true;
        }

        let last_failure = self.last_failure_time.read().await;
        if let Some(failure_time) = *last_failure {
            failure_time.elapsed() >= self.recovery_interval
        } else {
            true
        }
    }

    /// 重置故障计数
    pub fn reset_failures(&self) {
        self.current_failures.store(0, Ordering::Relaxed);
        self.in_failover.store(false, Ordering::Relaxed);
    }

    /// 是否处于故障转移状态
    pub fn is_in_failover(&self) -> bool {
        self.in_failover.load(Ordering::Relaxed)
    }
}

impl WebSocketPoolManager {
    /// 创建新的WebSocket连接池管理器
    ///
    /// 【功能】: 初始化WebSocket连接池管理器，配置负载均衡和故障转移
    /// 【参数】: WebSocket连接池配置
    /// 【返回值】: WebSocketPoolManager实例
    pub fn new(config: WebSocketPoolConfig) -> Self {
        let metrics = Arc::new(WebSocketPoolMetrics::new());

        // 创建负载均衡器
        let load_balancer = Arc::new(
            RwLock::new(
                LoadBalancer::new(LoadBalancingStrategy::RoundRobin, config.enable_load_balancing)
            )
        );

        // 创建故障转移管理器
        let failover_manager = Arc::new(
            RwLock::new(
                FailoverManager::new(
                    5, // 故障阈值
                    Duration::from_secs(30) // 恢复间隔
                )
            )
        );

        // 更新指标状态
        metrics.load_balancer_active.store(config.enable_load_balancing, Ordering::Relaxed);
        metrics.failover_active.store(config.enable_failover, Ordering::Relaxed);

        info!(
            "WEBSOCKET_POOL: 连接池管理器创建成功 - 最大连接数: {}, 池大小: {}, 负载均衡: {}, 故障转移: {}",
            config.max_connections,
            config.pool_size,
            config.enable_load_balancing,
            config.enable_failover
        );

        Self {
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            user_connections: Arc::new(RwLock::new(HashMap::new())),
            metrics,
            load_balancer,
            failover_manager,
            monitoring_handle: None,
        }
    }

    /// 启动内存监控任务
    ///
    /// 【功能】: 启动后台任务监控连接池内存使用情况
    /// 【返回值】: Result<(), String> - 启动结果
    pub async fn start_memory_monitoring(&mut self) -> Result<(), String> {
        if self.monitoring_handle.is_some() {
            return Err("内存监控任务已在运行".to_string());
        }

        let connections = self.connections.clone();
        let metrics = self.metrics.clone();
        let config = self.config.clone();

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30)); // 每30秒检查一次

            loop {
                interval.tick().await;

                // 检查内存使用情况
                let memory_usage = Self::check_memory_usage(&connections, &metrics).await;

                // 如果内存使用过高，触发清理
                if memory_usage > 80.0 {
                    // 80%阈值
                    warn!("WEBSOCKET_POOL: 内存使用率过高: {:.1}%, 开始清理", memory_usage);
                    Self::cleanup_idle_connections(&connections, &metrics, &config).await;
                }

                debug!("WEBSOCKET_POOL: 内存监控 - 使用率: {:.1}%", memory_usage);
            }
        });

        self.monitoring_handle = Some(handle);
        info!("WEBSOCKET_POOL: 内存监控任务已启动");
        Ok(())
    }

    /// 停止内存监控任务
    ///
    /// 【功能】: 停止后台内存监控任务
    pub async fn stop_memory_monitoring(&mut self) {
        if let Some(handle) = self.monitoring_handle.take() {
            handle.abort();
            info!("WEBSOCKET_POOL: 内存监控任务已停止");
        }
    }

    /// 检查内存使用情况
    ///
    /// 【功能】: 检查连接池的内存使用率
    /// 【返回值】: f64 - 内存使用率百分比
    async fn check_memory_usage(
        connections: &Arc<RwLock<HashMap<Uuid, WebSocketConnectionInfo>>>,
        metrics: &Arc<WebSocketPoolMetrics>
    ) -> f64 {
        let connections_guard = connections.read().await;
        let connection_count = connections_guard.len();
        drop(connections_guard);

        // 估算内存使用（每个连接约1KB）
        let estimated_memory_kb = connection_count * 1;
        let max_memory_kb = 1024 * 1024; // 1GB限制

        ((estimated_memory_kb as f64) / (max_memory_kb as f64)) * 100.0
    }

    /// 清理空闲连接
    ///
    /// 【功能】: 清理长时间空闲的连接以释放内存
    async fn cleanup_idle_connections(
        connections: &Arc<RwLock<HashMap<Uuid, WebSocketConnectionInfo>>>,
        metrics: &Arc<WebSocketPoolMetrics>,
        config: &WebSocketPoolConfig
    ) {
        let mut connections_guard = connections.write().await;
        let mut to_remove = Vec::new();
        let now = Instant::now();

        for (connection_id, connection) in connections_guard.iter() {
            let idle_duration = now.duration_since(connection.last_activity);
            if idle_duration > config.connection_timeout * 2 {
                to_remove.push(*connection_id);
            }
        }

        let removed_count = to_remove.len();
        for connection_id in to_remove {
            connections_guard.remove(&connection_id);
            metrics.record_disconnection();
        }

        drop(connections_guard);

        if removed_count > 0 {
            info!("WEBSOCKET_POOL: 清理了 {} 个空闲连接", removed_count);
        }
    }

    /// 添加新的WebSocket连接
    ///
    /// 【功能】: 将新的WebSocket连接添加到连接池中
    /// 【参数】:
    /// * `connection_id` - 连接唯一标识符
    /// * `user_id` - 用户ID
    /// * `sender` - 消息发送通道
    ///
    /// 【返回值】: Result<(), String> - 操作结果
    pub async fn add_connection(
        &self,
        connection_id: Uuid,
        user_id: Uuid,
        sender: mpsc::UnboundedSender<Message>
    ) -> Result<(), String> {
        let now = Instant::now();

        // 检查连接数限制
        let current_connections = self.metrics.active_connections.load(Ordering::Relaxed);
        if current_connections >= (self.config.max_connections as u64) {
            self.metrics.record_connection_failure();
            return Err(
                format!(
                    "连接池已满，当前连接数: {}, 最大连接数: {}",
                    current_connections,
                    self.config.max_connections
                )
            );
        }

        // 创建连接信息
        let connection_info = WebSocketConnectionInfo {
            connection_id,
            user_id,
            connected_at: now,
            last_activity: now,
            sender,
            is_active: AtomicBool::new(true),
            reconnect_count: AtomicU64::new(0),
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            latency_us: AtomicU64::new(0),
        };

        // 添加到连接映射
        {
            let mut connections = self.connections.write().await;
            connections.insert(connection_id, connection_info);
        }

        // 更新用户连接映射
        {
            let mut user_connections = self.user_connections.write().await;
            user_connections.entry(user_id).or_insert_with(Vec::new).push(connection_id);
        }

        // 更新指标
        self.metrics.record_new_connection();

        info!(
            "WEBSOCKET_POOL: 新连接已添加 - 连接ID: {}, 用户ID: {}, 当前活跃连接数: {}",
            connection_id,
            user_id,
            self.metrics.active_connections.load(Ordering::Relaxed)
        );

        Ok(())
    }

    /// 移除WebSocket连接
    ///
    /// 【功能】: 从连接池中移除指定的连接
    /// 【参数】: `connection_id` - 要移除的连接ID
    /// 【返回值】: Option<WebSocketConnectionInfo> - 被移除的连接信息
    pub async fn remove_connection(&self, connection_id: &Uuid) -> Option<WebSocketConnectionInfo> {
        // 从连接映射中移除
        let removed_connection = {
            let mut connections = self.connections.write().await;
            connections.remove(connection_id)
        };

        if let Some(ref connection) = removed_connection {
            // 从用户连接映射中移除
            {
                let mut user_connections = self.user_connections.write().await;
                if let Some(user_conn_list) = user_connections.get_mut(&connection.user_id) {
                    user_conn_list.retain(|&id| id != *connection_id);
                    if user_conn_list.is_empty() {
                        user_connections.remove(&connection.user_id);
                    }
                }
            }

            // 更新指标
            self.metrics.record_disconnection();

            info!(
                "WEBSOCKET_POOL: 连接已移除 - 连接ID: {}, 用户ID: {}, 剩余活跃连接数: {}",
                connection_id,
                connection.user_id,
                self.metrics.active_connections.load(Ordering::Relaxed)
            );
        }

        removed_connection
    }

    /// 获取用户的所有连接
    ///
    /// 【功能】: 获取指定用户的所有活跃连接ID
    /// 【参数】: `user_id` - 用户ID
    /// 【返回值】: Vec<Uuid> - 用户的连接ID列表
    pub async fn get_user_connections(&self, user_id: &Uuid) -> Vec<Uuid> {
        let user_connections = self.user_connections.read().await;
        user_connections.get(user_id).cloned().unwrap_or_default()
    }

    /// 向指定连接发送消息
    ///
    /// 【功能】: 通过连接ID向特定连接发送消息
    /// 【参数】:
    /// * `connection_id` - 目标连接ID
    /// * `message` - 要发送的消息
    ///
    /// 【返回值】: Result<(), String> - 发送结果
    pub async fn send_to_connection(
        &self,
        connection_id: &Uuid,
        message: Message
    ) -> Result<(), String> {
        let start_time = Instant::now();

        let connections = self.connections.read().await;
        if let Some(connection) = connections.get(connection_id) {
            match connection.sender.send(message) {
                Ok(_) => {
                    // 更新连接统计
                    connection.messages_sent.fetch_add(1, Ordering::Relaxed);

                    // 记录发送指标
                    let latency = start_time.elapsed();
                    self.metrics.record_message_sent(latency);

                    Ok(())
                }
                Err(e) => {
                    error!("WEBSOCKET_POOL: 消息发送失败 - 连接ID: {}, 错误: {}", connection_id, e);

                    // 记录故障
                    let failover_manager = self.failover_manager.read().await;
                    drop(connections); // 释放读锁
                    drop(failover_manager);

                    let failover_manager = self.failover_manager.read().await;
                    failover_manager.record_failure().await;

                    Err(format!("消息发送失败: {}", e))
                }
            }
        } else {
            Err(format!("连接 {} 不存在", connection_id))
        }
    }

    /// 向用户的所有连接广播消息
    ///
    /// 【功能】: 向指定用户的所有活跃连接发送消息
    /// 【参数】:
    /// * `user_id` - 目标用户ID
    /// * `message` - 要广播的消息
    ///
    /// 【返回值】: Result<usize, String> - 成功发送的连接数
    pub async fn broadcast_to_user(
        &self,
        user_id: &Uuid,
        message: Message
    ) -> Result<usize, String> {
        let connection_ids = self.get_user_connections(user_id).await;
        let mut successful_sends = 0;
        let mut errors = Vec::new();

        for connection_id in connection_ids {
            match self.send_to_connection(&connection_id, message.clone()).await {
                Ok(_) => {
                    successful_sends += 1;
                }
                Err(e) => errors.push(format!("连接 {}: {}", connection_id, e)),
            }
        }

        if errors.is_empty() {
            Ok(successful_sends)
        } else {
            Err(format!("部分发送失败: {}", errors.join(", ")))
        }
    }

    /// 获取连接池统计指标
    ///
    /// 【功能】: 返回连接池的性能和健康指标
    /// 【返回值】: Arc<WebSocketPoolMetrics> - 连接池指标的共享引用
    pub fn get_metrics(&self) -> Arc<WebSocketPoolMetrics> {
        self.metrics.clone()
    }

    /// 执行连接池健康检查
    ///
    /// 【功能】: 检查连接池和所有连接的健康状态
    /// 【返回值】: Result<bool> - 健康检查结果
    pub async fn health_check(&self) -> Result<bool, String> {
        let start_time = Instant::now();
        let mut healthy_connections = 0;
        let mut total_connections = 0;

        // 检查所有连接的健康状态
        {
            let connections = self.connections.read().await;
            total_connections = connections.len();

            for (connection_id, connection) in connections.iter() {
                if connection.is_active.load(Ordering::Relaxed) {
                    // 检查连接是否超时
                    let inactive_duration = connection.last_activity.elapsed();
                    if inactive_duration < self.config.connection_timeout * 2 {
                        healthy_connections += 1;
                    } else {
                        warn!(
                            "WEBSOCKET_POOL: 连接超时 - 连接ID: {}, 非活跃时间: {:?}",
                            connection_id,
                            inactive_duration
                        );
                    }
                }
            }
        }

        // 计算健康率
        let health_rate = if total_connections > 0 {
            ((healthy_connections as f64) / (total_connections as f64)) * 100.0
        } else {
            100.0
        };

        let is_healthy = health_rate >= 80.0; // 80%以上连接健康才认为池健康
        self.metrics.set_healthy(is_healthy);

        let duration = start_time.elapsed();
        info!(
            "WEBSOCKET_POOL: 健康检查完成 - 健康连接: {}/{}, 健康率: {:.1}%, 检查耗时: {:?}",
            healthy_connections,
            total_connections,
            health_rate,
            duration
        );

        Ok(is_healthy)
    }

    /// 检测内存泄漏
    ///
    /// 【功能】: 检测连接池是否存在内存泄漏
    /// 【返回值】: Result<MemoryLeakReport, String> - 内存泄漏检测报告
    pub async fn detect_memory_leaks(&self) -> Result<MemoryLeakReport, String> {
        let start_time = Instant::now();

        // 获取当前连接统计
        let connections = self.connections.read().await;
        let total_connections = connections.len();
        let mut active_connections = 0;
        let mut stale_connections = 0;
        let mut zombie_connections = 0;

        let now = Instant::now();

        for connection in connections.values() {
            if connection.is_active.load(Ordering::Relaxed) {
                active_connections += 1;

                // 检查是否为僵尸连接（活跃但长时间无消息）
                let inactive_duration = now.duration_since(connection.last_activity);
                if inactive_duration > Duration::from_secs(3600) {
                    // 1小时无活动
                    zombie_connections += 1;
                }
            } else {
                // 检查是否为陈旧连接（非活跃但未清理）
                let inactive_duration = now.duration_since(connection.last_activity);
                if inactive_duration > self.config.connection_timeout {
                    stale_connections += 1;
                }
            }
        }

        drop(connections);

        // 计算内存使用估算
        let estimated_memory_mb = (total_connections * 1024) / 1024; // 每连接1KB
        let memory_efficiency = if total_connections > 0 {
            ((active_connections as f64) / (total_connections as f64)) * 100.0
        } else {
            100.0
        };

        let detection_duration = start_time.elapsed();

        let report = MemoryLeakReport {
            total_connections,
            active_connections,
            stale_connections,
            zombie_connections,
            estimated_memory_mb,
            memory_efficiency,
            has_potential_leak: stale_connections > 0 || zombie_connections > 0,
            detection_duration,
        };

        info!(
            "WEBSOCKET_POOL: 内存泄漏检测完成 - 总连接: {}, 活跃: {}, 陈旧: {}, 僵尸: {}, 内存效率: {:.1}%",
            total_connections,
            active_connections,
            stale_connections,
            zombie_connections,
            memory_efficiency
        );

        Ok(report)
    }

    /// 强制清理所有连接
    ///
    /// 【功能】: 强制清理所有连接，用于测试或紧急情况
    /// 【返回值】: usize - 清理的连接数
    pub async fn force_cleanup_all(&self) -> usize {
        let mut connections = self.connections.write().await;
        let mut user_connections = self.user_connections.write().await;

        let removed_count = connections.len();
        connections.clear();
        user_connections.clear();

        // 重置指标
        self.metrics.active_connections.store(0, Ordering::Relaxed);
        self.metrics.idle_connections.store(0, Ordering::Relaxed);
        self.metrics.update_utilization();

        warn!("WEBSOCKET_POOL: 强制清理了所有 {} 个连接", removed_count);
        removed_count
    }

    /// 获取连接池详细统计
    ///
    /// 【功能】: 获取连接池的详细统计信息
    /// 【返回值】: ConnectionPoolStats - 连接池统计信息
    pub async fn get_detailed_stats(&self) -> ConnectionPoolStats {
        let connections = self.connections.read().await;
        let user_connections = self.user_connections.read().await;

        let total_connections = connections.len();
        let total_users = user_connections.len();
        let mut active_connections = 0;
        let mut idle_connections = 0;
        let mut total_messages_sent = 0;
        let mut total_messages_received = 0;

        for connection in connections.values() {
            if connection.is_active.load(Ordering::Relaxed) {
                active_connections += 1;
            } else {
                idle_connections += 1;
            }

            total_messages_sent += connection.messages_sent.load(Ordering::Relaxed);
            total_messages_received += connection.messages_received.load(Ordering::Relaxed);
        }

        drop(connections);
        drop(user_connections);

        ConnectionPoolStats {
            total_connections,
            active_connections,
            idle_connections,
            total_users,
            total_messages_sent,
            total_messages_received,
            pool_utilization: self.metrics.get_utilization_percentage(),
            is_healthy: self.metrics.is_healthy(),
        }
    }
}

/// 内存泄漏检测报告
#[derive(Debug, Clone)]
pub struct MemoryLeakReport {
    /// 总连接数
    pub total_connections: usize,
    /// 活跃连接数
    pub active_connections: usize,
    /// 陈旧连接数（非活跃但未清理）
    pub stale_connections: usize,
    /// 僵尸连接数（活跃但长时间无消息）
    pub zombie_connections: usize,
    /// 估算内存使用（MB）
    pub estimated_memory_mb: usize,
    /// 内存效率（活跃连接占比）
    pub memory_efficiency: f64,
    /// 是否存在潜在内存泄漏
    pub has_potential_leak: bool,
    /// 检测耗时
    pub detection_duration: Duration,
}

/// 连接池详细统计
#[derive(Debug, Clone)]
pub struct ConnectionPoolStats {
    /// 总连接数
    pub total_connections: usize,
    /// 活跃连接数
    pub active_connections: usize,
    /// 空闲连接数
    pub idle_connections: usize,
    /// 总用户数
    pub total_users: usize,
    /// 总发送消息数
    pub total_messages_sent: u64,
    /// 总接收消息数
    pub total_messages_received: u64,
    /// 连接池利用率
    pub pool_utilization: f64,
    /// 连接池健康状态
    pub is_healthy: bool,
}
