// examples/log_rotation_demo.rs
//
// 【日志文件轮转功能演示】
//
// 本演示程序展示如何使用新的日志文件轮转功能，包括：
// 1. 配置日志文件轮转策略
// 2. 设置非阻塞写入
// 3. 自动清理旧日志文件
// 4. 不同的轮转策略演示

use axum_tutorial::app::middleware::logger::{
    LoggerConfig, LogRotation, setup_file_rotation_logger
};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error, debug};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 日志文件轮转功能演示 ===\n");

    // 演示1：每分钟轮转的日志配置（用于快速演示）
    println!("1. 配置每分钟轮转的日志系统...");
    let config = LoggerConfig {
        file_logging: true,
        log_directory: "demo_logs".to_string(),
        file_name_prefix: "demo_app".to_string(),
        rotation: LogRotation::Minutely, // 每分钟轮转，便于演示
        non_blocking: true,
        max_log_files: 5, // 只保留5个文件
        max_file_size: 1024 * 1024, // 1MB
        error_tracing: true,
        json_format: false,
        show_spans: true,
        custom_filter: Some("debug".to_string()),
    };

    // 初始化日志系统
    let _guard = setup_file_rotation_logger(config)?;
    
    println!("✓ 日志系统初始化完成");
    println!("✓ 日志文件将保存到: demo_logs/");
    println!("✓ 文件名前缀: demo_app");
    println!("✓ 轮转策略: 每分钟");
    println!("✓ 最大保留文件数: 5");
    println!("✓ 非阻塞写入: 启用\n");

    // 演示2：生成不同级别的日志
    println!("2. 生成测试日志...");
    
    for i in 1..=20 {
        info!(
            iteration = i,
            timestamp = %chrono::Utc::now(),
            "这是第 {} 次日志记录",
            i
        );
        
        if i % 3 == 0 {
            warn!(
                iteration = i,
                "这是一个警告日志 - 迭代 {}",
                i
            );
        }
        
        if i % 5 == 0 {
            error!(
                iteration = i,
                error_code = "DEMO_ERROR",
                "这是一个错误日志 - 迭代 {}",
                i
            );
        }
        
        if i % 7 == 0 {
            debug!(
                iteration = i,
                debug_info = "详细调试信息",
                "调试日志 - 迭代 {}",
                i
            );
        }

        // 每隔2秒记录一次
        sleep(Duration::from_secs(2)).await;
        
        if i % 5 == 0 {
            println!("  已生成 {} 条日志记录", i);
        }
    }

    println!("\n3. 演示结构化日志记录...");
    
    // 演示结构化日志
    info!(
        user_id = "user_123",
        action = "login",
        ip_address = "192.168.1.100",
        user_agent = "Mozilla/5.0",
        success = true,
        duration_ms = 150,
        "用户登录成功"
    );

    warn!(
        user_id = "user_456",
        action = "failed_login",
        ip_address = "192.168.1.200",
        reason = "invalid_password",
        attempt_count = 3,
        "用户登录失败"
    );

    error!(
        error_type = "database_connection",
        database = "postgresql",
        host = "localhost",
        port = 5432,
        retry_count = 3,
        "数据库连接失败"
    );

    println!("✓ 结构化日志记录完成");

    // 演示4：等待文件轮转
    println!("\n4. 等待日志文件轮转...");
    println!("  (如果当前时间接近分钟边界，可能会看到新的日志文件生成)");
    
    // 继续生成一些日志以触发轮转
    for i in 1..=10 {
        info!(
            batch = "rotation_demo",
            sequence = i,
            "轮转演示日志 - 序号 {}",
            i
        );
        sleep(Duration::from_secs(3)).await;
    }

    println!("\n5. 演示完成！");
    println!("请检查 demo_logs/ 目录中的日志文件：");
    println!("  - demo_app.log (当前日志文件)");
    println!("  - demo_app.log.YYYY-MM-DD-HH-MM (轮转后的历史文件)");
    println!("  - 旧文件会根据配置自动清理");

    // 最后的清理信息
    info!(
        demo_status = "completed",
        total_duration_seconds = 60,
        "日志轮转演示程序执行完成"
    );

    println!("\n注意：");
    println!("- 在生产环境中，建议使用 LogRotation::Daily 或 LogRotation::Hourly");
    println!("- 非阻塞写入可以提高应用性能，特别是在高并发场景下");
    println!("- 定期清理旧日志文件有助于节省磁盘空间");
    println!("- 可以通过 RUST_LOG 环境变量调整日志级别");

    Ok(())
}
