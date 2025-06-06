// app/middleware/mod.rs
//
// 【中间件模块声明文件】
// 这个文件声明并导出中间件子模块，使它们在应用程序其他部分可见。
// 中间件是在请求处理流程中执行的额外逻辑，可以在请求到达处理器之前或响应发送之前执行。

// 声明logger子模块
// 这告诉Rust编译器，在当前目录下有一个名为logger.rs的文件，
// 它定义了一个名为logger的模块。
pub mod logger;

// 重新导出logger模块中的所有公共项
// 这样，其他模块可以通过 `use crate::app::middleware::setup_logger` 直接访问函数，
// 而不需要 `use crate::app::middleware::logger::setup_logger`。
pub use logger::*;

// --- Auth Middleware ---
pub mod auth_middleware;
// Re-exporting Claims here makes it available via `crate::app::middleware::Claims`
// This is for convenience if someone expects all auth related logic to be findable via middleware path.
// However, Claims is fundamentally a model, so `crate::app::model::Claims` is its canonical path.
// We also re-export it from `crate::app::model::mod.rs` and `crate::app::mod.rs`.
// If `auth_middleware.rs` defined a specific middleware struct (e.g., `JwtAuthMiddlewareLayer`),
// that would be more typically re-exported here.
// For `FromRequestParts` extractors, they are typically just imported where used.
pub use auth_middleware::*; // This will make the FromRequestParts<AppState> for Claims active.
                            // If Claims struct itself was in auth_middleware.rs, it would be re-exported.
                            // Since Claims is in model::user, this mostly ensures the module is linked.