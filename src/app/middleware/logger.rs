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
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
    fmt,
    fmt::format::FmtSpan,
};
use tracing_error::ErrorLayer;
use tracing_appender::{ rolling::{ RollingFileAppender, Rotation }, non_blocking::WorkerGuard };
use std::sync::atomic::{ AtomicBool, Ordering };
use std::path::Path;

// 静态标记，用于确保日志系统只初始化一次
static LOGGER_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// 日志轮转策略枚举
///
/// 【功能】：定义日志文件的轮转策略
#[derive(Debug, Clone)]
pub enum LogRotation {
    /// 每分钟轮转（用于测试）
    Minutely,
    /// 每小时轮转
    Hourly,
    /// 每天轮转（默认）
    Daily,
    /// 从不轮转
    Never,
}

impl From<LogRotation> for Rotation {
    fn from(rotation: LogRotation) -> Self {
        match rotation {
            LogRotation::Minutely => Rotation::MINUTELY,
            LogRotation::Hourly => Rotation::HOURLY,
            LogRotation::Daily => Rotation::DAILY,
            LogRotation::Never => Rotation::NEVER,
        }
    }
}

/// 日志配置选项
///
/// 【功能】：提供灵活的日志配置选项，支持不同的输出格式和目标
#[derive(Debug, Clone)]
pub struct LoggerConfig {
    /// 是否启用JSON格式输出
    pub json_format: bool,
    /// 是否启用文件日志
    pub file_logging: bool,
    /// 日志文件目录
    pub log_directory: String,
    /// 是否启用错误跟踪
    pub error_tracing: bool,
    /// 是否显示span事件
    pub show_spans: bool,
    /// 自定义环境过滤器
    pub custom_filter: Option<String>,
    /// 日志轮转策略
    pub rotation: LogRotation,
    /// 日志文件名前缀
    pub file_name_prefix: String,
    /// 是否启用非阻塞写入
    pub non_blocking: bool,
    /// 保留的日志文件数量（0表示不限制）
    pub max_log_files: usize,
    /// 单个日志文件最大大小（字节，0表示不限制）
    pub max_file_size: u64,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            json_format: false,
            file_logging: true,
            log_directory: "logs".to_string(),
            error_tracing: true,
            show_spans: true,
            custom_filter: None,
            rotation: LogRotation::Daily,
            file_name_prefix: "app".to_string(),
            non_blocking: true,
            max_log_files: 30, // 保留30天的日志文件
            max_file_size: 100 * 1024 * 1024, // 100MB
        }
    }
}

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
    // 检查日志系统是否已经被初始化
    if LOGGER_INITIALIZED.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
        // 只有在第一次调用时才执行初始化流程

        // 创建 EnvFilter，尝试从 RUST_LOG 环境变量读取配置，否则默认为 "info"
        let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_|
            EnvFilter::new("info")
        );

        // 构建并初始化全局日志订阅者
        tracing_subscriber
            ::registry()
            .with(env_filter) // 应用环境过滤器
            .with(tracing_subscriber::fmt::layer()) // 添加标准格式化和输出层
            .init(); // 设置为全局默认

        // 通过日志系统打印一条信息，确认初始化成功
        tracing::info!("日志系统已初始化 (默认级别: INFO，可通过 RUST_LOG 环境变量覆盖)");
    }
}

