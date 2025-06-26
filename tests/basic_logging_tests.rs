// tests/basic_logging_tests.rs
//
// /-----------------------------------------------------------------------------\
// |                        【基础日志记录测试模块】                             |
// |-----------------------------------------------------------------------------|
// | 简化的日志记录功能测试，专注于核心功能验证                                   |
// | 测试覆盖：                                                                  |
// | 1. 日志配置验证                                                             |
// | 2. 日志轮转枚举                                                             |
// | 3. 日志文件清理功能                                                         |
// | 4. 结构化日志记录                                                           |
// \-----------------------------------------------------------------------------/

use axum_tutorial::app::middleware::logger::{ LoggerConfig, LogRotation, cleanup_old_log_files };
use std::{ fs::{ self, File }, io::Write };
use tempfile::TempDir;
use tracing::{ debug, error, info, warn };
use tracing_test::traced_test;

/// 测试日志配置的默认值
#[cfg(test)]
mod config_basic_tests {
    use super::*;

    #[test]
    fn test_logger_config_defaults() {
        let config = LoggerConfig::default();

        assert!(!config.json_format);
        assert!(config.file_logging);
        assert_eq!(config.log_directory, "logs");
        assert!(config.error_tracing);
        assert!(config.show_spans);
        assert!(config.custom_filter.is_none());
        assert_eq!(config.file_name_prefix, "app");
        assert!(config.non_blocking);
        assert_eq!(config.max_log_files, 30);
        assert_eq!(config.max_file_size, 100 * 1024 * 1024);
    }

    #[test]
    fn test_log_rotation_enum_conversion() {
        use tracing_appender::rolling::Rotation;

        let daily: Rotation = LogRotation::Daily.into();
        let hourly: Rotation = LogRotation::Hourly.into();
        let minutely: Rotation = LogRotation::Minutely.into();
        let never: Rotation = LogRotation::Never.into();

        // 验证转换不会panic
        assert_eq!(format!("{:?}", daily), "Rotation(Daily)");
        assert_eq!(format!("{:?}", hourly), "Rotation(Hourly)");
        assert_eq!(format!("{:?}", minutely), "Rotation(Minutely)");
        assert_eq!(format!("{:?}", never), "Rotation(Never)");
    }

    #[test]
    fn test_logger_config_clone() {
        let config1 = LoggerConfig::default();
        let config2 = config1.clone();

        assert_eq!(config1.json_format, config2.json_format);
        assert_eq!(config1.file_logging, config2.file_logging);
        assert_eq!(config1.log_directory, config2.log_directory);
        assert_eq!(config1.file_name_prefix, config2.file_name_prefix);
    }

    #[test]
    fn test_custom_logger_config() {
        let config = LoggerConfig {
            json_format: true,
            file_logging: false,
            log_directory: "custom_logs".to_string(),
            error_tracing: false,
            show_spans: false,
            custom_filter: Some("debug".to_string()),
            rotation: LogRotation::Hourly,
            file_name_prefix: "custom".to_string(),
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
        assert_eq!(config.file_name_prefix, "custom");
        assert!(!config.non_blocking);
        assert_eq!(config.max_log_files, 10);
        assert_eq!(config.max_file_size, 50 * 1024 * 1024);
    }
}

/// 测试日志文件清理功能
#[cfg(test)]
mod file_cleanup_tests {
    use super::*;

    #[test]
    fn test_cleanup_old_log_files_basic() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path();
        let file_prefix = "test";

        // 创建测试日志文件
        let test_files = vec![
            format!("{}.log", file_prefix),
            format!("{}.log.2023-01-01", file_prefix),
            format!("{}.log.2023-01-02", file_prefix),
            format!("{}.log.2023-01-03", file_prefix),
            "other.txt".to_string()
        ];

        for file_name in &test_files {
            let file_path = log_dir.join(file_name);
            let mut file = File::create(&file_path).unwrap();
            writeln!(file, "test content").unwrap();
        }

        // 验证所有文件都被创建
        assert_eq!(fs::read_dir(log_dir).unwrap().count(), 5);

        // 执行清理，保留3个文件
        let result = cleanup_old_log_files(log_dir.to_string_lossy().as_ref(), file_prefix, 3);
        assert!(result.is_ok());

