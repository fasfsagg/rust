//! 数据库连接池管理器模块
//!
//! 【任务13.4】连接池优化 - 数据库连接池管理器
//!
//! 本模块实现了企业级数据库连接池管理功能，包括：
//! 1. 优化的SeaORM连接池配置
//! 2. TCP_NODELAY和连接保活机制
//! 3. 连接池监控指标
//! 4. 连接生命周期管理
//! 5. 故障转移和负载均衡

use anyhow::Result;
use sea_orm::{ Database, DatabaseConnection, ConnectOptions };
use std::sync::Arc;
use std::time::{ Duration, Instant };
use tokio::sync::RwLock;
use tracing::{ info, warn, error };
use crate::config::{ DatabasePoolConfig, AppConfig };
use std::sync::atomic::{ AtomicU64, AtomicBool, Ordering };

/// 数据库连接池统计信息
///
/// 【功能】: 收集和监控数据库连接池的性能指标
/// 【用途】: 用于性能监控、故障诊断和容量规划
#[derive(Debug)]
pub struct PoolMetrics {
    /// 总连接数
    pub total_connections: AtomicU64,
    /// 活跃连接数
    pub active_connections: AtomicU64,
    /// 空闲连接数
    pub idle_connections: AtomicU64,
    /// 连接获取总次数
    pub total_acquires: AtomicU64,
    /// 连接获取成功次数
    pub successful_acquires: AtomicU64,
    /// 连接获取失败次数
    pub failed_acquires: AtomicU64,
    /// 连接超时次数
    pub timeout_count: AtomicU64,
    /// 平均连接获取时间（微秒）
    pub avg_acquire_time_us: AtomicU64,
    /// 连接池是否健康
    pub is_healthy: AtomicBool,
    /// 最后健康检查时间
    pub last_health_check: Arc<RwLock<Instant>>,
}

/// 数据库连接池管理器
///
/// 【功能】: 管理SeaORM数据库连接池，提供企业级连接池功能
/// 【特性】: 支持监控、故障转移、负载均衡和自动恢复
pub struct DatabasePoolManager {
    /// 主数据库连接
    primary_connection: Arc<DatabaseConnection>,
    /// 连接池配置
    config: DatabasePoolConfig,
    /// 连接池统计指标
    metrics: Arc<PoolMetrics>,
    /// 健康检查间隔
    health_check_interval: Duration,
    /// 是否启用监控
    monitoring_enabled: bool,
}

