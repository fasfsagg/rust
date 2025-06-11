//! tests/common/mod.rs

use axum_tutorial::{
    config::AppConfig, // 导入应用配置结构体
    startup::init_app, // 导入应用初始化函数
};
use sea_orm::{ ConnectionTrait, DatabaseConnection, DbErr, Statement };
use std::net::SocketAddr;
use tokio::net::TcpListener; // 导入 Tokio 的 TCP 监听器
use uuid::Uuid; // 用于生成唯一标识符

/// TestApp 结构体，用于封装测试服务器的地址和数据库连接池
#[allow(dead_code)] // 允许结构体中包含未使用的字段 (例如 http_addr)
pub struct TestApp {
    pub http_addr: SocketAddr,
    pub db_connection: DatabaseConnection,
}

/// 启动应用以进行集成测试
///
/// 这个函数会执行以下操作:
/// 1. **配置加载**: 调用 `AppConfig::from_env()` 加载配置。
/// 2. **应用初始化**: 调用 `init_app(config)` 创建 Axum 应用实例。
/// 3. **端口监听**: 在 `127.0.0.1:0` 上绑定 TCP 监听器，
///    - `:0` 是一个特殊端口，它请求操作系统分配一个当前未被使用的任意端口。
///    - 这对于并行运行多个测试实例至关重要，可以避免端口冲突。
/// 4. **地址保存**: 保存操作系统分配的实际地址，用于后续的 HTTP 请求。
/// 5. **服务器启动**: 在一个独立的 Tokio 任务 (`tokio::spawn`) 中启动服务器。
///    - `tokio::spawn` 会在后台运行服务器，不会阻塞测试主线程的执行。
/// 6. **返回应用地址**: 将 `TestApp` 结构体（包含服务器地址和数据库连接池）返回给调用者。
pub async fn spawn_app() -> TestApp {
    // 为测试创建一个 TCP 监听器，绑定到随机可用端口
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("无法为测试绑定随机端口");
    // 获取操作系统分配的实际地址
    let addr = listener.local_addr().unwrap();

    // 生成唯一的数据库名称，以确保测试之间的隔离
    let db_name = Uuid::new_v4().to_string();
    // 使用临时文件作为数据库，而不是内存数据库，因为SQLite内存数据库在连接之间不共享
    let db_url = format!("sqlite:{}?mode=rwc", db_name);
    println!("INFO: 为测试创建唯一临时数据库文件: {}", db_url);

    // 加载应用配置并覆盖数据库 URL
    let mut config = AppConfig::from_env();
    config.database_url = db_url;

    // 初始化应用，现在它返回应用本身和数据库连接
    let (app, db_connection) = init_app(config).await.expect("无法初始化应用");

    // 确保数据库是空的
    cleanup_db(&db_connection).await.expect("无法清理数据库");

    // 在一个新的 Tokio 任务中启动服务器，使其在后台运行
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service()).await.unwrap();
    });

    TestApp {
        http_addr: addr,
        db_connection,
    }
}

/// 清理数据库，删除所有表中的数据
///
/// 在每个测试后调用此函数，以确保测试之间的独立性。
#[allow(dead_code)] // 允许这个函数在当前未被使用，因为它是一个供测试调用的工具函数
pub async fn cleanup_db(db: &DatabaseConnection) -> Result<(), DbErr> {
    // 打印清理开始日志
    println!("INFO: 开始清理测试数据库...");

    let tables = [
        "tasks",
        // 如果有其他表，也在这里添加
    ];

    for table in tables.iter() {
        let query = format!("DELETE FROM {};", table);
        match db.execute(Statement::from_string(db.get_database_backend(), query)).await {
            Ok(result) =>
                println!("INFO: 已清理表 {} - 删除了 {} 行", table, result.rows_affected()),
            Err(e) => {
                println!("ERROR: 清理表 {} 时出错: {}", table, e);
                return Err(e);
            }
        }
    }

    println!("INFO: 测试数据库清理完成");
    Ok(())
}
