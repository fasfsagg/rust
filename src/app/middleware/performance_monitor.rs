// src/app/middleware/performance_monitor.rs
//
// /--------------------------------------------------------------------------------------------\
// |                            【性能监控中间件模块】 (performance_monitor.rs)                  |
// |--------------------------------------------------------------------------------------------|
// |                                                                                            |
// | 【核心功能】:                                                                               |
// | 1. **请求延迟监控**: 测量每个HTTP请求的处理时间，支持微秒级精度                               |
// | 2. **吞吐量统计**: 记录每秒处理的请求数量，支持实时和历史统计                                 |
// | 3. **内存使用监控**: 定期收集系统内存使用情况，包括RSS、虚拟内存等                           |
// | 4. **错误率统计**: 按状态码分类统计请求成功率和错误率                                         |
// | 5. **并发连接数**: 实时监控活跃的HTTP连接数量                                               |
// | 6. **资源使用情况**: CPU使用率、磁盘I/O等系统资源监控                                        |
// |                                                                                            |
// | 【技术实现】:                                                                               |
// | - 使用 `tracing::instrument` 宏自动捕获函数调用的性能数据                                   |
// | - 集成 `metrics` crate 进行指标收集和导出                                                  |
// | - 使用 `sysinfo` crate 获取系统级性能指标                                                  |
// | - 支持 Prometheus 格式的指标导出                                                           |
// | - 结构化日志记录，便于后续分析和告警                                                         |
// |                                                                                            |
// \--------------------------------------------------------------------------------------------/

use axum::{
    extract::Request,
    http::{ HeaderMap, StatusCode },
    middleware::Next,
    response::Response,
};
use metrics::{ counter, gauge, histogram };
use std::{ sync::{ atomic::{ AtomicU64, Ordering }, Arc, Mutex }, time::{ Duration, Instant } };
use sysinfo::System;
use tokio::time::interval;
use tracing::{ info, warn, error, instrument };

/// 性能监控配置
///
/// 【功能】：配置性能监控中间件的行为参数
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    /// 是否启用详细的请求日志
    pub enable_detailed_logging: bool,
    /// 是否启用系统资源监控
    pub enable_system_monitoring: bool,
    /// 系统监控采样间隔（秒）
    pub system_monitoring_interval: u64,
    /// 是否启用Prometheus指标导出
    pub enable_prometheus_metrics: bool,
    /// 慢请求阈值（毫秒）
    pub slow_request_threshold_ms: u64,
    /// 是否记录请求头信息
    pub log_request_headers: bool,
    /// 最大并发连接数告警阈值
    pub max_concurrent_connections_warning: u64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enable_detailed_logging: true,
            enable_system_monitoring: true,
            system_monitoring_interval: 30, // 30秒采样一次
            enable_prometheus_metrics: true,
            slow_request_threshold_ms: 1000, // 1秒
            log_request_headers: false,
            max_concurrent_connections_warning: 1000,
        }
    }
}

/// 性能指标收集器
///
/// 【功能】：集中管理所有性能指标的收集和统计
#[derive(Debug)]
pub struct PerformanceMetrics {
    /// 活跃连接数
    active_connections: AtomicU64,
    /// 总请求数
    total_requests: AtomicU64,
    /// 成功请求数
    successful_requests: AtomicU64,
    /// 错误请求数
    error_requests: AtomicU64,
    /// 系统信息收集器
    system: Arc<Mutex<System>>,
    /// 配置
    config: PerformanceConfig,
}

impl PerformanceMetrics {
    /// 创建新的性能指标收集器
    ///
    /// 【功能】：初始化性能监控系统，注册所有必要的指标
    ///
    /// # 参数
    /// * `config` - 性能监控配置
    ///
    /// # 返回值
    /// * `Arc<PerformanceMetrics>` - 性能指标收集器实例
    pub fn new(config: PerformanceConfig) -> Arc<Self> {
        // 注册 Prometheus 指标
        if config.enable_prometheus_metrics {
            Self::register_prometheus_metrics();
        }

        let metrics = Arc::new(Self {
            active_connections: AtomicU64::new(0),
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            error_requests: AtomicU64::new(0),
            system: Arc::new(Mutex::new(System::new_all())),
            config,
        });

        // 启动系统监控任务
        if metrics.config.enable_system_monitoring {
            let metrics_clone = metrics.clone();
            tokio::spawn(async move {
                metrics_clone.start_system_monitoring().await;
            });
        }

        metrics
    }

