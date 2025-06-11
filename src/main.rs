//! main.rs (启动器)
//!
//! ## 【二进制可执行文件入口】 (Binary Crate Entry Point)
//!
//! ### 职责 (Responsibilities)
//! - **唯一职责**: 作为程序的可执行入口点。
//! - **调用库**: 调用 `axum-tutorial` 库的 `run` 函数来启动整个应用程序。
//!
//! ### 设计决策 (Design Rationale)
//! - **关注点分离**: `main.rs` 只关心"启动"这一行为，而将所有
//!   复杂的应用逻辑（配置、初始化、路由等）委托给库 (`lib.rs`)。
//! - **符合 Rust 惯例**: 这是 Rust 社区推荐的构建可测试应用程序的标准模式。
//!   一个包可以同时拥有一个库 (`lib.rs`) 和一个或多个二进制文件
//!   (`main.rs`, `bin/*.rs`)，这些二进制文件通常是库的使用者。

use anyhow::Result;
use axum_tutorial::config::AppConfig;
use axum_tutorial::run;

/// 主函数 (程序入口)
///
/// `#[tokio::main]` 宏设置并启动 Tokio 异步运行时。
#[tokio::main]
async fn main() -> Result<()> {
    // --- 步骤 1: 加载应用程序配置 ---
    // 从环境变量或 .env 文件中加载配置。
    let config = AppConfig::from_env();

    // --- 步骤 2: 运行应用程序 ---
    // 调用库中定义的 `run` 函数，并将配置传递给它。
    // `run` 函数将处理所有后续的初始化和服务器启动逻辑。
    // `.await` 等待 `run` 函数完成（在服务器关闭或出错时）。
    run(config).await
}