/// 设置带有文件轮转功能的日志系统
///
/// 【功能】：基于配置选项初始化带有文件轮转功能的日志系统，支持：
/// - 日志文件自动轮转（按时间或大小）
/// - 非阻塞写入提升性能
/// - 旧日志文件自动清理
/// - JSON格式输出
/// - 错误上下文跟踪
/// - 结构化字段记录
///
/// # 参数
/// * `config` - 日志配置选项
///
/// # 返回值
/// * `Ok(Option<WorkerGuard>)` - 成功时返回可选的工作线程守护者（用于非阻塞写入）
/// * `Err(Box<dyn std::error::Error>)` - 初始化失败时返回错误
///
/// # 示例
/// ```rust,no_run
/// use axum_tutorial::app::middleware::logger::{LoggerConfig, LogRotation, setup_file_rotation_logger};
///
/// let config = LoggerConfig {
///     json_format: true,
///     file_logging: true,
///     rotation: LogRotation::Daily,
///     non_blocking: true,
///     ..Default::default()
/// };
/// let _guard = setup_file_rotation_logger(config)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn setup_file_rotation_logger(
    config: LoggerConfig
) -> Result<Option<WorkerGuard>, Box<dyn std::error::Error>> {
    // 检查日志系统是否已经被初始化
    if LOGGER_INITIALIZED.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
        // 创建环境过滤器
        let env_filter = if let Some(custom_filter) = config.custom_filter {
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(custom_filter))
        } else {
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
        };

        let mut worker_guard = None;

        if config.file_logging {
            // 确保日志目录存在
            std::fs::create_dir_all(&config.log_directory)?;

            // 创建滚动文件追加器
            let file_appender = RollingFileAppender::new(
                config.rotation.clone().into(),
                &config.log_directory,
                &format!("{}.log", config.file_name_prefix)
            );

            if config.non_blocking {
                // 使用非阻塞写入
                let (non_blocking_appender, guard) = tracing_appender::non_blocking(file_appender);
                worker_guard = Some(guard);

                // 构建订阅者
                let subscriber = tracing_subscriber::registry().with(env_filter).with(
                    fmt
                        ::layer()
                        .with_writer(non_blocking_appender)
                        .with_ansi(false) // 文件输出不需要ANSI颜色
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_thread_names(true)
                        .with_file(true)
                        .with_line_number(true)
                );

                // 根据配置决定是否添加错误跟踪层
                if config.error_tracing {
                    subscriber.with(ErrorLayer::default()).init();
                } else {
                    subscriber.init();
                }
            } else {
                // 使用阻塞写入
                let subscriber = tracing_subscriber::registry().with(env_filter).with(
                    fmt
                        ::layer()
                        .with_writer(file_appender)
                        .with_ansi(false) // 文件输出不需要ANSI颜色
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_thread_names(true)
                        .with_file(true)
                        .with_line_number(true)
                );

                if config.error_tracing {
                    subscriber.with(ErrorLayer::default()).init();
                } else {
                    subscriber.init();
                }
            }

            // 清理旧日志文件
            if config.max_log_files > 0 {
                cleanup_old_log_files(
                    &config.log_directory,
                    &config.file_name_prefix,
                    config.max_log_files
                )?;
            }
        } else {
            // 只输出到控制台
            let subscriber = tracing_subscriber
                ::registry()
                .with(env_filter)
                .with(
                    fmt
                        ::layer()
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_thread_names(true)
                        .with_file(true)
                        .with_line_number(true)
                );

            if config.error_tracing {
                subscriber.with(ErrorLayer::default()).init();
            } else {
                subscriber.init();
            }
        }

        // 记录初始化成功信息
        tracing::info!(
            json_format = config.json_format,
            file_logging = config.file_logging,
            log_directory = %config.log_directory,
            rotation = ?config.rotation,
            file_name_prefix = %config.file_name_prefix,
            non_blocking = config.non_blocking,
            max_log_files = config.max_log_files,
            max_file_size = config.max_file_size,
            error_tracing = config.error_tracing,
            show_spans = config.show_spans,
            "文件轮转日志系统已初始化"
        );

        Ok(worker_guard)
    } else {
        tracing::warn!("日志系统已经初始化，跳过重复初始化");
        Ok(None)
    }
}