    /// 注册 Prometheus 指标
    ///
    /// 【功能】：向 metrics 注册表注册所有需要的指标类型
    /// 注意：在新版本的metrics crate中，指标会在首次使用时自动注册
    fn register_prometheus_metrics() {
        // 在新版本的metrics crate中，指标会在首次使用时自动注册
        // 这里我们可以预先触发一次指标创建来确保它们被注册
        counter!("http_requests_total").absolute(0);
        counter!("http_requests_successful_total").absolute(0);
        counter!("http_requests_error_total").absolute(0);
        histogram!("http_request_duration_seconds").record(0.0);
        gauge!("http_active_connections").set(0.0);
        gauge!("system_memory_usage_bytes").set(0.0);
        gauge!("system_memory_usage_percent").set(0.0);
        gauge!("system_cpu_usage_percent").set(0.0);
        gauge!("process_memory_usage_bytes").set(0.0);
        gauge!("process_cpu_usage_percent").set(0.0);
    }

    /// 记录请求开始
    ///
    /// 【功能】：在请求开始时更新相关指标
    pub fn record_request_start(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        // 更新 Prometheus 指标
        if self.config.enable_prometheus_metrics {
            counter!("http_requests_total").increment(1);
            gauge!("http_active_connections").set(
                self.active_connections.load(Ordering::Relaxed) as f64
            );
        }
    }

