// src/app/utils/error_recovery.rs
//
// /-----------------------------------------------------------------------------\
// |                        【错误恢复机制核心模块】                             |
// |-----------------------------------------------------------------------------|
// | 基于 Axum 0.8.4 实现企业级错误恢复机制，包含：                             |
// | 1. 指数退避重试机制 (Exponential Backoff Retry)                            |
// | 2. 断路器模式 (Circuit Breaker Pattern)                                    |
// | 3. 优雅降级处理 (Graceful Degradation)                                     |
// | 4. 错误恢复策略配置                                                        |
// | 5. 恢复状态监控和指标收集                                                  |
// \-----------------------------------------------------------------------------/

use std::{ sync::{ Arc, RwLock }, time::{ Duration, Instant }, collections::HashMap, fmt::Debug };
use backoff::{ ExponentialBackoff, backoff::Backoff };
use serde::{ Serialize, Deserialize };
use tracing::{ error, warn, info, debug, instrument };
use crate::error::{ AppError, Result };

/// 错误类型分类
///
/// 【功能】：根据错误类型决定是否应该重试
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorCategory {
    /// 临时性错误，可以重试
    Transient,
    /// 永久性错误，不应重试
    Permanent,
    /// 限流错误，需要特殊处理
    RateLimit,
    /// 超时错误，可以重试但需要调整策略
    Timeout,
}

/// 错误恢复配置
///
/// 【功能】：定义错误恢复机制的各种配置参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecoveryConfig {
    /// 重试配置
    pub retry: RetryConfig,
    /// 断路器配置
    pub circuit_breaker: CircuitBreakerSettings,
    /// 降级配置
    pub degradation: DegradationConfig,
    /// 错误分类配置
    pub error_classification: ErrorClassificationConfig,
}

/// 错误分类配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorClassificationConfig {
    /// 临时性错误的HTTP状态码列表
    pub transient_status_codes: Vec<u16>,
    /// 永久性错误的HTTP状态码列表
    pub permanent_status_codes: Vec<u16>,
    /// 限流错误的HTTP状态码列表
    pub rate_limit_status_codes: Vec<u16>,
    /// 超时错误的HTTP状态码列表
    pub timeout_status_codes: Vec<u16>,
}

/// 重试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 初始延迟时间（毫秒）
    pub initial_delay_ms: u64,
    /// 最大延迟时间（毫秒）
    pub max_delay_ms: u64,
    /// 退避倍数
    pub multiplier: f64,
    /// 随机化因子
    pub randomization_factor: f64,
    /// 是否对不同错误类型使用不同的重试策略
    pub use_smart_retry: bool,
    /// 超时错误的最大重试次数
    pub timeout_max_retries: u32,
    /// 限流错误的最大重试次数
    pub rate_limit_max_retries: u32,
    /// 限流错误的初始延迟时间（毫秒）
    pub rate_limit_initial_delay_ms: u64,
    /// 限流错误的最大延迟时间（毫秒）
    pub rate_limit_max_delay_ms: u64,
}

/// 断路器设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerSettings {
    /// 失败阈值
    pub failure_threshold: u32,
    /// 恢复超时时间（秒）
    pub recovery_timeout_secs: u64,
    /// 请求超时时间（秒）
    pub request_timeout_secs: u64,
}

/// 降级配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradationConfig {
    /// 是否启用降级
    pub enabled: bool,
    /// 降级响应缓存时间（秒）
    pub cache_duration_secs: u64,
    /// 默认降级响应
    pub default_response: String,
}

