//! 应用程序工具模块
//!
//! 这个模块包含了应用程序中常用的工具函数和辅助功能。
//! 主要目的是减少代码重复，提高代码的可维护性和一致性。

pub mod uuid_utils;
pub mod validation_utils;
pub mod jwt_utils;
pub mod auth_service;
pub mod error_recovery;

// 重新导出常用的工具函数
pub use uuid_utils::*;
pub use validation_utils::*;
pub use jwt_utils::{ Claims, JwtError, JwtUtils };
pub use auth_service::{ AuthService, TokenExtractionMethod };
pub use error_recovery::{
    ErrorRecoveryManager,
    ErrorRecoveryConfig,
    RetryConfig,
    CircuitBreakerSettings,
    DegradationConfig,
    RecoveryStatus,
    RetryStats,
};

// 测试模块 - 包含在 error_recovery.rs 文件中
// 不需要单独的 mod 声明，因为测试在同一个文件中