        // 计算剩余的日志文件
        let remaining_log_files: Vec<_> = fs
            ::read_dir(log_dir)
            .unwrap()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let file_name = entry.file_name().to_string_lossy().to_string();
                if
                    file_name.starts_with(file_prefix) &&
                    (file_name.ends_with(".log") || file_name.contains(".log."))
                {
                    Some(file_name)
                } else {
                    None
                }
            })
            .collect();

        // 应该保留3个日志文件
        assert_eq!(remaining_log_files.len(), 3);

        // other.txt应该还在
        assert!(log_dir.join("other.txt").exists());
    }

    #[test]
    fn test_cleanup_with_no_files_to_remove() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path();
        let file_prefix = "test";

        // 只创建2个文件，但要求保留5个
        let test_files = vec![
            format!("{}.log", file_prefix),
            format!("{}.log.2023-01-01", file_prefix)
        ];

        for file_name in &test_files {
            let file_path = log_dir.join(file_name);
            File::create(&file_path).unwrap();
        }

        let result = cleanup_old_log_files(
            log_dir.to_string_lossy().as_ref(),
            file_prefix,
            5 // 要求保留5个，但只有2个
        );
        assert!(result.is_ok());

        // 所有文件都应该保留
        let remaining_files: Vec<_> = fs
            ::read_dir(log_dir)
            .unwrap()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name.starts_with(file_prefix) {
                    Some(file_name)
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(remaining_files.len(), 2);
    }

    #[test]
    fn test_cleanup_nonexistent_directory() {
        let result = cleanup_old_log_files("/nonexistent/directory", "test", 5);
        // 对于不存在的目录，函数设计为返回Ok()（优雅处理）
        assert!(result.is_ok());
    }

    #[test]
    fn test_cleanup_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path();

        let result = cleanup_old_log_files(log_dir.to_string_lossy().as_ref(), "test", 5);
        // 空目录应该成功处理
        assert!(result.is_ok());
    }

    #[test]
    fn test_cleanup_with_zero_max_files() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path();
        let file_prefix = "test";

        // 创建一些测试文件
        for i in 1..=3 {
            let file_path = log_dir.join(format!("{}.log.2023-01-0{}", file_prefix, i));
            File::create(&file_path).unwrap();
        }

        let result = cleanup_old_log_files(
            log_dir.to_string_lossy().as_ref(),
            file_prefix,
            0 // 保留0个文件，应该删除所有日志文件
        );
        assert!(result.is_ok());

        // 验证所有日志文件都被删除
        let remaining_files: Vec<_> = fs
            ::read_dir(log_dir)
            .unwrap()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name.starts_with(file_prefix) {
                    Some(file_name)
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(remaining_files.len(), 0);
    }
}

/// 测试日志级别和过滤功能
#[cfg(test)]
mod logging_level_tests {
    use super::*;

    #[traced_test]
    #[test]
    fn test_different_log_levels() {
        // 测试不同级别的日志记录
        debug!("这是一条调试消息");
        info!("这是一条信息消息");
        warn!("这是一条警告消息");
        error!("这是一条错误消息");

        // 验证日志被正确记录（通过 tracing-test 的 logs_contain 宏）
        assert!(logs_contain("这是一条信息消息"));
        assert!(logs_contain("这是一条警告消息"));
        assert!(logs_contain("这是一条错误消息"));
    }

    #[traced_test]
    #[test]
    fn test_structured_logging() {
        let user_id = 12345;
        let action = "login";

        info!(
            user_id = user_id,
            action = action,
            timestamp = %chrono::Utc::now(),
            "用户执行操作"
        );

        assert!(logs_contain("用户执行操作"));
        assert!(logs_contain("user_id"));
        assert!(logs_contain("action"));
    }

    #[traced_test]
    #[test]
    fn test_error_with_context() {
        let error_msg = "数据库连接失败";
        let error_code = "DB_CONNECTION_ERROR";

        error!(
            error_message = error_msg,
            error_code = error_code,
            retry_count = 3,
            "数据库操作失败"
        );

        assert!(logs_contain("数据库操作失败"));
        assert!(logs_contain("error_message"));
        assert!(logs_contain("error_code"));
        assert!(logs_contain("retry_count"));
    }

    #[traced_test]
    #[test]
    fn test_warning_with_fields() {
        let request_id = "req-12345";
        let processing_time_ms = 1500;

        warn!(
            request_id = request_id,
            processing_time_ms = processing_time_ms,
            threshold_ms = 1000,
            "请求处理时间超过阈值"
        );

        assert!(logs_contain("请求处理时间超过阈值"));
        assert!(logs_contain("request_id"));
        assert!(logs_contain("processing_time_ms"));
        assert!(logs_contain("threshold_ms"));
    }
}
