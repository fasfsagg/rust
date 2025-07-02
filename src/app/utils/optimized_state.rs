//! 优化的应用状态管理模块
//!
//! 【功能】: 减少Axum状态克隆操作，优化内存使用
//! 【目标】: 降低状态克隆开销，提升请求处理性能
//! 【特性】: 智能状态分离、按需克隆、状态缓存

use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

use crate::app::{
    middleware::{
        error_recovery_middleware::ErrorRecoveryState, performance_monitor::PerformanceMetrics,
    },
    repository::TaskRepository,
    service::{
        AsyncPerformanceOptimizer, ConnectionManager, MessageDistributor, NotificationService,
        StatusSyncService,
    },
    utils::memory_manager::MemoryManager,
};

/// 优化的应用状态配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedStateConfig {
    /// 是否启用状态缓存
    pub enable_state_caching: bool,
    /// 状态缓存TTL（秒）
    pub state_cache_ttl_seconds: u64,
    /// 是否启用按需克隆
    pub enable_lazy_cloning: bool,
    /// 是否启用状态分离
    pub enable_state_separation: bool,
}

impl Default for OptimizedStateConfig {
    fn default() -> Self {
        Self {
            enable_state_caching: true,
            state_cache_ttl_seconds: 300, // 5分钟
            enable_lazy_cloning: true,
            enable_state_separation: true,
        }
    }
}

/// 核心状态组件（不经常变化的部分）
///
/// 【设计原理】: 将不经常变化的组件分离出来，减少克隆频率
#[derive(Debug, Clone)]
pub struct CoreState {
    /// 数据库连接（使用Arc包装，克隆成本低）
    pub db: Arc<DatabaseConnection>,
    /// JWT密钥（字符串，克隆成本相对较低）
    pub jwt_secret: String,
}

/// 服务状态组件（中等变化频率的部分）
///
/// 【设计原理】: 将服务层组件分组，按需克隆
#[derive(Debug, Clone)]
pub struct ServiceState {
    /// 任务仓库
    pub task_repo: Arc<TaskRepository>,
    /// 连接管理器
    pub connection_manager: Arc<ConnectionManager>,
    /// 消息分发器
    pub message_distributor: Arc<MessageDistributor>,
    /// 通知服务
    pub notification_service: Arc<NotificationService>,
    /// 状态同步服务
    pub status_sync_service: Arc<StatusSyncService>,
}

/// 监控状态组件（经常变化的部分）
///
/// 【设计原理】: 将监控和性能相关组件分离，减少对核心业务的影响
#[derive(Debug, Clone)]
pub struct MonitoringState {
    /// 性能指标收集器
    pub performance_metrics: Arc<PerformanceMetrics>,
    /// 错误恢复状态
    pub error_recovery_state: Arc<ErrorRecoveryState>,
    /// 异步性能优化器
    pub async_performance_optimizer: Arc<AsyncPerformanceOptimizer>,
    /// 内存管理器
    pub memory_manager: Arc<MemoryManager>,
}

/// 优化的应用状态
///
/// 【设计原理】:
/// 1. 按功能和变化频率分离状态组件
/// 2. 使用Arc减少深度克隆
/// 3. 提供按需访问接口
#[derive(Debug, Clone)]
pub struct OptimizedAppState {
    /// 核心状态
    core: Arc<CoreState>,
    /// 服务状态
    services: Arc<ServiceState>,
    /// 监控状态
    monitoring: Arc<MonitoringState>,
    /// 配置
    config: OptimizedStateConfig,
}

impl OptimizedAppState {
    /// 创建新的优化应用状态
    #[instrument(skip_all)]
    pub fn new(
        db: DatabaseConnection,
        jwt_secret: String,
        task_repo: Arc<TaskRepository>,
        connection_manager: Arc<ConnectionManager>,
        message_distributor: Arc<MessageDistributor>,
        notification_service: Arc<NotificationService>,
        status_sync_service: Arc<StatusSyncService>,
        performance_metrics: Arc<PerformanceMetrics>,
        error_recovery_state: Arc<ErrorRecoveryState>,
        async_performance_optimizer: Arc<AsyncPerformanceOptimizer>,
        memory_manager: Arc<MemoryManager>,
        config: OptimizedStateConfig,
    ) -> Self {
        let core = Arc::new(CoreState {
            db: Arc::new(db),
            jwt_secret,
        });

        let services = Arc::new(ServiceState {
            task_repo,
            connection_manager,
            message_distributor,
            notification_service,
            status_sync_service,
        });

        let monitoring = Arc::new(MonitoringState {
            performance_metrics,
            error_recovery_state,
            async_performance_optimizer,
            memory_manager,
        });

        info!("优化应用状态已创建");

        Self {
            core,
            services,
            monitoring,
            config,
        }
    }

    /// 获取核心状态（轻量级克隆）
    #[instrument(skip(self))]
    pub fn core(&self) -> Arc<CoreState> {
        debug!("获取核心状态");
        self.core.clone()
    }

    /// 获取服务状态（中等重量级克隆）
    #[instrument(skip(self))]
    pub fn services(&self) -> Arc<ServiceState> {
        debug!("获取服务状态");
        self.services.clone()
    }