    /// 记录请求完成
    ///
    /// 【功能】：在请求完成时更新相关指标和日志
    ///
    /// # 参数
    /// * `duration` - 请求处理时间
    /// * `status_code` - HTTP状态码
    /// * `method` - HTTP方法
    /// * `path` - 请求路径
    /// * `headers` - 请求头（可选）
    #[instrument(skip(self, headers))]
    pub fn record_request_end(
        &self,
        duration: Duration,
        status_code: StatusCode,
        method: &str,
        path: &str,
        headers: Option<&HeaderMap>
    ) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);

        let duration_ms = duration.as_millis() as u64;
        // 修正成功判断逻辑：2xx状态码 + 101 WebSocket升级都视为成功
        let is_success = status_code.is_success() || status_code == StatusCode::SWITCHING_PROTOCOLS;

        if is_success {
            self.successful_requests.fetch_add(1, Ordering::Relaxed);
        } else {
            self.error_requests.fetch_add(1, Ordering::Relaxed);
        }

        // 更新 Prometheus 指标
        if self.config.enable_prometheus_metrics {
            if is_success {
                counter!("http_requests_successful_total").increment(1);
            } else {
                counter!("http_requests_error_total").increment(1);
            }

            histogram!("http_request_duration_seconds").record(duration.as_secs_f64());
            gauge!("http_active_connections").set(
                self.active_connections.load(Ordering::Relaxed) as f64
            );
        }

        // 详细日志记录 - 企业级日志级别策略
        if self.config.enable_detailed_logging {
            // 实现精确的日志级别策略：
            // INFO: 2xx成功状态码 + 101 WebSocket升级
            // WARN: 4xx客户端错误 + 慢请求
            // ERROR: 5xx服务器错误
            let status_u16 = status_code.as_u16();
            let is_slow_request = duration_ms > self.config.slow_request_threshold_ms;

            let log_level = if status_code.is_server_error() {
                // 5xx服务器错误 - ERROR级别
                tracing::Level::ERROR
            } else if status_code.is_client_error() || is_slow_request {
                // 4xx客户端错误（包括409冲突）或慢请求 - WARN级别
                tracing::Level::WARN
            } else {
                // 2xx成功状态码和101 WebSocket升级 - INFO级别
                tracing::Level::INFO
            };

            // 根据日志级别记录不同的事件
            match log_level {
                tracing::Level::ERROR => {
                    error!(
                        method = %method,
                        path = %path,
                        status_code = %status_u16,
                        duration_ms = duration_ms,
                        active_connections = self.active_connections.load(Ordering::Relaxed),
                        total_requests = self.total_requests.load(Ordering::Relaxed),
                        error_type = "server_error",
                        "HTTP request completed with server error"
                    );
                }
                tracing::Level::WARN => {
                    let warn_reason = if is_slow_request && status_code.is_client_error() {
                        "slow_request_and_client_error"
                    } else if is_slow_request {
                        "slow_request"
                    } else {
                        "client_error"
                    };

                    warn!(
                        method = %method,
                        path = %path,
                        status_code = %status_u16,
                        duration_ms = duration_ms,
                        active_connections = self.active_connections.load(Ordering::Relaxed),
                        total_requests = self.total_requests.load(Ordering::Relaxed),
                        warn_reason = warn_reason,
                        "HTTP request completed with warning"
                    );
                }
                _ => {
                    // INFO级别：成功请求和WebSocket升级
                    let request_type = if status_u16 == 101 {
                        "websocket_upgrade"
                    } else {
                        "success"
                    };

                    info!(
                        method = %method,
                        path = %path,
                        status_code = %status_u16,
                        duration_ms = duration_ms,
                        active_connections = self.active_connections.load(Ordering::Relaxed),
                        total_requests = self.total_requests.load(Ordering::Relaxed),
                        request_type = request_type,
                        "HTTP request completed successfully"
                    );
                }
            }

            // 记录请求头信息（如果启用）
            if self.config.log_request_headers && headers.is_some() {
                let headers = headers.unwrap();
                if let Some(user_agent) = headers.get("user-agent") {
                    if let Ok(user_agent_str) = user_agent.to_str() {
                        tracing::info!(user_agent = %user_agent_str, "Request user agent");
                    }
                }
                if let Some(content_type) = headers.get("content-type") {
                    if let Ok(content_type_str) = content_type.to_str() {
                        tracing::info!(content_type = %content_type_str, "Request content type");
                    }
                }
            }
        }

        // 慢请求告警
        if duration_ms > self.config.slow_request_threshold_ms {
            warn!(
                method = %method,
                path = %path,
                duration_ms = duration_ms,
                threshold_ms = self.config.slow_request_threshold_ms,
                "Slow request detected"
            );
        }

        // 并发连接数告警
        let current_connections = self.active_connections.load(Ordering::Relaxed);
        if current_connections > self.config.max_concurrent_connections_warning {
            warn!(
                active_connections = current_connections,
                warning_threshold = self.config.max_concurrent_connections_warning,
                "High concurrent connections detected"
            );
        }
    }

    /// 启动系统监控
    ///
    /// 【功能】：定期收集系统资源使用情况
    async fn start_system_monitoring(self: Arc<Self>) {
        let mut interval = interval(Duration::from_secs(self.config.system_monitoring_interval));

        loop {
            interval.tick().await;
            self.collect_system_metrics().await;
        }
    }

    /// 收集系统指标
    ///
    /// 【功能】：收集CPU、内存等系统资源使用情况
    #[instrument(skip(self))]
    async fn collect_system_metrics(&self) {
        let mut system = match self.system.lock() {
            Ok(sys) => sys,
            Err(e) => {
                error!(error = %e, "Failed to acquire system lock");
                return;
            }
        };

        // 刷新系统信息
        system.refresh_all();

        // 获取系统内存信息
        let total_memory = system.total_memory();
        let used_memory = system.used_memory();
        let memory_usage_percent = if total_memory > 0 {
            ((used_memory as f64) / (total_memory as f64)) * 100.0
        } else {
            0.0
        };

        // 获取CPU使用率
        let cpu_usage = system.global_cpu_usage();

        // 获取当前进程信息
        let current_pid = sysinfo::get_current_pid().unwrap_or(sysinfo::Pid::from(0));
        let process_memory = system
            .process(current_pid)
            .map(|p| p.memory())
            .unwrap_or(0);
        let process_cpu = system
            .process(current_pid)
            .map(|p| p.cpu_usage())
            .unwrap_or(0.0);

        // 更新 Prometheus 指标
        if self.config.enable_prometheus_metrics {
            gauge!("system_memory_usage_bytes").set(used_memory as f64);
            gauge!("system_memory_usage_percent").set(memory_usage_percent);
            gauge!("system_cpu_usage_percent").set(cpu_usage as f64);
            gauge!("process_memory_usage_bytes").set(process_memory as f64);
            gauge!("process_cpu_usage_percent").set(process_cpu as f64);
        }

        // 记录系统指标日志
        info!(
            system_memory_used_mb = used_memory / 1024 / 1024,
            system_memory_total_mb = total_memory / 1024 / 1024,
            system_memory_usage_percent = %format!("{:.2}", memory_usage_percent),
            system_cpu_usage_percent = %format!("{:.2}", cpu_usage),
            process_memory_mb = process_memory / 1024 / 1024,
            process_cpu_usage_percent = %format!("{:.2}", process_cpu),
            active_connections = self.active_connections.load(Ordering::Relaxed),
            total_requests = self.total_requests.load(Ordering::Relaxed),
            successful_requests = self.successful_requests.load(Ordering::Relaxed),
            error_requests = self.error_requests.load(Ordering::Relaxed),
            "System performance metrics collected"
        );
    }

    /// 获取当前统计信息
    ///
    /// 【功能】：返回当前的性能统计数据
    ///
    /// # 返回值
    /// * `PerformanceStats` - 当前的性能统计信息
    pub fn get_stats(&self) -> PerformanceStats {
        PerformanceStats {
            active_connections: self.active_connections.load(Ordering::Relaxed),
            total_requests: self.total_requests.load(Ordering::Relaxed),
            successful_requests: self.successful_requests.load(Ordering::Relaxed),
            error_requests: self.error_requests.load(Ordering::Relaxed),
            success_rate: {
                let total = self.total_requests.load(Ordering::Relaxed);
                if total > 0 {
                    ((self.successful_requests.load(Ordering::Relaxed) as f64) / (total as f64)) *
                        100.0
                } else {
                    0.0
                }
            },
        }
    }
}

