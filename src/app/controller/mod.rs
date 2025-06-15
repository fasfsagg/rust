// app/controller/mod.rs
//
// 【控制器模块声明文件】
// 这个文件声明并导出控制器子模块，使它们在应用程序其他部分可见。
// 控制器层是分层架构中直接处理HTTP请求和响应的层，它调用服务层完成业务操作。

// /---------------------------------------------------------------------------\
// |                             【模块功能图示】                              |
// |---------------------------------------------------------------------------|
// |      crate::app::controller (本模块)                                    |
// |          |                                                              |
// |          +-- pub mod task_controller; (声明子模块)                      |
// |          |      |                                                       |
// |          |      +--> src/app/controller/task_controller.rs (子模块文件) |
// |          |             - create_task_handler                            |
// |          |             - get_all_tasks_handler                          |
// |          |             - get_task_handler                               |
// |          |             - update_task_handler                            |
// |          |             - delete_task_handler                            |
// |          |             - ws_handler (WebSocket 处理函数)                |
// |          |                                                              |
// |          +-- pub use task_controller::*; (重新导出子模块公共项)           |
// |                 |                                                       |
// |                 +--> 使外部(主要是 routes.rs) 可以直接 `use` 处理函数  |
// \---------------------------------------------------------------------------/
//
// 文件路径: src/app/controller/mod.rs
//
// 【模块核心职责】
// 这个 `mod.rs` 文件作为 `src/app/controller/` 目录的【入口和组织者】。
// 它的主要职责是：
// 1. **声明子模块**: 定义此目录下包含哪些处理 HTTP 请求的控制器子模块（例如 `task_controller`）。
// 2. **重新导出**: 将子模块中的公共项（主要是 Axum 的【处理函数 Handler】）导出到 `controller` 模块的命名空间下，方便路由层 (`routes.rs`) 引用它们。
//
// 【本文件具体作用】
// 1. 通过 `pub mod task_controller;` 声明了 `task_controller` 这个公共子模块，对应 `task_controller.rs` 文件。
// 2. 通过 `pub use task_controller::*;` 将 `task_controller.rs` 中所有公共的处理函数（如 `create_task_handler`, `get_all_tasks_handler` 等）重新导出。
//    - 这使得 `routes.rs` 在定义路由时，可以直接使用 `controller::create_task_handler` 来指定处理函数，而无需写成 `controller::task_controller::create_task_handler`。

// --- 声明子模块 ---
// `pub mod task_controller;`
// 【作用】: 声明存在一个名为 `task_controller` 的公共子模块。
// 【查找规则】: Rust 编译器会查找 `src/app/controller/task_controller.rs` 文件。
// 【可见性】: `pub` 使得 `task_controller` 模块本身可以被外部访问。[[关键语法要素: pub, mod]]
pub mod task_controller;

// 声明认证控制器模块
pub mod auth_controller;

// --- 重新导出公共项 ---
// `pub use task_controller::*;`
// 【作用】: 将 `task_controller` 模块中所有 `pub` 的项（主要是 Handler 函数）引入到当前的 `controller` 模块作用域，并使它们也成为 `pub`。
// 【效果】: 简化路由层 (`routes.rs`) 定义路由时的处理函数路径。
// 【* 通配符】: 导出 `task_controller` 模块内的所有公共项。[[关键语法要素: pub, use, * (glob)]]
pub use task_controller::*;

// 重新导出认证控制器的公共项
pub use auth_controller::*;