/// 错误恢复状态
#[derive(Debug, Clone, Serialize)]
pub struct RecoveryStatus {
    /// 服务名称
    pub service_name: String,
    /// 重试统计
    pub retry_stats: RetryStats,
    /// 断路器状态
    pub circuit_breaker_state: String,
    /// 降级状态
    pub degradation_active: bool,
    /// 最后更新时间
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// 重试统计
#[derive(Debug, Clone, Serialize)]
pub struct RetryStats {
    /// 总重试次数
    pub total_retries: u64,
    /// 成功重试次数
    pub successful_retries: u64,
    /// 失败重试次数
    pub failed_retries: u64,
    /// 平均重试延迟（毫秒）
    pub avg_retry_delay_ms: f64,
}

impl Default for ErrorRecoveryConfig {
    fn default() -> Self {
        Self {
            retry: RetryConfig {
                max_retries: 3,
                initial_delay_ms: 100,
                max_delay_ms: 5000,
                multiplier: 2.0,
                randomization_factor: 0.1,
                use_smart_retry: true,
                timeout_max_retries: 2,
                rate_limit_max_retries: 5,
                rate_limit_initial_delay_ms: 1000,
                rate_limit_max_delay_ms: 30000,
            },
            circuit_breaker: CircuitBreakerSettings {
                failure_threshold: 5,
                recovery_timeout_secs: 30,
                request_timeout_secs: 10,
            },
            degradation: DegradationConfig {
                enabled: true,
                cache_duration_secs: 300,
                default_response: "服务暂时不可用，请稍后重试".to_string(),
            },
            error_classification: ErrorClassificationConfig {
                transient_status_codes: vec![500, 502, 503, 504],
                permanent_status_codes: vec![400, 401, 403, 404, 422],
                rate_limit_status_codes: vec![429],
                timeout_status_codes: vec![408, 504],
            },
        }
    }
}

impl Default for RetryStats {
    fn default() -> Self {
        Self {
            total_retries: 0,
            successful_retries: 0,
            failed_retries: 0,
            avg_retry_delay_ms: 0.0,
        }
    }
}

impl ErrorClassificationConfig {
    /// 根据HTTP状态码分类错误
    ///
    /// # 参数
    /// * `status_code` - HTTP状态码
    ///
    /// # 返回值
    /// * `ErrorCategory` - 错误分类
    pub fn classify_error(&self, status_code: u16) -> ErrorCategory {
        if self.rate_limit_status_codes.contains(&status_code) {
            ErrorCategory::RateLimit
        } else if self.timeout_status_codes.contains(&status_code) {
            ErrorCategory::Timeout
        } else if self.transient_status_codes.contains(&status_code) {
            ErrorCategory::Transient
        } else if self.permanent_status_codes.contains(&status_code) {
            ErrorCategory::Permanent
        } else {
            // 默认情况下，5xx错误被认为是临时性的，4xx错误被认为是永久性的
            if status_code >= 500 {
                ErrorCategory::Transient
            } else if status_code >= 400 {
                ErrorCategory::Permanent
            } else {
                ErrorCategory::Transient
            }
        }
    }

