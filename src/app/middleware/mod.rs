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

// 重新导出logger模块中的所有公共项
// 这样，其他模块可以通过 `use crate::app::middleware::setup_logger` 直接访问函数，
// 而不需要 `use crate::app::middleware::logger::setup_logger`。
pub use logger::*;

// 重新导出auth_middleware模块中的所有公共项
pub use auth_middleware::*;