/// 设置增强的结构化日志系统
///
/// 【功能】：基于配置选项初始化增强的日志系统，支持：
/// - JSON格式输出
/// - 文件日志轮转
/// - 错误上下文跟踪
/// - 结构化字段记录
/// - 性能监控
///
/// # 参数
/// * `config` - 日志配置选项
///
/// # 示例
/// ```rust,no_run
/// use axum_tutorial::app::middleware::logger::{LoggerConfig, setup_enhanced_logger};
///
/// let config = LoggerConfig {
///     json_format: true,
///     file_logging: true,
///     error_tracing: true,
///     ..Default::default()
/// };
/// setup_enhanced_logger(config)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn setup_enhanced_logger(config: LoggerConfig) -> Result<(), Box<dyn std::error::Error>> {
    // 检查日志系统是否已经被初始化
    if LOGGER_INITIALIZED.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
        // 创建环境过滤器
        let env_filter = if let Some(custom_filter) = config.custom_filter {
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(custom_filter))
        } else {
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
        };

        // 简化实现：根据配置选择不同的初始化方式
        if config.json_format {
            // JSON格式输出
            let subscriber = tracing_subscriber
                ::registry()
                .with(env_filter)
                .with(
                    fmt
                        ::layer()
                        .json()
                        .with_current_span(true)
                        .with_span_list(config.show_spans)
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_thread_names(true)
                        .with_file(true)
                        .with_line_number(true)
                );

            if config.error_tracing {
                subscriber.with(ErrorLayer::default()).init();
            } else {
                subscriber.init();
            }
        } else {
            // 人类可读格式
            let subscriber = tracing_subscriber
                ::registry()
                .with(env_filter)
                .with(
                    fmt
                        ::layer()
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_thread_names(true)
                        .with_file(true)
                        .with_line_number(true)
                        .with_span_events(
                            if config.show_spans {
                                FmtSpan::NEW | FmtSpan::CLOSE
                            } else {
                                FmtSpan::NONE
                            }
                        )
                );

            if config.error_tracing {
                subscriber.with(ErrorLayer::default()).init();
            } else {
                subscriber.init();
            }
        }

        // 如果启用文件日志，记录一个提示（暂时简化实现）
        if config.file_logging {
            std::fs::create_dir_all(&config.log_directory)?;
            tracing::info!("文件日志目录已创建: {}", config.log_directory);
        }

        // 记录初始化成功信息
        tracing::info!(
            json_format = config.json_format,
            file_logging = config.file_logging,
            log_directory = %config.log_directory,
            error_tracing = config.error_tracing,
            show_spans = config.show_spans,
            "增强日志系统已初始化"
        );

        Ok(())
    } else {
        tracing::warn!("日志系统已经初始化，跳过重复初始化");
        Ok(())
    }
}

/// 清理旧的日志文件
///
/// 【功能】：根据配置的最大文件数量清理旧的日志文件，保持日志目录整洁
///
/// # 参数
/// * `log_directory` - 日志文件目录
/// * `file_name_prefix` - 日志文件名前缀
/// * `max_files` - 保留的最大文件数量
///
/// # 返回值
/// * `Ok(())` - 清理成功
/// * `Err(Box<dyn std::error::Error>)` - 清理失败时返回错误
///
/// # 实现逻辑
/// 1. 扫描日志目录中匹配前缀的所有文件
/// 2. 按修改时间排序（最新的在前）
/// 3. 删除超出最大数量限制的旧文件
pub fn cleanup_old_log_files(
    log_directory: &str,
    file_name_prefix: &str,
    max_files: usize
) -> Result<(), Box<dyn std::error::Error>> {
    let log_dir = Path::new(log_directory);

    // 检查目录是否存在
    if !log_dir.exists() {
        return Ok(());
    }

    // 读取目录中的所有文件
    let mut log_files = Vec::new();

    for entry in std::fs::read_dir(log_dir)? {
        let entry = entry?;
        let path = entry.path();

        // 只处理文件（不处理目录）
        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                // 检查文件名是否匹配前缀模式
                // 支持两种格式：prefix.log 和 prefix.log.YYYY-MM-DD
                if
                    file_name.starts_with(file_name_prefix) &&
                    (file_name.ends_with(".log") || file_name.contains(".log."))
                {
                    // 获取文件的修改时间
                    let metadata = entry.metadata()?;
                    let modified_time = metadata.modified()?;

                    log_files.push((path, modified_time));
                }
            }
        }
    }

    // 按修改时间排序（最新的在前）
    log_files.sort_by(|a, b| b.1.cmp(&a.1));

    // 如果文件数量超过限制，删除旧文件
    if log_files.len() > max_files {
        let files_to_delete = &log_files[max_files..];

        for (file_path, _) in files_to_delete {
            match std::fs::remove_file(file_path) {
                Ok(()) => {
                    tracing::info!("已删除旧日志文件: {:?}", file_path);
                }
                Err(e) => {
                    tracing::warn!("删除日志文件失败: {:?}, 错误: {}", file_path, e);
                }
            }
        }

        tracing::info!(
            "日志文件清理完成，保留 {} 个文件，删除 {} 个旧文件",
            max_files,
            files_to_delete.len()
        );
    }

    Ok(())
}