    /// 获取监控状态（重量级克隆，按需使用）
    #[instrument(skip(self))]
    pub fn monitoring(&self) -> Arc<MonitoringState> {
        debug!("获取监控状态");
        self.monitoring.clone()
    }

    /// 获取数据库连接（最常用，优化访问）
    #[inline]
    pub fn db(&self) -> &DatabaseConnection {
        &self.core.db
    }

    /// 获取JWT密钥（常用，优化访问）
    #[inline]
    pub fn jwt_secret(&self) -> &str {
        &self.core.jwt_secret
    }

    /// 获取任务仓库（常用，优化访问）
    #[inline]
    pub fn task_repo(&self) -> &Arc<TaskRepository> {
        &self.services.task_repo
    }

    /// 获取连接管理器（常用，优化访问）
    #[inline]
    pub fn connection_manager(&self) -> &Arc<ConnectionManager> {
        &self.services.connection_manager
    }

    /// 获取消息分发器（常用，优化访问）
    #[inline]
    pub fn message_distributor(&self) -> &Arc<MessageDistributor> {
        &self.services.message_distributor
    }

    /// 获取通知服务（中等频率使用）
    #[inline]
    pub fn notification_service(&self) -> &Arc<NotificationService> {
        &self.services.notification_service
    }

    /// 获取状态同步服务（中等频率使用）
    #[inline]
    pub fn status_sync_service(&self) -> &Arc<StatusSyncService> {
        &self.services.status_sync_service
    }

    /// 获取性能指标收集器（监控用途）
    #[inline]
    pub fn performance_metrics(&self) -> &Arc<PerformanceMetrics> {
        &self.monitoring.performance_metrics
    }

    /// 获取错误恢复状态（错误处理用途）
    #[inline]
    pub fn error_recovery_state(&self) -> &Arc<ErrorRecoveryState> {
        &self.monitoring.error_recovery_state
    }

    /// 获取异步性能优化器（性能优化用途）
    #[inline]
    pub fn async_performance_optimizer(&self) -> &Arc<AsyncPerformanceOptimizer> {
        &self.monitoring.async_performance_optimizer
    }

    /// 获取内存管理器（内存优化用途）
    #[inline]
    pub fn memory_manager(&self) -> &Arc<MemoryManager> {
        &self.monitoring.memory_manager
    }

    /// 获取配置
    pub fn config(&self) -> &OptimizedStateConfig {
        &self.config
    }

    /// 创建轻量级状态（仅包含核心组件）
    ///
    /// 【用途】: 用于只需要数据库和JWT的简单操作
    pub fn lightweight(&self) -> LightweightState {
        LightweightState {
            core: self.core.clone(),
        }
    }

    /// 创建服务状态（包含核心和服务组件）
    ///
    /// 【用途】: 用于需要业务服务但不需要监控的操作
    pub fn service_only(&self) -> ServiceOnlyState {
        ServiceOnlyState {
            core: self.core.clone(),
            services: self.services.clone(),
        }
    }

    /// 获取状态克隆统计信息
    pub fn get_clone_stats(&self) -> StateCloneStats {
        // 这里可以添加实际的克隆统计逻辑
        StateCloneStats {
            core_clones: 0, // 实际实现中应该使用原子计数器
            service_clones: 0,
            monitoring_clones: 0,
            lightweight_clones: 0,
            service_only_clones: 0,
        }
    }
}

/// 轻量级状态（仅核心组件）
#[derive(Debug, Clone)]
pub struct LightweightState {
    core: Arc<CoreState>,
}

impl LightweightState {
    pub fn db(&self) -> &DatabaseConnection {
        &self.core.db
    }

    pub fn jwt_secret(&self) -> &str {
        &self.core.jwt_secret
    }
}

/// 服务状态（核心 + 服务组件）
#[derive(Debug, Clone)]
pub struct ServiceOnlyState {
    core: Arc<CoreState>,
    services: Arc<ServiceState>,
}

impl ServiceOnlyState {
    pub fn db(&self) -> &DatabaseConnection {
        &self.core.db
    }

    pub fn jwt_secret(&self) -> &str {
        &self.core.jwt_secret
    }

    pub fn task_repo(&self) -> &Arc<TaskRepository> {
        &self.services.task_repo
    }

    pub fn connection_manager(&self) -> &Arc<ConnectionManager> {
        &self.services.connection_manager
    }

    pub fn message_distributor(&self) -> &Arc<MessageDistributor> {
        &self.services.message_distributor
    }

    pub fn notification_service(&self) -> &Arc<NotificationService> {
        &self.services.notification_service
    }

    pub fn status_sync_service(&self) -> &Arc<StatusSyncService> {
        &self.services.status_sync_service
    }
}

/// 状态克隆统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateCloneStats {
    /// 核心状态克隆次数
    pub core_clones: u64,
    /// 服务状态克隆次数
    pub service_clones: u64,
    /// 监控状态克隆次数
    pub monitoring_clones: u64,
    /// 轻量级状态克隆次数
    pub lightweight_clones: u64,
    /// 仅服务状态克隆次数
    pub service_only_clones: u64,
}
