//! 应用程序工具模块
//!
//! 这个模块包含了应用程序中常用的工具函数和辅助功能。
//! 主要目的是减少代码重复，提高代码的可维护性和一致性。

pub mod uuid_utils;
pub mod validation_utils;

// 重新导出常用的工具函数
pub use uuid_utils::*;
pub use validation_utils::*;
