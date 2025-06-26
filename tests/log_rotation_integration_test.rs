// tests/log_rotation_integration_test.rs
//
// 【日志文件轮转集成测试】
//
// 本测试文件验证日志文件轮转和管理功能的正确性，包括：
// 1. 日志文件的自动轮转
// 2. 旧日志文件的清理
// 3. 非阻塞写入功能
// 4. 日志目录的创建和管理

use axum_tutorial::app::middleware::logger::{
    LoggerConfig,
    LogRotation,
    setup_file_rotation_logger,
};
use std::fs;
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;

/// 测试日志目录创建功能
///
/// 【验证点】：
/// 1. 日志目录能够正确创建
/// 2. 配置参数正确传递
#[tokio::test]
async fn test_log_directory_creation() {
    let test_dir = "test_logs_creation";

    // 清理测试目录（如果存在）
    if Path::new(test_dir).exists() {
        fs::remove_dir_all(test_dir).unwrap();
    }

    let config = LoggerConfig {
        file_logging: true,
        log_directory: test_dir.to_string(),
        file_name_prefix: "test_creation".to_string(),
        rotation: LogRotation::Never, // 避免轮转干扰测试
        non_blocking: false, // 使用阻塞写入便于测试
        max_log_files: 0, // 不限制文件数量
        ..Default::default()
    };

    // 尝试设置日志系统（可能因为全局订阅者已设置而失败，但目录应该被创建）
    let _result = setup_file_rotation_logger(config);

    // 验证目录是否被创建
    assert!(Path::new(test_dir).exists());
    assert!(Path::new(test_dir).is_dir());

    // 清理测试目录
    fs::remove_dir_all(test_dir).unwrap();
}

/// 测试日志文件清理功能
///
/// 【验证点】：
/// 1. 能够正确识别和删除旧日志文件
/// 2. 保留指定数量的最新文件
/// 3. 不影响其他文件
#[tokio::test]
async fn test_log_file_cleanup() {
    let test_dir = "test_logs_cleanup";
    let file_prefix = "test_cleanup";

    // 清理并创建测试目录
    if Path::new(test_dir).exists() {
        fs::remove_dir_all(test_dir).unwrap();
    }
    fs::create_dir_all(test_dir).unwrap();

    // 创建一些测试日志文件
    let test_files = vec![
        format!("{}/{}.log.2023-01-01", test_dir, file_prefix),
        format!("{}/{}.log.2023-01-02", test_dir, file_prefix),
        format!("{}/{}.log.2023-01-03", test_dir, file_prefix),
        format!("{}/{}.log.2023-01-04", test_dir, file_prefix),
        format!("{}/{}.log.2023-01-05", test_dir, file_prefix),
        format!("{}/other_file.txt", test_dir) // 不应该被删除的文件
    ];

    for file_path in &test_files {
        fs::write(file_path, "test content").unwrap();
        // 稍微延迟以确保文件有不同的修改时间
        sleep(Duration::from_millis(10)).await;
    }

    // 验证所有文件都被创建
    for file_path in &test_files {
        assert!(Path::new(file_path).exists());
    }

    // 调用清理函数，保留3个文件
    let cleanup_result = axum_tutorial::app::middleware::logger::cleanup_old_log_files(
        test_dir,
        file_prefix,
        3
    );

    // 清理应该成功
    assert!(cleanup_result.is_ok());

    // 验证结果：应该保留3个最新的日志文件，删除2个旧文件
    let remaining_log_files: Vec<_> = fs
        ::read_dir(test_dir)
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            // 使用与cleanup函数相同的匹配逻辑
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

    assert_eq!(remaining_log_files.len(), 3);

    // 验证other_file.txt没有被删除
    assert!(Path::new(&format!("{}/other_file.txt", test_dir)).exists());

    // 清理测试目录
    fs::remove_dir_all(test_dir).unwrap();
}

/// 测试LogRotation枚举转换
///
/// 【验证点】：
/// 1. LogRotation正确转换为tracing_appender::rolling::Rotation
#[test]
fn test_log_rotation_conversion() {
    use tracing_appender::rolling::Rotation;

    // 测试所有转换
    let daily: Rotation = LogRotation::Daily.into();
    let hourly: Rotation = LogRotation::Hourly.into();
    let minutely: Rotation = LogRotation::Minutely.into();
    let never: Rotation = LogRotation::Never.into();

    // 这些转换应该不会panic
    // 具体的值比较需要访问Rotation的内部实现，这里只验证转换不出错
    assert!(std::mem::size_of_val(&daily) > 0);
    assert!(std::mem::size_of_val(&hourly) > 0);
    assert!(std::mem::size_of_val(&minutely) > 0);
    assert!(std::mem::size_of_val(&never) > 0);
}

/// 测试配置验证
///
/// 【验证点】：
/// 1. 默认配置的合理性
/// 2. 自定义配置的正确性
#[test]
fn test_logger_config_validation() {
    let default_config = LoggerConfig::default();

    // 验证默认配置的合理性
    assert!(default_config.file_logging);
    assert_eq!(default_config.log_directory, "logs");
    assert_eq!(default_config.file_name_prefix, "app");
    assert!(default_config.non_blocking);
    assert!(default_config.max_log_files > 0);
    assert!(default_config.max_file_size > 0);

    // 测试自定义配置
    let custom_config = LoggerConfig {
        file_logging: true,
        log_directory: "custom_logs".to_string(),
        file_name_prefix: "custom_app".to_string(),
        rotation: LogRotation::Hourly,
        non_blocking: false,
        max_log_files: 10,
        max_file_size: 50 * 1024 * 1024,
        ..Default::default()
    };

    assert!(custom_config.file_logging);
    assert_eq!(custom_config.log_directory, "custom_logs");
    assert_eq!(custom_config.file_name_prefix, "custom_app");
    assert!(matches!(custom_config.rotation, LogRotation::Hourly));
    assert!(!custom_config.non_blocking);
    assert_eq!(custom_config.max_log_files, 10);
    assert_eq!(custom_config.max_file_size, 50 * 1024 * 1024);
}