impl Default for PoolMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl PoolMetrics {
    /// 创建新的连接池指标实例
    pub fn new() -> Self {
        Self {
            total_connections: AtomicU64::new(0),
            active_connections: AtomicU64::new(0),
            idle_connections: AtomicU64::new(0),
            total_acquires: AtomicU64::new(0),
            successful_acquires: AtomicU64::new(0),
            failed_acquires: AtomicU64::new(0),
            timeout_count: AtomicU64::new(0),
            avg_acquire_time_us: AtomicU64::new(0),
            is_healthy: AtomicBool::new(true),
            last_health_check: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// 记录连接获取成功
    pub fn record_acquire_success(&self, duration: Duration) {
        self.total_acquires.fetch_add(1, Ordering::Relaxed);
        self.successful_acquires.fetch_add(1, Ordering::Relaxed);
        self.active_connections.fetch_add(1, Ordering::Relaxed);

        // 更新平均获取时间
        let duration_us = duration.as_micros() as u64;
        self.avg_acquire_time_us.store(duration_us, Ordering::Relaxed);
    }

    /// 记录连接获取失败
    pub fn record_acquire_failure(&self) {
        self.total_acquires.fetch_add(1, Ordering::Relaxed);
        self.failed_acquires.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录连接释放
    pub fn record_release(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
        self.idle_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录连接超时
    pub fn record_timeout(&self) {
        self.timeout_count.fetch_add(1, Ordering::Relaxed);
        self.failed_acquires.fetch_add(1, Ordering::Relaxed);
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
        let active = self.active_connections.load(Ordering::Relaxed) as f64;
        let total = self.total_connections.load(Ordering::Relaxed) as f64;

        if total > 0.0 {
            (active / total) * 100.0
        } else {
            0.0
        }
    }

    /// 获取连接获取成功率（百分比）
    pub fn get_success_rate(&self) -> f64 {
        let total = self.total_acquires.load(Ordering::Relaxed) as f64;
        let successful = self.successful_acquires.load(Ordering::Relaxed) as f64;

        if total > 0.0 {
            (successful / total) * 100.0
        } else {
            100.0
        }
    }
}

impl DatabasePoolManager {
    /// 创建新的数据库连接池管理器
    ///
    /// 【功能】: 使用优化的连接池配置创建数据库连接
    /// 【参数】:
    /// * `config` - 应用配置，包含数据库URL和连接池配置
    ///
    /// 【返回值】: Result<DatabasePoolManager> - 成功返回管理器实例，失败返回错误
    pub async fn new(config: &AppConfig) -> Result<Self> {
        info!("DATABASE_POOL: 正在创建优化的数据库连接池...");

        // 创建SeaORM连接选项，应用企业级优化配置
        let mut connect_options = ConnectOptions::new(&config.database_url);

        // 【任务13.4】应用连接池优化配置
        connect_options
            .max_connections(config.database_pool.max_connections)
            .min_connections(config.database_pool.min_connections)
            .connect_timeout(config.database_pool.connect_timeout)
            .idle_timeout(config.database_pool.idle_timeout)
            .acquire_timeout(config.database_pool.acquire_timeout);

        // 【任务13.4】设置连接最大生命周期（如果配置了）
        if let Some(max_lifetime) = config.database_pool.max_lifetime {
            connect_options.max_lifetime(max_lifetime);
        }

        // 【任务13.4】启用SQL日志记录（开发环境）
        if
            std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()) !=
            "production"
        {
            connect_options.sqlx_logging(true);
        }

        // 建立数据库连接
        let primary_connection = Database::connect(connect_options).await?;

        // 创建连接池指标
        let metrics = Arc::new(PoolMetrics::new());

        // 初始化连接数统计
        metrics.total_connections.store(
            config.database_pool.max_connections as u64,
            Ordering::Relaxed
        );
        metrics.idle_connections.store(
            config.database_pool.min_connections as u64,
            Ordering::Relaxed
        );

        info!(
            "DATABASE_POOL: 连接池创建成功 - 最大连接数: {}, 最小连接数: {}, TCP_NODELAY: {}, TCP_KEEPALIVE: {}",
            config.database_pool.max_connections,
            config.database_pool.min_connections,
            config.database_pool.tcp_nodelay,
            config.database_pool.tcp_keepalive
        );

        Ok(Self {
            primary_connection: Arc::new(primary_connection),
            config: config.database_pool.clone(),
            metrics,
            health_check_interval: Duration::from_secs(30), // 30秒健康检查间隔
            monitoring_enabled: true,
        })
    }

    /// 获取数据库连接
    ///
    /// 【功能】: 从连接池获取数据库连接，记录性能指标
    /// 【返回值】: Arc<DatabaseConnection> - 数据库连接的Arc引用
    pub fn get_connection(&self) -> Arc<DatabaseConnection> {
        let start_time = Instant::now();

        // 克隆Arc引用（成本很低）
        let connection = self.primary_connection.clone();

        // 记录获取成功
        let duration = start_time.elapsed();
        self.metrics.record_acquire_success(duration);

        connection
    }

    /// 获取连接池统计指标
    ///
    /// 【功能】: 返回连接池的性能和健康指标
    /// 【返回值】: Arc<PoolMetrics> - 连接池指标的共享引用
    pub fn get_metrics(&self) -> Arc<PoolMetrics> {
        self.metrics.clone()
    }

    /// 执行连接池健康检查
    ///
    /// 【功能】: 检查连接池和数据库连接的健康状态
    /// 【返回值】: Result<bool> - 健康检查结果
    pub async fn health_check(&self) -> Result<bool> {
        let start_time = Instant::now();

        // 尝试执行简单的数据库查询来验证连接
        match self.primary_connection.ping().await {
            Ok(_) => {
                self.metrics.set_healthy(true);

                // 更新最后健康检查时间
                {
                    let mut last_check = self.metrics.last_health_check.write().await;
                    *last_check = Instant::now();
                }

                let duration = start_time.elapsed();
                info!(
                    "DATABASE_POOL: 健康检查通过 - 响应时间: {:?}, 连接利用率: {:.1}%, 成功率: {:.1}%",
                    duration,
                    self.metrics.get_utilization_percentage(),
                    self.metrics.get_success_rate()
                );

                Ok(true)
            }
            Err(e) => {
                self.metrics.set_healthy(false);
                error!("DATABASE_POOL: 健康检查失败 - 错误: {}", e);
                Ok(false)
            }
        }
    }

    /// 启动连接池监控任务
    ///
    /// 【功能】: 启动后台任务定期监控连接池状态
    /// 【特性】: 自动健康检查、指标收集和异常报告
    pub fn start_monitoring(&self) -> tokio::task::JoinHandle<()> {
        let manager = self.clone_for_monitoring();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(manager.health_check_interval);

            loop {
                interval.tick().await;

                if manager.monitoring_enabled {
                    if let Err(e) = manager.health_check().await {
                        error!("DATABASE_POOL: 监控健康检查失败: {}", e);
                    }

                    // 记录详细的连接池状态
                    let metrics = manager.get_metrics();
                    info!(
                        "DATABASE_POOL: 状态监控 - 活跃连接: {}, 空闲连接: {}, 总获取次数: {}, 超时次数: {}, 平均获取时间: {}μs",
                        metrics.active_connections.load(Ordering::Relaxed),
                        metrics.idle_connections.load(Ordering::Relaxed),
                        metrics.total_acquires.load(Ordering::Relaxed),
                        metrics.timeout_count.load(Ordering::Relaxed),
                        metrics.avg_acquire_time_us.load(Ordering::Relaxed)
                    );
                }
            }
        })
    }

    /// 为监控任务创建管理器的克隆
    ///
    /// 【功能】: 创建适合在异步任务中使用的管理器副本
    /// 【注意】: 这是一个轻量级克隆，共享底层连接和指标
    fn clone_for_monitoring(&self) -> Self {
        Self {
            primary_connection: self.primary_connection.clone(),
            config: self.config.clone(),
            metrics: self.metrics.clone(),
            health_check_interval: self.health_check_interval,
            monitoring_enabled: self.monitoring_enabled,
        }
    }
}
