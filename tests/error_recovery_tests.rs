//! 错误恢复机制测试
//!
//! 这个模块测试错误恢复机制的各种功能，包括：
//! - 重试配置测试
//! - 断路器配置测试
//! - 错误分类测试
//! - 错误恢复管理器测试

use axum_tutorial::{
    app::utils::error_recovery::{
        RetryConfig,
        CircuitBreakerSettings,
        ErrorRecoveryManager,
        ErrorRecoveryConfig,
        ErrorClassificationConfig,
        DegradationConfig,
        ErrorCategory,
        SimpleCircuitBreaker,
        CircuitBreakerState,
    },
};

use tracing_test::traced_test;

/// 创建测试用的错误恢复配置
fn create_test_config() -> ErrorRecoveryConfig {
    ErrorRecoveryConfig {
        retry: RetryConfig {
            max_retries: 3,
            initial_delay_ms: 10, // 缩短测试时间
            max_delay_ms: 100,
            multiplier: 2.0,
            randomization_factor: 0.1,
            use_smart_retry: true,
            timeout_max_retries: 2,
            rate_limit_max_retries: 5,
            rate_limit_initial_delay_ms: 50,
            rate_limit_max_delay_ms: 500,
        },
        circuit_breaker: CircuitBreakerSettings {
            failure_threshold: 2,
            recovery_timeout_secs: 1,
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

/// 测试重试配置
#[cfg(test)]
mod retry_config_tests {
    use super::*;

    #[test]
    fn test_retry_config_defaults() {
        let config = ErrorRecoveryConfig::default();

        assert_eq!(config.retry.max_retries, 3);
        assert_eq!(config.retry.initial_delay_ms, 100);
        assert_eq!(config.retry.max_delay_ms, 5000);
        assert_eq!(config.retry.multiplier, 2.0);
        assert_eq!(config.retry.randomization_factor, 0.1);
    }

    #[test]
    fn test_retry_config_custom() {
        let config = RetryConfig {
            max_retries: 5,
            initial_delay_ms: 200,
            max_delay_ms: 120000,
            multiplier: 1.5,
            randomization_factor: 0.2,
            use_smart_retry: true,
            timeout_max_retries: 3,
            rate_limit_max_retries: 10,
            rate_limit_initial_delay_ms: 1000,
            rate_limit_max_delay_ms: 60000,
        };

        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_delay_ms, 200);
        assert_eq!(config.max_delay_ms, 120000);
        assert_eq!(config.multiplier, 1.5);
        assert_eq!(config.randomization_factor, 0.2);
    }
}

/// 测试断路器配置
#[cfg(test)]
mod circuit_breaker_config_tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_config_defaults() {
        let config = ErrorRecoveryConfig::default();

        assert_eq!(config.circuit_breaker.failure_threshold, 5);
        assert_eq!(config.circuit_breaker.recovery_timeout_secs, 30);
        assert_eq!(config.circuit_breaker.request_timeout_secs, 10);
    }

    #[test]
    fn test_circuit_breaker_config_custom() {
        let config = CircuitBreakerSettings {
            failure_threshold: 10,
            recovery_timeout_secs: 120,
            request_timeout_secs: 5,
        };

        assert_eq!(config.failure_threshold, 10);
        assert_eq!(config.recovery_timeout_secs, 120);
        assert_eq!(config.request_timeout_secs, 5);
    }

    #[test]
    fn test_create_circuit_breaker() {
        let config = CircuitBreakerSettings {
            failure_threshold: 3,
            recovery_timeout_secs: 30,
            request_timeout_secs: 10,
        };
        let circuit_breaker = SimpleCircuitBreaker::new(config);

        // 验证断路器初始状态为关闭
        assert_eq!(circuit_breaker.state(), CircuitBreakerState::Closed);
    }
}

/// 测试错误分类
#[cfg(test)]
mod error_classification_tests {
    use super::*;

    #[test]
    fn test_error_classification() {
        let config = ErrorClassificationConfig {
            transient_status_codes: vec![500, 502, 503, 504],
            permanent_status_codes: vec![400, 401, 403, 404, 422],
            rate_limit_status_codes: vec![429],
            timeout_status_codes: vec![408, 504],
        };

        // 测试临时性错误
        assert_eq!(config.classify_error(500), ErrorCategory::Transient);
        assert_eq!(config.classify_error(502), ErrorCategory::Transient);
        assert_eq!(config.classify_error(503), ErrorCategory::Transient);

        // 测试永久性错误
        assert_eq!(config.classify_error(400), ErrorCategory::Permanent);
        assert_eq!(config.classify_error(401), ErrorCategory::Permanent);
        assert_eq!(config.classify_error(404), ErrorCategory::Permanent);

        // 测试限流错误
        assert_eq!(config.classify_error(429), ErrorCategory::RateLimit);

        // 测试超时错误
        assert_eq!(config.classify_error(408), ErrorCategory::Timeout);
    }

    #[test]
    fn test_should_retry() {
        let config = ErrorClassificationConfig {
            transient_status_codes: vec![500, 502, 503, 504],
            permanent_status_codes: vec![400, 401, 403, 404, 422],
            rate_limit_status_codes: vec![429],
            timeout_status_codes: vec![408, 504],
        };

        // 应该重试的错误
        assert!(config.should_retry(500)); // 临时性错误
        assert!(config.should_retry(429)); // 限流错误
        assert!(config.should_retry(408)); // 超时错误

        // 不应该重试的错误
        assert!(!config.should_retry(400)); // 永久性错误
        assert!(!config.should_retry(401)); // 永久性错误
        assert!(!config.should_retry(404)); // 永久性错误
    }
}

/// 测试错误恢复管理器
#[cfg(test)]
mod error_recovery_manager_tests {
    use super::*;

    #[test]
    fn test_error_recovery_manager_creation() {
        let config = create_test_config();
        let _manager = ErrorRecoveryManager::new(config);

        // 验证管理器创建成功
        // 这是一个基本的创建测试
    }

    #[test]
    fn test_error_recovery_manager_with_default_config() {
        let _manager = ErrorRecoveryManager::with_default_config();

        // 验证使用默认配置创建管理器成功
    }

    #[traced_test]
    #[test]
    fn test_error_recovery_config_creation() {
        let config = create_test_config();

        // 验证配置各部分都正确设置
        assert_eq!(config.retry.max_retries, 3);
        assert_eq!(config.circuit_breaker.failure_threshold, 2);
        assert!(config.degradation.enabled);
        assert_eq!(config.degradation.default_response, "测试降级响应");
    }
}

/// 测试降级配置
#[cfg(test)]
mod degradation_config_tests {
    use super::*;

    #[test]
    fn test_degradation_config_defaults() {
        let config = ErrorRecoveryConfig::default();

        assert!(config.degradation.enabled);
        assert_eq!(config.degradation.cache_duration_secs, 300);
        assert_eq!(config.degradation.default_response, "服务暂时不可用，请稍后重试");
    }

    #[test]
    fn test_degradation_config_custom() {
        let config = DegradationConfig {
            enabled: false,
            cache_duration_secs: 60,
            default_response: "自定义降级响应".to_string(),
        };

        assert!(!config.enabled);
        assert_eq!(config.cache_duration_secs, 60);
        assert_eq!(config.default_response, "自定义降级响应");
    }
}
