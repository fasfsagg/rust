// app/mod.rs
//
// 【应用核心模块声明文件】
// 这个文件是应用程序核心模块的"入口点"，它声明并导出所有子模块，
// 使它们在应用程序其他部分可见。
//
// 在这个分层架构中，app目录包含了应用程序的核心逻辑，包括：
// - 控制器层 (controller): 处理HTTP请求和响应
// - 服务层 (service): 实现业务逻辑
// - 模型层 (model): 定义数据结构
// - 中间件层 (middleware): 提供请求处理管道中的附加功能

// 声明controller子模块
pub mod controller;

// 声明service子模块
pub mod service;

// 声明model子模块
pub mod model;

// 声明middleware子模块
pub mod middleware;

// 声明repository子模块
pub mod repository;

// 这里没有重新导出子模块中的项（与其他mod.rs文件不同）。
// 这是因为app模块的子模块之间有明确的层次和依赖关系，
// 我们希望保持这种明确的引用路径，例如：
// - 使用service: use crate::app::service::create_task;
// - 使用controller: use crate::app::controller::create_task;
//
// 这种方式使代码更清晰，更容易理解模块之间的关系。
