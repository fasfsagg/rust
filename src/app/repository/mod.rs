//! `repository/mod.rs`
//!
//! 这个模块是仓库层的根模块，负责导出所有具体的仓库实现。
//! 仓库层（Repository Pattern）的目的是将数据访问逻辑（例如数据库查询）
//! 从业务逻辑（服务层）中分离出来，形成一个独立的、可测试的、可替换的组件。
//!
//! ## 设计原则
//! - **封装数据源**: 隐藏底层数据存储（如 SeaORM、SQL、NoSQL）的实现细节。
//! - **提供清晰的 API**: 为服务层提供面向领域的、与数据相关的接口。

pub mod task_repository;

// 重新导出 TaskRepository 以便上层模块（主要是 service）可以更方便地使用。
// 使用 `crate::app::repository::TaskRepository` 而不是 `...::task_repository::TaskRepository`
pub use task_repository::TaskRepository;
