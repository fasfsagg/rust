// app/service/mod.rs
//
// 【服务模块声明文件】
// 这个文件声明并导出服务子模块，使它们在应用程序其他部分可见。
// 服务层是分层架构中的重要一层，包含所有业务逻辑，调用数据库层进行数据操作。

// /------------------------------------------------------------------------\
// |                          【模块功能图示】                            |
// |------------------------------------------------------------------------|
// |      crate::app::service (本模块)                                    |
// |          |                                                        |
// |          +-- pub mod task_service; (声明子模块)                   |
// |          |      |                                                 |
// |          |      +--> src/app/service/task_service.rs (子模块文件) |
// |          |             - create_task_svc                           |
// |          |             - get_all_tasks_svc                         |
// |          |             - get_task_by_id_svc                        |
// |          |             - update_task_svc                           |
// |          |             - delete_task_svc                           |
// |          |                                                        |
// |          +-- pub use task_service::*; (重新导出子模块公共项)        |
// |                 |                                                 |
// |                 +--> 使外部可以直接 use crate::app::service::...   |
// \------------------------------------------------------------------------/
//
// 文件路径: src/app/service/mod.rs
//
// 【模块核心职责】
// 这个 `mod.rs` 文件是 `src/app/service/` 目录的【入口和组织者】。
// 它的职责与 `model/mod.rs` 类似：
// 1. **声明子模块**: 定义此目录下存在哪些提供服务逻辑的子模块（例如 `task_service`）。
// 2. **重新导出**: 将子模块中的公共项（通常是服务函数）导出到 `service` 模块的命名空间下，方便其他层（主要是 Controller 层）调用。
//
// 【本文件具体作用】
// 1. 通过 `pub mod task_service;` 声明了 `task_service` 这个公共子模块，对应 `task_service.rs` 文件。
// 2. 通过 `pub use task_service::*;` 将 `task_service.rs` 中所有公共的函数（如 `create_task_svc`, `get_all_tasks_svc` 等）重新导出。
//    - 这使得 Controller 层可以直接通过 `crate::app::service::create_task_svc(...)` 来调用服务函数，而无需写成 `crate::app::service::task_service::create_task_svc(...)`。

// --- 声明子模块 ---
// `pub mod task_service;`
// 【作用】: 声明存在一个名为 `task_service` 的公共子模块。
// 【查找规则】: Rust 编译器会查找 `src/app/service/task_service.rs` 文件。
// 【可见性】: `pub` 使得 `task_service` 模块本身可以被外部访问。[[关键语法要素: pub, mod]]
pub mod task_service;

// --- 重新导出公共项 ---
// `pub use task_service::*;`
// 【作用】: 将 `task_service` 模块中所有 `pub` 的项（主要是服务函数）引入到当前的 `service` 模块作用域，并使它们也成为 `pub`。
// 【效果】: 简化 Controller 层及其他调用者的导入和调用路径。
// 【* 通配符】: 导出 `task_service` 模块内的所有公共项。[[关键语法要素: pub, use, * (glob)]]
pub use task_service::*;