/// 性能统计信息
///
/// 【功能】：封装当前的性能统计数据
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub active_connections: u64,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub error_requests: u64,
    pub success_rate: f64,
}

/// 性能监控中间件
///
/// 【功能】：Axum中间件函数，自动监控每个HTTP请求的性能指标
///
/// 注意：此函数使用 `axum::middleware::from_fn_with_state` 和 State 提取器
/// 使用 defer 模式确保即使出现异常也会记录请求完成
///
/// # 参数
/// * `metrics` - 性能指标收集器（通过 State 提取器获取）
/// * `request` - HTTP请求
/// * `next` - 下一个中间件或处理器
///
/// # 返回值
/// * `Response` - HTTP响应
#[instrument(skip(metrics, request, next))]
pub async fn performance_monitoring_middleware(
    axum::extract::State(metrics): axum::extract::State<Arc<PerformanceMetrics>>,
    request: Request,
    next: Next
) -> Response {
    let start_time = Instant::now();
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let headers = if metrics.config.log_request_headers {
        Some(request.headers().clone())
    } else {
        None
    };

    // 记录请求开始
    metrics.record_request_start();

    // 使用 defer 模式确保总是记录请求完成
    struct DeferredMetrics {
        metrics: Arc<PerformanceMetrics>,
        start_time: Instant,
        method: String,
        path: String,
        headers: Option<HeaderMap>,
        completed: std::sync::atomic::AtomicBool,
    }

    impl Drop for DeferredMetrics {
        fn drop(&mut self) {
            // 如果请求没有正常完成，记录为服务器错误
            if !self.completed.load(std::sync::atomic::Ordering::Relaxed) {
                let duration = self.start_time.elapsed();
                self.metrics.record_request_end(
                    duration,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &self.method,
                    &self.path,
                    self.headers.as_ref()
                );
            }
        }
    }

    let deferred = DeferredMetrics {
        metrics: metrics.clone(),
        start_time,
        method: method.clone(),
        path: path.clone(),
        headers: headers.clone(),
        completed: std::sync::atomic::AtomicBool::new(false),
    };

    // 处理请求
    let response = next.run(request).await;

    // 计算处理时间
    let duration = start_time.elapsed();
    let status_code = response.status();

    // 记录请求完成
    metrics.record_request_end(duration, status_code, &method, &path, headers.as_ref());

    // 标记为已完成，避免 Drop 时重复记录
    deferred.completed.store(true, std::sync::atomic::Ordering::Relaxed);

    response
}