    /// 判断错误是否应该重试
    ///
    /// # 参数
    /// * `status_code` - HTTP状态码
    ///
    /// # 返回值
    /// * `bool` - 是否应该重试
    pub fn should_retry(&self, status_code: u16) -> bool {
        match self.classify_error(status_code) {
            ErrorCategory::Transient | ErrorCategory::RateLimit | ErrorCategory::Timeout => true,
            ErrorCategory::Permanent => false,
        }
    }
}

/// 简单的断路器状态
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// 简单的断路器实现
#[derive(Debug)]
pub struct SimpleCircuitBreaker {
    failure_count: Arc<RwLock<u32>>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    state: Arc<RwLock<CircuitBreakerState>>,
    config: CircuitBreakerSettings,
}

impl SimpleCircuitBreaker {
    pub fn new(config: CircuitBreakerSettings) -> Self {
        Self {
            failure_count: Arc::new(RwLock::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            state: Arc::new(RwLock::new(CircuitBreakerState::Closed)),
            config,
        }
    }

    pub fn state(&self) -> CircuitBreakerState {
        self.state.read().unwrap().clone()
    }

    pub async fn call<F, T, Fut>(&self, operation: F) -> Result<T>
        where F: FnOnce() -> Fut, Fut: std::future::Future<Output = Result<T>>
    {
        // 检查断路器状态
        let current_state = self.state();

        match current_state {
            CircuitBreakerState::Open => {
                // 断路器开启状态：检查是否可以尝试恢复
                if let Some(last_failure) = *self.last_failure_time.read().unwrap() {
                    if
                        last_failure.elapsed() >
                        Duration::from_secs(self.config.recovery_timeout_secs)
                    {
                        // 转换到半开状态，允许一个请求通过进行测试
                        *self.state.write().unwrap() = CircuitBreakerState::HalfOpen;
                        debug!("断路器从开启状态转换到半开状态，允许测试请求");
                    } else {
                        // 仍在恢复超时期间，拒绝请求
                        return Err(
                            AppError::with_span_trace(
                                "断路器开启，请求被拒绝".to_string(),
                                axum::http::StatusCode::SERVICE_UNAVAILABLE
                            )
                        );
                    }
                } else {
                    // 如果没有记录失败时间但状态为开启，这是异常情况，直接拒绝请求
                    return Err(
                        AppError::with_span_trace(
                            "断路器开启，请求被拒绝".to_string(),
                            axum::http::StatusCode::SERVICE_UNAVAILABLE
                        )
                    );
                }
            }
            CircuitBreakerState::HalfOpen => {
                // 半开状态：允许一个请求通过进行测试
                debug!("断路器处于半开状态，允许测试请求通过");
            }
            CircuitBreakerState::Closed => {
                // 正常状态：允许所有请求通过
                debug!("断路器处于关闭状态，允许请求通过");
            }
        }

        // 执行操作
        match operation().await {
            Ok(result) => {
                // 成功时重置失败计数并更新状态
                *self.failure_count.write().unwrap() = 0;
                if self.state() == CircuitBreakerState::HalfOpen {
                    // 从半开状态转换回关闭状态
                    *self.state.write().unwrap() = CircuitBreakerState::Closed;
                    debug!("断路器从半开状态转换回关闭状态");
                }
                Ok(result)
            }
            Err(err) => {
                // 失败时增加失败计数并记录失败时间
                let mut count = self.failure_count.write().unwrap();
                *count += 1;
                *self.last_failure_time.write().unwrap() = Some(Instant::now());

                // 检查是否达到失败阈值，需要开启断路器
                if *count >= self.config.failure_threshold {
                    *self.state.write().unwrap() = CircuitBreakerState::Open;
                    debug!(
                        failure_count = *count,
                        threshold = self.config.failure_threshold,
                        "断路器达到失败阈值，转换到开启状态"
                    );
                }

                Err(err)
            }
        }
    }
}

/// 错误恢复管理器
///
/// 【功能】：统一管理所有错误恢复机制
#[derive(Debug)]
pub struct ErrorRecoveryManager {
    /// 配置
    config: ErrorRecoveryConfig,
    /// 断路器集合（按服务名称索引）
    circuit_breakers: Arc<RwLock<HashMap<String, Arc<SimpleCircuitBreaker>>>>,
    /// 恢复状态统计
    recovery_stats: Arc<RwLock<HashMap<String, RecoveryStatus>>>,
    /// 降级缓存
    degradation_cache: Arc<RwLock<HashMap<String, (String, Instant)>>>,
}

impl ErrorRecoveryManager {
    /// 创建新的错误恢复管理器
    ///
    /// # 参数
    /// * `config` - 错误恢复配置
    ///
    /// # 返回值
    /// * `ErrorRecoveryManager` - 错误恢复管理器实例
    pub fn new(config: ErrorRecoveryConfig) -> Self {
        Self {
            config,
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            recovery_stats: Arc::new(RwLock::new(HashMap::new())),
            degradation_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 使用默认配置创建错误恢复管理器
    pub fn with_default_config() -> Self {
        Self::new(ErrorRecoveryConfig::default())
    }

    /// 从AppError中提取HTTP状态码
    ///
    /// # 参数
    /// * `error` - 应用错误
    ///
    /// # 返回值
    /// * `Option<u16>` - HTTP状态码
    fn extract_status_code(&self, error: &AppError) -> Option<u16> {
        match error {
            AppError::TracedError { status_code, .. } => Some(status_code.as_u16()),
            AppError::InvalidToken(_) => Some(401),
            AppError::InvalidCredentials => Some(401),
            AppError::UserAlreadyExists(_) => Some(409),
            AppError::TaskNotFound(_) => Some(404),
            AppError::BadRequest(_) => Some(400),
            AppError::DbErr(_) => Some(500),
            AppError::PasswordHashError(_) => Some(500),
            AppError::TokenGenerationError(_) => Some(500),
        }
    }

    /// 根据错误类型选择重试策略
    ///
    /// # 参数
    /// * `error` - 应用错误
    ///
    /// # 返回值
    /// * `(u32, u64, u64)` - (最大重试次数, 初始延迟, 最大延迟)
    fn get_retry_strategy(&self, error: &AppError) -> (u32, u64, u64) {
        if !self.config.retry.use_smart_retry {
            return (
                self.config.retry.max_retries,
                self.config.retry.initial_delay_ms,
                self.config.retry.max_delay_ms,
            );
        }

        if let Some(status_code) = self.extract_status_code(error) {
            match self.config.error_classification.classify_error(status_code) {
                ErrorCategory::RateLimit =>
                    (
                        self.config.retry.rate_limit_max_retries,
                        self.config.retry.rate_limit_initial_delay_ms,
                        self.config.retry.rate_limit_max_delay_ms,
                    ),
                ErrorCategory::Timeout =>
                    (
                        self.config.retry.timeout_max_retries,
                        self.config.retry.initial_delay_ms,
                        self.config.retry.max_delay_ms,
                    ),
                ErrorCategory::Transient =>
                    (
                        self.config.retry.max_retries,
                        self.config.retry.initial_delay_ms,
                        self.config.retry.max_delay_ms,
                    ),
                ErrorCategory::Permanent => (0, 0, 0), // 不重试永久性错误
            }
        } else {
            // 默认策略
            (
                self.config.retry.max_retries,
                self.config.retry.initial_delay_ms,
                self.config.retry.max_delay_ms,
            )
        }
    }

    /// 获取或创建断路器
    ///
    /// # 参数
    /// * `service_name` - 服务名称
    ///
    /// # 返回值
    /// * `SimpleCircuitBreaker` - 断路器实例
    fn get_or_create_circuit_breaker(&self, service_name: &str) -> Arc<SimpleCircuitBreaker> {
        let mut breakers = self.circuit_breakers.write().unwrap();

        if let Some(breaker) = breakers.get(service_name) {
            // 返回已存在的断路器实例
            breaker.clone()
        } else {
            // 创建新的断路器实例并存储
            let breaker = Arc::new(SimpleCircuitBreaker::new(self.config.circuit_breaker.clone()));
            breakers.insert(service_name.to_string(), breaker.clone());
            breaker
        }
    }

    /// 更新恢复状态统计
    ///
    /// # 参数
    /// * `service_name` - 服务名称
    /// * `retry_count` - 重试次数
    /// * `success` - 是否成功
    /// * `delay_ms` - 延迟时间（毫秒）
    fn update_recovery_stats(
        &self,
        service_name: &str,
        retry_count: u32,
        success: bool,
        delay_ms: u64
    ) {
        let mut stats = self.recovery_stats.write().unwrap();
        let entry = stats.entry(service_name.to_string()).or_insert_with(|| RecoveryStatus {
            service_name: service_name.to_string(),
            retry_stats: RetryStats::default(),
            circuit_breaker_state: "CLOSED".to_string(),
            degradation_active: false,
            last_updated: chrono::Utc::now(),
        });

        entry.retry_stats.total_retries += retry_count as u64;
        if success {
            entry.retry_stats.successful_retries += 1;
        } else {
            entry.retry_stats.failed_retries += 1;
        }

        // 更新平均延迟
        let total_operations =
            entry.retry_stats.successful_retries + entry.retry_stats.failed_retries;
        if total_operations > 0 {
            entry.retry_stats.avg_retry_delay_ms =
                (entry.retry_stats.avg_retry_delay_ms * ((total_operations - 1) as f64) +
                    (delay_ms as f64)) /
                (total_operations as f64);
        }

        entry.last_updated = chrono::Utc::now();
    }

    /// 执行带重试的操作
    ///
    /// 【功能】：使用智能指数退避策略执行可能失败的操作
    ///
    /// # 参数
    /// * `service_name` - 服务名称
    /// * `operation` - 要执行的操作
    ///
    /// # 返回值
    /// * `Result<T>` - 操作结果
    #[instrument(skip(self, operation), fields(service_name = %service_name))]
    pub async fn execute_with_retry<T, F, Fut>(&self, service_name: &str, operation: F) -> Result<T>
        where F: Fn() -> Fut, Fut: std::future::Future<Output = Result<T>>, T: Clone
    {
        let mut retry_count = 0;
        let start_time = Instant::now();
        let mut current_strategy = (
            self.config.retry.max_retries,
            self.config.retry.initial_delay_ms,
            self.config.retry.max_delay_ms,
        );

        loop {
            match operation().await {
                Ok(result) => {
                    let elapsed = start_time.elapsed().as_millis() as u64;
                    self.update_recovery_stats(service_name, retry_count, true, elapsed);

                    if retry_count > 0 {
                        info!(
                            service_name = %service_name,
                            retry_count = retry_count,
                            elapsed_ms = elapsed,
                            "Operation succeeded after retries"
                        );
                    }

                    return Ok(result);
                }
                Err(err) => {
                    // 根据错误类型调整重试策略
                    if retry_count == 0 {
                        current_strategy = self.get_retry_strategy(&err);
                        debug!(
                            service_name = %service_name,
                            max_retries = current_strategy.0,
                            initial_delay_ms = current_strategy.1,
                            max_delay_ms = current_strategy.2,
                            error_type = ?self.extract_status_code(&err),
                            "Selected retry strategy based on error type"
                        );
                    }

                    retry_count += 1;

                    // 检查是否应该重试
                    if current_strategy.0 == 0 || retry_count > current_strategy.0 {
                        let elapsed = start_time.elapsed().as_millis() as u64;
                        self.update_recovery_stats(service_name, retry_count, false, elapsed);

                        if current_strategy.0 == 0 {
                            debug!(
                                service_name = %service_name,
                                error = ?err,
                                "Permanent error detected, not retrying"
                            );
                        } else {
                            error!(
                                service_name = %service_name,
                                retry_count = retry_count,
                                error = ?err,
                                "Operation failed after maximum retries"
                            );
                        }

                        return Err(err);
                    }

                    // 计算延迟时间
                    let mut backoff = ExponentialBackoff {
                        initial_interval: Duration::from_millis(current_strategy.1),
                        max_interval: Duration::from_millis(current_strategy.2),
                        multiplier: self.config.retry.multiplier,
                        randomization_factor: self.config.retry.randomization_factor,
                        max_elapsed_time: None,
                        ..Default::default()
                    };

                    // 跳过前面的退避步骤
                    for _ in 1..retry_count {
                        backoff.next_backoff();
                    }

                    if let Some(delay) = backoff.next_backoff() {
                        warn!(
                            service_name = %service_name,
                            retry_count = retry_count,
                            delay_ms = delay.as_millis(),
                            error = ?err,
                            error_category = ?self.extract_status_code(&err).map(|code|
                                self.config.error_classification.classify_error(code)),
                            "Operation failed, retrying after delay"
                        );

                        tokio::time::sleep(delay).await;
                    } else {
                        let elapsed = start_time.elapsed().as_millis() as u64;
                        self.update_recovery_stats(service_name, retry_count, false, elapsed);

                        error!(
                            service_name = %service_name,
                            retry_count = retry_count,
                            error = ?err,
                            "Backoff exhausted, operation failed"
                        );

                        return Err(err);
                    }
                }
            }
        }
    }

    /// 执行带断路器保护的操作
    ///
    /// 【功能】：使用断路器模式保护服务调用
    ///
    /// # 参数
    /// * `service_name` - 服务名称
    /// * `operation` - 要执行的操作
    ///
    /// # 返回值
    /// * `Result<T>` - 操作结果
    #[instrument(skip(self, operation), fields(service_name = %service_name))]
    pub async fn execute_with_circuit_breaker<T, F, Fut>(
        &self,
        service_name: &str,
        operation: F
    )
        -> Result<T>
        where F: Fn() -> Fut, Fut: std::future::Future<Output = Result<T>>, T: Clone
    {
        let breaker = self.get_or_create_circuit_breaker(service_name);

        // 更新断路器状态统计
        {
            let mut stats = self.recovery_stats.write().unwrap();
            let entry = stats.entry(service_name.to_string()).or_insert_with(|| RecoveryStatus {
                service_name: service_name.to_string(),
                retry_stats: RetryStats::default(),
                circuit_breaker_state: "CLOSED".to_string(),
                degradation_active: false,
                last_updated: chrono::Utc::now(),
            });

            entry.circuit_breaker_state = format!("{:?}", breaker.state());
            entry.last_updated = chrono::Utc::now();
        }

        // 使用我们自己的断路器实现
        match breaker.call(operation).await {
            Ok(result) => {
                debug!(
                    service_name = %service_name,
                    circuit_breaker_state = ?breaker.state(),
                    "Circuit breaker operation succeeded"
                );
                Ok(result)
            }
            Err(err) => {
                // 检查是否是断路器拒绝的错误
                if err.to_string().contains("断路器开启") {
                    warn!(
                        service_name = %service_name,
                        "Circuit breaker is open, operation rejected"
                    );

                    // 尝试降级处理
                    if self.config.degradation.enabled {
                        // 执行降级处理（主要是为了记录状态和缓存）
                        let _ = self.execute_degradation(service_name).await;
                    }
                    Err(err)
                } else {
                    error!(
                        service_name = %service_name,
                        error = ?err,
                        "Circuit breaker operation failed"
                    );
                    Err(err)
                }
            }
        }
    }

    /// 执行降级处理
    ///
    /// 【功能】：当主要服务不可用时，提供降级响应
    ///
    /// # 参数
    /// * `service_name` - 服务名称
    ///
    /// # 返回值
    /// * `Result<String>` - 降级响应结果
    #[instrument(skip(self), fields(service_name = %service_name))]
    async fn execute_degradation(&self, service_name: &str) -> Result<String> {
        // 检查降级缓存
        {
            let cache = self.degradation_cache.read().unwrap();
            if let Some((cached_response, cached_time)) = cache.get(service_name) {
                let cache_duration = Duration::from_secs(
                    self.config.degradation.cache_duration_secs
                );
                if cached_time.elapsed() < cache_duration {
                    debug!(
                        service_name = %service_name,
                        "Returning cached degradation response"
                    );

                    // 直接返回缓存的响应
                    return Ok(cached_response.clone());
                }
            }
        }

        // 更新降级状态
        {
            let mut stats = self.recovery_stats.write().unwrap();
            let entry = stats.entry(service_name.to_string()).or_insert_with(|| RecoveryStatus {
                service_name: service_name.to_string(),
                retry_stats: RetryStats::default(),
                circuit_breaker_state: "OPEN".to_string(),
                degradation_active: false,
                last_updated: chrono::Utc::now(),
            });

            entry.degradation_active = true;
            entry.last_updated = chrono::Utc::now();
        }

        warn!(
            service_name = %service_name,
            "Executing degradation for service"
        );

        // 生成降级响应
        let degradation_response = self.config.degradation.default_response.clone();

        // 缓存降级响应
        {
            let mut cache = self.degradation_cache.write().unwrap();
            cache.insert(service_name.to_string(), (degradation_response.clone(), Instant::now()));
        }

        // 返回错误，因为我们无法生成有效的 T 类型响应
        Err(
            AppError::with_span_trace(
                degradation_response,
                axum::http::StatusCode::SERVICE_UNAVAILABLE
            )
        )
    }

    /// 执行完整的错误恢复策略（重试 + 断路器 + 降级）
    ///
    /// 【功能】：组合使用所有错误恢复机制
    ///
    /// # 参数
    /// * `service_name` - 服务名称
    /// * `operation` - 要执行的操作
    ///
    /// # 返回值
    /// * `Result<T>` - 操作结果
    #[instrument(skip(self, operation), fields(service_name = %service_name))]
    pub async fn execute_with_full_recovery<T, F, Fut>(
        &self,
        service_name: &str,
        operation: F
    )
        -> Result<T>
        where F: Fn() -> Fut + Clone, Fut: std::future::Future<Output = Result<T>>, T: Clone
    {
        info!(
            service_name = %service_name,
            "Executing operation with full error recovery"
        );

        // 首先尝试断路器保护的重试操作
        let retry_operation = {
            let op = operation.clone();
            move || {
                let op_inner = op.clone();
                async move { op_inner().await }
            }
        };

        match
            self.execute_with_circuit_breaker(service_name, || {
                self.execute_with_retry(service_name, retry_operation.clone())
            }).await
        {
            Ok(result) => {
                // 成功时清除降级状态
                {
                    let mut stats = self.recovery_stats.write().unwrap();
                    if let Some(entry) = stats.get_mut(service_name) {
                        entry.degradation_active = false;
                        entry.last_updated = chrono::Utc::now();
                    }
                }
                Ok(result)
            }
            Err(err) => {
                // 如果启用了降级，记录降级状态但仍返回原错误
                if self.config.degradation.enabled {
                    warn!(
                        service_name = %service_name,
                        error = ?err,
                        "Primary operation failed, degradation enabled but returning original error"
                    );

                    // 执行降级处理（主要是为了记录状态和缓存）
                    let _ = self.execute_degradation(service_name).await;
                }
                Err(err)
            }
        }
    }

    /// 获取恢复状态统计
    ///
    /// # 返回值
    /// * `HashMap<String, RecoveryStatus>` - 所有服务的恢复状态
    pub fn get_recovery_stats(&self) -> HashMap<String, RecoveryStatus> {
        self.recovery_stats.read().unwrap().clone()
    }

    /// 获取特定服务的恢复状态
    ///
    /// # 参数
    /// * `service_name` - 服务名称
    ///
    /// # 返回值
    /// * `Option<RecoveryStatus>` - 服务的恢复状态
    pub fn get_service_recovery_status(&self, service_name: &str) -> Option<RecoveryStatus> {
        self.recovery_stats.read().unwrap().get(service_name).cloned()
    }

    /// 重置服务的恢复状态
    ///
    /// # 参数
    /// * `service_name` - 服务名称
    pub fn reset_service_recovery(&self, service_name: &str) {
        let mut stats = self.recovery_stats.write().unwrap();
        stats.remove(service_name);

        let mut breakers = self.circuit_breakers.write().unwrap();
        breakers.remove(service_name);

        let mut cache = self.degradation_cache.write().unwrap();
        cache.remove(service_name);

        info!(
            service_name = %service_name,
            "Reset recovery state for service"
        );
    }

    /// 清理过期的降级缓存
    pub fn cleanup_expired_cache(&self) {
        let mut cache = self.degradation_cache.write().unwrap();
        let cache_duration = Duration::from_secs(self.config.degradation.cache_duration_secs);

        cache.retain(|service_name, (_, cached_time)| {
            let is_valid = cached_time.elapsed() < cache_duration;
            if !is_valid {
                debug!(
                    service_name = %service_name,
                    "Removing expired degradation cache entry"
                );
            }
            is_valid
        });
    }

    /// 获取错误恢复配置（用于测试）
    #[cfg(test)]
    pub fn get_config(&self) -> &ErrorRecoveryConfig {
        &self.config
    }

    /// 公开降级执行方法（用于测试）
    #[cfg(test)]
    pub async fn test_execute_degradation(&self, service_name: &str) -> Result<String> {
        self.execute_degradation(service_name).await
    }
}

// 单元测试模块
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{ Arc, atomic::{ AtomicU32, Ordering } };

    /// 创建测试用的错误恢复配置
    fn create_test_config() -> ErrorRecoveryConfig {
        ErrorRecoveryConfig {
            retry: RetryConfig {
                max_retries: 3,
                initial_delay_ms: 10, // 减少测试时间
                max_delay_ms: 100, // 减少测试时间
                multiplier: 2.0,
                randomization_factor: 0.1,
                use_smart_retry: true,
                timeout_max_retries: 2,
                rate_limit_max_retries: 5,
                rate_limit_initial_delay_ms: 50,
                rate_limit_max_delay_ms: 500,
            },
            circuit_breaker: CircuitBreakerSettings {
                failure_threshold: 2, // 降低阈值以便测试
                recovery_timeout_secs: 1, // 减少测试时间
                request_timeout_secs: 1,
            },
            degradation: DegradationConfig {
                enabled: true,
                cache_duration_secs: 5,
                default_response: "测试降级响应".to_string(),
            },
            error_classification: ErrorClassificationConfig {
                transient_status_codes: vec![500, 502, 503, 504],
                permanent_status_codes: vec![400, 401, 403, 404, 422],
                rate_limit_status_codes: vec![429],
                timeout_status_codes: vec![408, 504],
            },
        }
    }

    /// 测试错误恢复管理器的创建
    #[tokio::test]
    async fn test_error_recovery_manager_creation() {
        let config = create_test_config();
        let manager = ErrorRecoveryManager::new(config.clone());

        // 验证管理器创建成功
        assert_eq!(manager.get_config().retry.max_retries, 3);
        assert_eq!(manager.get_config().circuit_breaker.failure_threshold, 2);
        assert!(manager.get_config().degradation.enabled);
    }

    /// 测试默认配置的错误恢复管理器
    #[tokio::test]
    async fn test_error_recovery_manager_default() {
        let manager = ErrorRecoveryManager::with_default_config();

        // 验证默认配置
        assert_eq!(manager.get_config().retry.max_retries, 3);
        assert_eq!(manager.get_config().circuit_breaker.failure_threshold, 5);
        assert!(manager.get_config().degradation.enabled);
    }

    /// 测试成功的重试操作
    #[tokio::test]
    async fn test_retry_success() {
        let manager = ErrorRecoveryManager::new(create_test_config());
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = manager.execute_with_retry("test_service", || {
            let count = call_count_clone.clone();
            async move {
                let current = count.fetch_add(1, Ordering::SeqCst);
                if current < 2 {
                    // 前两次调用失败
                    Err(
                        AppError::with_span_trace(
                            "测试错误".to_string(),
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR
                        )
                    )
                } else {
                    // 第三次调用成功
                    Ok("成功".to_string())
                }
            }
        }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "成功");
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    /// 测试重试失败（超过最大重试次数）
    #[tokio::test]
    async fn test_retry_failure() {
        let manager = ErrorRecoveryManager::new(create_test_config());
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result: Result<String> = manager.execute_with_retry("test_service", || {
            let count = call_count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                // 总是失败
                Err(
                    AppError::with_span_trace(
                        "持续错误".to_string(),
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR
                    )
                )
            }
        }).await;

        assert!(result.is_err());
        // 应该调用 1 + max_retries 次
        assert_eq!(call_count.load(Ordering::SeqCst), 4);
    }

    /// 测试断路器的正常状态
    #[tokio::test]
    async fn test_circuit_breaker_closed() {
        let manager = ErrorRecoveryManager::new(create_test_config());
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = manager.execute_with_circuit_breaker("test_service", || {
            let count = call_count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok("成功".to_string())
            }
        }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "成功");
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    /// 测试断路器开启状态
    #[tokio::test]
    async fn test_circuit_breaker_open() {
        let manager = ErrorRecoveryManager::new(create_test_config());
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        // 先触发足够的失败来开启断路器（失败阈值是2）
        for _ in 0..2 {
            let _: Result<String> = manager.execute_with_circuit_breaker("test_service", || {
                let count = call_count_clone.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Err(
                        AppError::with_span_trace(
                            "测试错误".to_string(),
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR
                        )
                    )
                }
            }).await;
        }

        // 现在断路器应该是开启的，后续调用应该被拒绝
        let result: Result<String> = manager.execute_with_circuit_breaker("test_service", || {
            let count = call_count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok("不应该被调用".to_string())
            }
        }).await;

