// app/middleware/logger.rs
//
// /---------------------------------------------------------------------------------------------------------\
// |                                    【日志中间件模块】 (logger.rs)                                     |
// |---------------------------------------------------------------------------------------------------------|
// |                                                                                                         |
// | 1. **导入依赖**:                                                                                        |
// |    - `tower_http::trace::{self, TraceLayer}`: Tower HTTP 提供的请求跟踪中间件和相关工具。             |
// |    - `tracing::Level`: 定义日志级别 (如 INFO, DEBUG, ERROR)。                                        |
// |    - `tracing_subscriber::{...}`: 用于配置和初始化 `tracing` 日志系统的工具。                         |
// |                                                                                                         |
// | 2. **`setup_logger()` 函数**: 公共函数，用于初始化全局日志系统。                                         |
// |    - **职责**: 配置日志的级别、格式和输出目标。通常在应用启动时调用一次。                               |
// |    - **实现**:                                                                                         |
// |      - `EnvFilter::try_from_default_env()`: 尝试从 `RUST_LOG` 环境变量读取日志过滤指令。             |
// |        例如 `RUST_LOG=info,my_app=debug` 表示默认级别为 INFO，但 `my_app` 模块为 DEBUG。              |
// |      - `.unwrap_or_else(|_| EnvFilter::new("info"))`: 如果环境变量未设置，则默认使用 "info" 级别。  |
// |      - `tracing_subscriber::registry()`: 创建一个订阅者注册表 (registry)。                             |
// |      - `.with(env_filter)`: 将环境过滤器层添加到注册表中。                                           |
// |      - `.with(tracing_subscriber::fmt::layer())`: 添加格式化层，将日志输出到标准输出 (控制台)。      |
// |      - `.init()`: 将构建好的订阅者设置为全局默认日志处理器。                                         |
// |                                                                                                         |
// | 3. **`trace_layer()` 函数**: 公共函数，用于创建并返回一个配置好的 `TraceLayer` 中间件。                   |
// |    - **职责**: 提供一个即插即用的中间件，用于自动记录 HTTP 请求的生命周期事件。                         |
// |    - **返回类型**: `TraceLayer<...>` (具体的分类器类型通常不重要)。                                   |
// |    - **实现**:                                                                                         |
// |      - `TraceLayer::new_for_http()`: 创建一个专门为 HTTP 优化的 `TraceLayer`。                         |
// |      - `.on_request(...)`: 配置当请求开始时记录日志的行为 (默认 INFO 级别)。                         |
// |      - `.on_response(...)`: 配置当响应生成时记录日志的行为 (默认 INFO 级别)。                         |
// |      - `.on_body_chunk(...)`: 配置当处理响应体数据块时的行为 (通常用于调试)。                         |
// |      - `.on_failure(...)`: 配置当请求处理失败时记录日志的行为 (默认 ERROR 级别)。                     |
// |                                                                                                         |
// \---------------------------------------------------------------------------------------------------------/
//
// 【核心职责】: 配置和提供日志记录功能，包括应用程序的全局日志设置和针对 HTTP 请求的详细跟踪中间件。
// 【关键技术】: `tracing` (日志框架), `tracing_subscriber` (日志配置), `tower_http::trace::TraceLayer` (HTTP 请求跟踪中间件), `EnvFilter` (通过环境变量控制日志级别)。

// --- 导入依赖 ---
// `tower_http::trace`: 包含 TraceLayer 中间件和相关的配置助手 (DefaultOnRequest, DefaultOnResponse 等)。
use tower_http::trace::{ self, TraceLayer };
// `tracing::Level`: 定义不同的日志严重级别 (ERROR, WARN, INFO, DEBUG, TRACE)。
use tracing::Level;
// `tracing_subscriber`: 用于配置 `tracing` 日志系统的核心库。
// `SubscriberExt`: 扩展 trait，提供 `.with()` 方法来组合不同的日志层 (Layer)。
// `SubscriberInitExt`: 扩展 trait，提供 `.init()` 方法来设置全局日志订阅者。
// `EnvFilter`: 一个日志层，根据环境变量 (通常是 `RUST_LOG`) 来过滤日志事件。
use tracing_subscriber::{ layer::SubscriberExt, util::SubscriberInitExt, EnvFilter };

