//! `lib.rs`
//!
//! ## 【项目库根模块】 (Library Root)
//!
//! ### 职责 (Responsibilities)
//! 1. **模块组织中心 (Module Hub)**:
//!    - 使用 `pub mod` 声明并公开项目的所有核心模块 (`app`, `config`, `error`, `routes`, `startup`)。
//!    - 这将它们组合成一个单一的、内聚的库 (`axum-tutorial`)。
//!
//! 2. **应用执行入口 (Application Runner)**:
//!    - 提供一个公共的 `run` 函数，封装了应用的完整启动序列：
//!      - 初始化应用（日志、数据库、路由）。
//!      - 绑定 TCP 端口。
//!      - 启动 Axum 服务器。
//!
//! ### 设计决策 (Design Rationale)
//! - **创建库包 (Library Crate)**: 将项目核心逻辑从 `main.rs` 抽离到 `lib.rs` 中，
//!   使得 `axum-tutorial` 成为一个【库包】。
//! - **提升可测试性 (Testability)**: 作为一个库，其所有公共函数和模块都可以被
//!   集成测试 (`/tests` 目录) 和单元测试直接导入和调用。这是解决之前
//!   `unresolved module` 编译错误的关键。
//! - **清晰的关注点分离 (Separation of Concerns)**:
//!   - `lib.rs`: 定义"是什么"和"做什么"（应用的核心能力）。
//!   - `main.rs`: 充当启动器，决定"何时"运行。
//!   - `/tests/*.rs`: 充当验证器，验证 `lib.rs` 中定义的功能是否正确。

// --- 公开项目核心模块 ---
pub mod app;
pub mod config;
pub mod error;
pub mod routes;
pub mod startup;

use anyhow::Result;
use config::AppConfig;
use tokio::net::TcpListener;
use tracing::info;

/// 运行应用程序的主函数。
///
/// 此函数负责初始化应用、绑定服务器地址并启动 HTTP 服务器。
/// 它被设计为可以从 `main.rs` 调用，也可以在集成测试中用于启动一个真实的应用实例。
///
/// # 参数
/// - `config`: `AppConfig` 的一个实例，包含了所有的应用配置。
///
/// # 返回
/// - `Result<()>`: 如果服务器成功运行并正常关闭，则返回 `Ok(())`。
///   如果在此过程中发生任何错误（例如，无法绑定端口），则返回错误。
pub async fn run(config: AppConfig) -> Result<()> {
    // 初始化应用，包括日志、数据库和路由
    let (app, _) = startup::init_app(config.clone()).await?;

    // 绑定 TCP 监听器
    let http_addr = config.http_addr;
    let listener = TcpListener::bind(http_addr).await?;
    info!("HTTP/1.1 服务器启动，监听地址: http://{}", http_addr);

    // 启动 Axum 服务
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