        // 断路器开启时应该返回错误
        assert!(result.is_err());
        // 验证操作没有被实际调用（断路器拒绝了请求）
        // 由于断路器已开启，最后一次调用不应该增加计数
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    /// 测试错误恢复状态统计
    #[tokio::test]
    async fn test_recovery_stats() {
        let manager = ErrorRecoveryManager::new(create_test_config());

        // 执行一些操作来生成统计数据
        let _: Result<String> = manager.execute_with_retry("service1", || {
            async move { Ok("成功".to_string()) }
        }).await;

        let _: Result<String> = manager.execute_with_retry("service2", || {
            async move {
                Err(
                    AppError::with_span_trace(
                        "测试错误".to_string(),
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR
                    )
                )
            }
        }).await;

        // 获取统计数据
        let stats = manager.get_recovery_stats();

        // 验证统计数据
        assert!(stats.contains_key("service1"));
        assert!(stats.contains_key("service2"));

        let service1_stats = stats.get("service1").unwrap();
        assert_eq!(service1_stats.service_name, "service1");
        assert!(service1_stats.retry_stats.successful_retries > 0);
    }

    /// 测试服务恢复状态重置
    #[tokio::test]
    async fn test_reset_service_recovery() {
        let manager = ErrorRecoveryManager::new(create_test_config());

        // 先生成一些统计数据
        let _: Result<String> = manager.execute_with_retry("test_service", || {
            async move { Ok("成功".to_string()) }
        }).await;

        // 验证统计数据存在
        let stats_before = manager.get_recovery_stats();
        assert!(stats_before.contains_key("test_service"));

        // 重置服务恢复状态
        manager.reset_service_recovery("test_service");

        // 验证统计数据被清除
        let stats_after = manager.get_recovery_stats();
        assert!(!stats_after.contains_key("test_service"));
    }

    /// 测试降级缓存清理
    #[tokio::test]
    async fn test_cleanup_expired_cache() {
        let manager = ErrorRecoveryManager::new(create_test_config());

        // 执行降级操作来创建缓存
        let _ = manager.test_execute_degradation("test_service").await;

        // 清理过期缓存
        manager.cleanup_expired_cache();

        // 由于缓存时间很短，这个测试主要验证方法不会崩溃
        // 在实际应用中，可能需要更复杂的测试来验证缓存清理逻辑
    }

    /// 测试完整错误恢复流程
    #[tokio::test]
    async fn test_full_error_recovery() {
        let manager = ErrorRecoveryManager::new(create_test_config());
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result: Result<String> = manager.execute_with_full_recovery("test_service", || {
            let count = call_count_clone.clone();
            async move {
                let current = count.fetch_add(1, Ordering::SeqCst);
                if current < 1 {
                    // 第一次调用失败
                    Err(
                        AppError::with_span_trace(
                            "测试错误".to_string(),
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR
                        )
                    )
                } else {
                    // 第二次调用成功
                    Ok("成功".to_string())
                }
            }
        }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "成功");
    }
}