/// 设置应用程序的全局日志系统 (Function to Setup Application Logging)
///
/// 【功能】: 初始化 `tracing` 日志框架。
///          配置日志事件如何被过滤、格式化和输出。
/// 【调用时机】: 通常在应用程序启动的最开始阶段 (例如 `startup.rs` 的 `init_app` 函数中) 调用一次。
///
/// # 【实现细节】
/// 1. **环境过滤器 (`EnvFilter`)**: [[Tracing 配置: EnvFilter]]
///    - `EnvFilter::try_from_default_env()`: 尝试从环境变量 `RUST_LOG` 中读取过滤指令。
///      这允许开发者在运行时通过设置环境变量来动态调整日志的详细程度，而无需重新编译代码。
///      例如: `RUST_LOG=info` (只显示 INFO 及以上级别), `RUST_LOG=debug` (显示 DEBUG 及以上), `RUST_LOG=axum_tutorial=trace` (只显示本应用的 TRACE 日志)。
///    - `.unwrap_or_else(|_| EnvFilter::new("info"))`: 如果 `RUST_LOG` 环境变量没有设置，则默认使用 `info` 级别过滤。
///      这意味着默认情况下，只有 INFO, WARN, ERROR 级别的日志会被显示。
/// 2. **订阅者构建 (`tracing_subscriber::registry()`)**: [[Tracing 配置: Registry & Layers]]
///    - `registry()`: 创建一个基础的订阅者注册表，用于组合不同的日志处理层。
///    - `.with(env_filter)`: 添加之前创建的环境过滤器层。只有通过这个过滤器的日志事件才会继续向下传递。
///    - `.with(tracing_subscriber::fmt::layer())`: 添加格式化层。
///      `fmt::layer()` 提供了一个标准的日志格式，并将日志输出到标准错误流 (stderr) 或标准输出流 (stdout)。
/// 3. **初始化 (`.init()`)**: [[Tracing 配置: Initialization]]
///    - `.init()`: 将构建好的订阅者设置为【全局默认】日志处理器。
///      一旦设置，应用程序中所有通过 `tracing` 宏 (如 `info!`, `debug!`, `error!`) 发出的日志事件都将被这个订阅者处理。
pub fn setup_logger() {
    // 创建 EnvFilter，尝试从 RUST_LOG 环境变量读取配置，否则默认为 "info"
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // 构建并初始化全局日志订阅者
    tracing_subscriber
        ::registry()
        .with(env_filter) // 应用环境过滤器
        .with(tracing_subscriber::fmt::layer()) // 添加标准格式化和输出层
        .init(); // 设置为全局默认

    // 通过日志系统打印一条信息，确认初始化成功
    tracing::info!("日志系统已初始化 (默认级别: INFO，可通过 RUST_LOG 环境变量覆盖)");
}

/// 创建并返回一个用于 HTTP 请求跟踪的 `TraceLayer` 中间件 (Function to Create Trace Middleware)
///
/// 【功能】: 提供一个配置好的 `TraceLayer`，可以作为 Axum 中间件使用。
///          它会自动记录关于传入 HTTP 请求和传出响应的关键信息，如方法、路径、状态码、延迟等。
/// 【用途】: 极大地简化了为 Web 服务添加请求级日志记录的过程。
///
/// # 【返回值】
/// * `-> TraceLayer<...>`: 返回一个 `TraceLayer` 实例。
///                       具体的泛型参数 `SharedClassifier<ServerErrorsAsFailures>` 是 `TraceLayer` 内部使用的请求分类器，
///                       通常我们不需要关心它的具体类型，只需知道它是一个实现了 `Layer` trait 的中间件即可。
pub fn trace_layer() -> TraceLayer<tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>> {
    // `TraceLayer::new_for_http()`: 创建一个针对 HTTP 优化的 TraceLayer。
    // 它使用一个默认的分类器，将 HTTP 状态码 4xx 和 5xx 视为失败。
    TraceLayer::new_for_http()
        // `.on_request(...)`: 配置当请求开始时如何记录日志。[[TraceLayer 配置: on_request]]
        //   - `trace::DefaultOnRequest::new()`: 使用默认的请求日志格式。
        //   - `.level(Level::INFO)`: 将请求开始事件的日志级别设置为 INFO。
        .on_request(trace::DefaultOnRequest::new().level(Level::INFO))
        // `.on_response(...)`: 配置当响应头准备好时如何记录日志。[[TraceLayer 配置: on_response]]
        //   - `trace::DefaultOnResponse::new()`: 使用默认的响应日志格式，包含状态码和延迟。
        //   - `.level(Level::INFO)`: 将响应事件的日志级别设置为 INFO。
        //   - `.latency_unit(tower_http::LatencyUnit::Micros)`: (可选) 设置延迟单位为微秒。
        .on_response(trace::DefaultOnResponse::new().level(Level::INFO))
        // `.on_body_chunk(...)`: 配置当处理响应体数据块时如何记录日志 (可选)。[[TraceLayer 配置: on_body_chunk]]
        // 通常在调试流式响应时有用。
        .on_body_chunk(trace::DefaultOnBodyChunk::new())
        // `.on_failure(...)`: 配置当请求处理过程中发生分类为失败的事件时如何记录日志。[[TraceLayer 配置: on_failure]]
        //   - `trace::DefaultOnFailure::new()`: 使用默认的失败日志格式。
        //   - `.level(Level::ERROR)`: 将失败事件的日志级别设置为 ERROR。
        //     (默认分类器将 5xx 错误视为失败)
        .on_failure(trace::DefaultOnFailure::new().level(Level::ERROR))
}
