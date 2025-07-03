//! 应用程序工具模块
//!
//! 这个模块包含了应用程序中常用的工具函数和辅助功能。
//! 主要目的是减少代码重复，提高代码的可维护性和一致性。

pub mod auth_service;
pub mod database_pool_manager; // 【任务13.4新增】数据库连接池管理器
pub mod error_recovery;
pub mod jwt_utils;
pub mod memory_manager;
pub mod memory_manager_benchmark;
pub mod optimized_state;

pub mod uuid_utils;
pub mod validation_utils;
pub mod websocket_pool_manager; // 【任务13.4新增】WebSocket连接池管理器 // 【任务13.4新增】连接池管理器测试

// 重新导出常用的工具函数
pub use auth_service::{AuthService, TokenExtractionMethod};
pub use database_pool_manager::{DatabasePoolManager, PoolMetrics}; // 【任务13.4新增】
pub use error_recovery::{
    CircuitBreakerSettings, DegradationConfig, ErrorRecoveryConfig, ErrorRecoveryManager,
    RecoveryStatus, RetryConfig, RetryStats,
};
pub use jwt_utils::{Claims, JwtError, JwtUtils};
pub use uuid_utils::*;
pub use validation_utils::*;
pub use websocket_pool_manager::{
    FailoverManager, LoadBalancer, LoadBalancingStrategy, WebSocketConnectionInfo,
    WebSocketPoolManager, WebSocketPoolMetrics,
}; // 【任务13.4新增】

// 测试模块 - 包含在 error_recovery.rs 文件中
// 不需要单独的 mod 声明，因为测试在同一个文件中