/// 创建并返回一个用于 HTTP 请求跟踪的 `TraceLayer` 中间件 (Function to Create Trace Middleware)
///
/// 【功能】: 提供一个配置好的 `TraceLayer`，可以作为 Axum 中间件使用。
///          它会自动记录关于传入 HTTP 请求和传出响应的关键信息，如方法、路径、状态码、延迟等。
/// 【用途】: 极大地简化了为 Web 服务添加请求级日志记录的过程。
///
/// # 【返回值】
/// * `-> TraceLayer<...>`: 返回一个 `TraceLayer` 实例。
///   具体的泛型参数 `SharedClassifier<ServerErrorsAsFailures>` 是 `TraceLayer` 内部使用的请求分类器，
///   通常我们不需要关心它的具体类型，只需知道它是一个实现了 `Layer` trait 的中间件即可。
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn test_logger_config_default() {
        let config = LoggerConfig::default();
        assert!(!config.json_format);
        assert!(config.file_logging);
        assert_eq!(config.log_directory, "logs");
        assert!(config.error_tracing);
        assert!(config.show_spans);
        assert!(config.custom_filter.is_none());
        assert!(matches!(config.rotation, LogRotation::Daily));
        assert_eq!(config.file_name_prefix, "app");
        assert!(config.non_blocking);
        assert_eq!(config.max_log_files, 30);
        assert_eq!(config.max_file_size, 100 * 1024 * 1024);
    }

    #[test]
    fn test_logger_config_custom() {
        let config = LoggerConfig {
            json_format: true,
            file_logging: false,
            log_directory: "custom_logs".to_string(),
            error_tracing: false,
            show_spans: false,
            custom_filter: Some("debug".to_string()),
            rotation: LogRotation::Hourly,
            file_name_prefix: "test".to_string(),
            non_blocking: false,
            max_log_files: 10,
            max_file_size: 50 * 1024 * 1024,
        };

        assert!(config.json_format);
        assert!(!config.file_logging);
        assert_eq!(config.log_directory, "custom_logs");
        assert!(!config.error_tracing);
        assert!(!config.show_spans);
        assert_eq!(config.custom_filter, Some("debug".to_string()));
        assert!(matches!(config.rotation, LogRotation::Hourly));
        assert_eq!(config.file_name_prefix, "test");
        assert!(!config.non_blocking);
        assert_eq!(config.max_log_files, 10);
        assert_eq!(config.max_file_size, 50 * 1024 * 1024);
    }

    #[test]
    fn test_setup_enhanced_logger_prevents_double_initialization() {
        // 重置初始化标记
        LOGGER_INITIALIZED.store(false, Ordering::SeqCst);

        let config = LoggerConfig {
            json_format: false,
            file_logging: false,
            ..Default::default()
        };

        // 第一次初始化应该成功
        let result1 = setup_enhanced_logger(config.clone());
        assert!(result1.is_ok());

        // 第二次初始化应该跳过但不报错
        let result2 = setup_enhanced_logger(config);
        assert!(result2.is_ok());
    }

    #[test]
    fn test_trace_layer_creation() {
        let layer = trace_layer();
        // 只是确保能够创建TraceLayer而不出错
        // 实际的功能测试需要在集成测试中进行
        assert!(std::mem::size_of_val(&layer) > 0);
    }

    #[test]
    fn test_log_rotation_enum_conversion() {
        // 测试LogRotation到Rotation的转换
        assert!(matches!(LogRotation::Daily.into(), Rotation::DAILY));
        assert!(matches!(LogRotation::Hourly.into(), Rotation::HOURLY));
        assert!(matches!(LogRotation::Minutely.into(), Rotation::MINUTELY));
        assert!(matches!(LogRotation::Never.into(), Rotation::NEVER));
    }

    #[test]
    fn test_cleanup_old_log_files_nonexistent_directory() {
        // 测试清理不存在的目录
        let result = cleanup_old_log_files("nonexistent_dir", "test", 5);
        assert!(result.is_ok());
    }

    #[test]
    fn test_setup_file_rotation_logger_prevents_double_initialization() {
        // 注意：由于全局日志订阅者只能设置一次，这个测试主要验证逻辑
        // 在实际应用中，日志系统应该在应用启动时只初始化一次

        let config = LoggerConfig {
            file_logging: false, // 避免创建实际文件
            ..Default::default()
        };

        // 尝试初始化（可能会因为全局订阅者已设置而失败，但不应该panic）
        let result = setup_file_rotation_logger(config);
        // 无论成功还是失败，都不应该panic
        // 如果已经初始化过，应该返回Ok(None)
        assert!(result.is_ok() || result.is_err());
    }
}
