// app/middleware/mod.rs
//
// 【中间件模块声明文件】
// 这个文件声明并导出中间件子模块，使它们在应用程序其他部分可见。
// 中间件是在请求处理流程中执行的额外逻辑，可以在请求到达处理器之前或响应发送之前执行。

// 声明logger子模块
// 这告诉Rust编译器，在当前目录下有一个名为logger.rs的文件，
// 它定义了一个名为logger的模块。
pub mod logger;

// 声明auth_middleware子模块
// JWT 认证中间件模块
pub mod auth_middleware;

// 声明error_handling子模块
// 全局错误处理中间件模块
pub mod error_handling;

// 声明error_recovery_middleware子模块
// 错误恢复机制中间件模块
pub mod error_recovery_middleware;

// 声明performance_monitor子模块
// 性能监控和指标收集中间件模块
pub mod performance_monitor;

// 声明security_audit子模块
// 安全审计日志中间件模块
pub mod security_audit;

// 重新导出logger模块中的所有公共项
// 这样，其他模块可以通过 `use crate::app::middleware::setup_logger` 直接访问函数，
// 而不需要 `use crate::app::middleware::logger::setup_logger`。
pub use logger::*;

// 重新导出auth_middleware模块中的所有公共项
pub use auth_middleware::*;

// 重新导出error_handling模块中的所有公共项
pub use error_handling::*;

// 重新导出error_recovery_middleware模块中的所有公共项
pub use error_recovery_middleware::*;

// 重新导出performance_monitor模块中的所有公共项
pub use performance_monitor::*;

// 重新导出security_audit模块中的所有公共项
pub use security_audit::*;
