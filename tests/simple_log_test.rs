// tests/simple_log_test.rs
//
// 简单的日志轮转功能测试

use axum_tutorial::app::middleware::logger::{LoggerConfig, LogRotation};
use std::fs;
use std::path::Path;

#[test]
fn test_cleanup_function_basic() {
    let test_dir = "test_cleanup_basic";
    let file_prefix = "test";
    
    // 清理并创建测试目录
    if Path::new(test_dir).exists() {
        fs::remove_dir_all(test_dir).unwrap();
    }
    fs::create_dir_all(test_dir).unwrap();

    // 创建一些测试文件
    let test_files = vec![
        format!("{}/{}.log", test_dir, file_prefix),
        format!("{}/{}.log.2023-01-01", test_dir, file_prefix),
        format!("{}/{}.log.2023-01-02", test_dir, file_prefix),
        format!("{}/other.txt", test_dir),
    ];

    for file_path in &test_files {
        fs::write(file_path, "test content").unwrap();
    }

    // 验证文件创建成功
    assert_eq!(fs::read_dir(test_dir).unwrap().count(), 4);

    // 调用清理函数，保留2个文件
    let result = axum_tutorial::app::middleware::logger::cleanup_old_log_files(
        test_dir,
        file_prefix,
        2,
    );
    
    assert!(result.is_ok());

    // 计算剩余的日志文件
    let remaining_log_files: Vec<_> = fs::read_dir(test_dir)
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.starts_with(file_prefix) && 
               (file_name.ends_with(".log") || file_name.contains(".log.")) {
                Some(file_name)
            } else {
                None
            }
        })
        .collect();

    // 应该保留2个日志文件
    assert_eq!(remaining_log_files.len(), 2);

    // other.txt应该还在
    assert!(Path::new(&format!("{}/other.txt", test_dir)).exists());

    // 清理测试目录
    fs::remove_dir_all(test_dir).unwrap();
}

#[test]
fn test_log_rotation_enum() {
    use tracing_appender::rolling::Rotation;
    
    // 测试枚举转换不会panic
    let _: Rotation = LogRotation::Daily.into();
    let _: Rotation = LogRotation::Hourly.into();
    let _: Rotation = LogRotation::Minutely.into();
    let _: Rotation = LogRotation::Never.into();
}

#[test]
fn test_logger_config_defaults() {
    let config = LoggerConfig::default();
    
    assert!(config.file_logging);
    assert_eq!(config.log_directory, "logs");
    assert_eq!(config.file_name_prefix, "app");
    assert!(config.non_blocking);
    assert_eq!(config.max_log_files, 30);
    assert_eq!(config.max_file_size, 100 * 1024 * 1024);
}