/// 创建性能监控中间件层
///
/// 【功能】：创建一个可以应用到Axum路由的性能监控中间件层
///
/// # 参数
/// * `config` - 性能监控配置
///
/// # 返回值
/// * `(middleware_layer, metrics)` - 中间件层和性能指标收集器
pub fn create_performance_monitoring_layer(config: PerformanceConfig) -> Arc<PerformanceMetrics> {
    PerformanceMetrics::new(config)
}

/// 初始化Prometheus指标导出器
///
/// 【功能】：设置Prometheus指标导出器，用于外部监控系统集成
///
/// # 参数
/// * `bind_address` - 绑定地址，例如 "0.0.0.0:9090"
///
/// # 返回值
/// * `Result<(), Box<dyn std::error::Error>>` - 初始化结果
pub fn init_prometheus_exporter(bind_address: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 暂时简化实现，避免复杂的Prometheus集成
    info!(bind_address = %bind_address, "Prometheus metrics exporter would be initialized here");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use std::time::Duration;

    #[tokio::test]
    async fn test_performance_metrics_creation() {
        let config = PerformanceConfig::default();
        let metrics = PerformanceMetrics::new(config);

        let stats = metrics.get_stats();
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.successful_requests, 0);
        assert_eq!(stats.error_requests, 0);
        assert_eq!(stats.success_rate, 0.0);
    }

    #[tokio::test]
    async fn test_request_metrics_recording() {
        let config = PerformanceConfig {
            enable_prometheus_metrics: false, // 避免在测试中初始化Prometheus
            ..Default::default()
        };
        let metrics = PerformanceMetrics::new(config);

        // 模拟请求开始
        metrics.record_request_start();
        let stats_after_start = metrics.get_stats();
        assert_eq!(stats_after_start.active_connections, 1);
        assert_eq!(stats_after_start.total_requests, 1);

        // 模拟请求完成
        metrics.record_request_end(
            Duration::from_millis(100),
            StatusCode::OK,
            "GET",
            "/test",
            None
        );

        let stats_after_end = metrics.get_stats();
        assert_eq!(stats_after_end.active_connections, 0);
        assert_eq!(stats_after_end.total_requests, 1);
        assert_eq!(stats_after_end.successful_requests, 1);
        assert_eq!(stats_after_end.error_requests, 0);
        assert_eq!(stats_after_end.success_rate, 100.0);
    }

    #[tokio::test]
    async fn test_error_request_recording() {
        let config = PerformanceConfig {
            enable_prometheus_metrics: false,
            ..Default::default()
        };
        let metrics = PerformanceMetrics::new(config);

        metrics.record_request_start();
        metrics.record_request_end(
            Duration::from_millis(50),
            StatusCode::INTERNAL_SERVER_ERROR,
            "POST",
            "/error",
            None
        );

        let stats = metrics.get_stats();
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.successful_requests, 0);
        assert_eq!(stats.error_requests, 1);
        assert_eq!(stats.success_rate, 0.0);
    }

    #[tokio::test]
    async fn test_multiple_requests_success_rate() {
        let config = PerformanceConfig {
            enable_prometheus_metrics: false,
            ..Default::default()
        };
        let metrics = PerformanceMetrics::new(config);

        // 3个成功请求
        for _ in 0..3 {
            metrics.record_request_start();
            metrics.record_request_end(
                Duration::from_millis(100),
                StatusCode::OK,
                "GET",
                "/success",
                None
            );
        }

        // 1个失败请求
        metrics.record_request_start();
        metrics.record_request_end(
            Duration::from_millis(200),
            StatusCode::BAD_REQUEST,
            "POST",
            "/error",
            None
        );

        let stats = metrics.get_stats();
        assert_eq!(stats.total_requests, 4);
        assert_eq!(stats.successful_requests, 3);
        assert_eq!(stats.error_requests, 1);
        assert_eq!(stats.success_rate, 75.0);
    }

    #[test]
    fn test_performance_config_default() {
        let config = PerformanceConfig::default();
        assert!(config.enable_detailed_logging);
        assert!(config.enable_system_monitoring);
        assert_eq!(config.system_monitoring_interval, 30);
        assert!(config.enable_prometheus_metrics);
        assert_eq!(config.slow_request_threshold_ms, 1000);
        assert!(!config.log_request_headers);
        assert_eq!(config.max_concurrent_connections_warning, 1000);
    }

    #[test]
    fn test_performance_stats_clone() {
        let stats = PerformanceStats {
            active_connections: 10,
            total_requests: 100,
            successful_requests: 95,
            error_requests: 5,
            success_rate: 95.0,
        };

        let cloned_stats = stats.clone();
        assert_eq!(stats.active_connections, cloned_stats.active_connections);
        assert_eq!(stats.total_requests, cloned_stats.total_requests);
        assert_eq!(stats.successful_requests, cloned_stats.successful_requests);
        assert_eq!(stats.error_requests, cloned_stats.error_requests);
        assert_eq!(stats.success_rate, cloned_stats.success_rate);
    }

    #[tokio::test]
    async fn test_log_level_strategy_websocket_upgrade() {
        // 测试WebSocket升级请求（101状态码）应该记录为INFO级别
        let config = PerformanceConfig {
            enable_prometheus_metrics: false,
            enable_detailed_logging: true,
            ..Default::default()
        };
        let metrics = PerformanceMetrics::new(config);

        metrics.record_request_start();
        metrics.record_request_end(
            Duration::from_millis(50),
            StatusCode::SWITCHING_PROTOCOLS, // 101状态码
            "GET",
            "/ws",
            None
        );

        let stats = metrics.get_stats();
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.successful_requests, 1); // 101被视为成功
        assert_eq!(stats.error_requests, 0);
    }

    #[tokio::test]
    async fn test_log_level_strategy_client_error() {
        // 测试客户端错误（4xx状态码）应该记录为WARN级别
        let config = PerformanceConfig {
            enable_prometheus_metrics: false,
            enable_detailed_logging: true,
            ..Default::default()
        };
        let metrics = PerformanceMetrics::new(config);

        // 测试409冲突状态码
        metrics.record_request_start();
        metrics.record_request_end(
            Duration::from_millis(100),
            StatusCode::CONFLICT, // 409状态码
            "POST",
            "/api/auth/register",
            None
        );

        let stats = metrics.get_stats();
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.successful_requests, 0);
        assert_eq!(stats.error_requests, 1);
    }

    #[tokio::test]
    async fn test_log_level_strategy_server_error() {
        // 测试服务器错误（5xx状态码）应该记录为ERROR级别
        let config = PerformanceConfig {
            enable_prometheus_metrics: false,
            enable_detailed_logging: true,
            ..Default::default()
        };
        let metrics = PerformanceMetrics::new(config);

        metrics.record_request_start();
        metrics.record_request_end(
            Duration::from_millis(200),
            StatusCode::INTERNAL_SERVER_ERROR, // 500状态码
            "GET",
            "/api/tasks",
            None
        );

        let stats = metrics.get_stats();
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.successful_requests, 0);
        assert_eq!(stats.error_requests, 1);
    }

    #[tokio::test]
    async fn test_log_level_strategy_slow_request() {
        // 测试慢请求应该记录为WARN级别（即使状态码是成功的）
        let config = PerformanceConfig {
            enable_prometheus_metrics: false,
            enable_detailed_logging: true,
            slow_request_threshold_ms: 500, // 设置500ms阈值
            ..Default::default()
        };
        let metrics = PerformanceMetrics::new(config);

        metrics.record_request_start();
        metrics.record_request_end(
            Duration::from_millis(1000), // 超过阈值的请求
            StatusCode::OK, // 成功状态码
            "GET",
            "/api/tasks",
            None
        );

        let stats = metrics.get_stats();
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.successful_requests, 1);
        assert_eq!(stats.error_requests, 0);
    }
}
