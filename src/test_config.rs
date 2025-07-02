// src/test_config.rs
//! 测试配置模块
//!
//! 提供统一的测试配置管理，包括：
//! - 测试数据库配置
//! - 测试环境设置
//! - 测试工具配置
//! - 测试数据工厂

use std::sync::Once;
use sea_orm::{ Database, DatabaseConnection };
use tempfile::TempDir;
use tracing_subscriber::{ EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt };
use crate::config::AppConfig;
use migration::{ Migrator, MigratorTrait };

static INIT: Once = Once::new();

/// 测试配置结构体
#[derive(Debug)]
pub struct TestConfig {
    /// 测试数据库连接字符串
    pub database_url: String,
    /// 测试服务器端口
    pub server_port: u16,
    /// 测试JWT密钥
    pub jwt_secret: String,
    /// 测试临时目录路径
    pub temp_dir_path: Option<String>,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            database_url: "sqlite::memory:".to_string(), // 使用SQLite内存数据库进行测试
            server_port: 0, // 使用随机端口
            jwt_secret: "test_jwt_secret_key_for_testing_only".to_string(),
            temp_dir_path: None,
        }
    }
}

impl TestConfig {
    /// 创建新的测试配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建带临时目录的测试配置
    pub fn with_temp_dir() -> anyhow::Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let temp_path = temp_dir.path().to_string_lossy().to_string();
        // 注意：这里我们只保存路径，临时目录会在函数结束时被删除
        // 在实际使用中，你可能需要不同的策略来管理临时目录的生命周期
        Ok(Self {
            temp_dir_path: Some(temp_path),
            ..Self::default()
        })
    }

    /// 创建测试用的应用配置
    pub fn to_app_config(&self) -> AppConfig {
        use crate::config::{ DatabasePoolConfig, WebSocketPoolConfig };
        use std::net::SocketAddr;

        let addr: SocketAddr = if self.server_port == 0 {
            "127.0.0.1:0".parse().unwrap()
        } else {
            format!("127.0.0.1:{}", self.server_port).parse().unwrap()
        };

        AppConfig {
            http_addr: addr,
            database_url: self.database_url.clone(),
            jwt_secret: self.jwt_secret.clone(),
            database_pool: DatabasePoolConfig::development(),
            websocket_pool: WebSocketPoolConfig::development(),
        }
    }

    /// 获取测试数据库连接
    pub async fn get_database_connection(&self) -> anyhow::Result<DatabaseConnection> {
        let db = Database::connect(&self.database_url).await?;

        // 运行迁移
        Migrator::up(&db, None).await?;

        Ok(db)
    }
}

/// 初始化测试日志系统
pub fn init_test_logging() {
    INIT.call_once(|| {
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));

        // 尝试设置全局默认订阅者，如果已经设置则忽略错误
        let _ = tracing_subscriber
            ::registry()
            .with(fmt::layer().with_test_writer())
            .with(filter)
            .try_init();
    });
}

/// 测试数据工厂
pub mod test_data {
    use uuid::Uuid;
    use chrono::Utc;
    use crate::app::model::task::{ CreateTaskPayload, UpdateTaskPayload };
    // 注意：用户模型暂时不可用，使用占位符结构体
    #[derive(Debug)]
    pub struct CreateUserPayload {
        pub username: String,
        pub email: String,
        pub password: String,
    }

    #[derive(Debug)]
    pub struct LoginPayload {
        pub username: String,
        pub password: String,
    }

    /// 创建测试任务数据
    pub fn create_test_task(title: Option<&str>, description: Option<&str>) -> CreateTaskPayload {
        CreateTaskPayload {
            title: title.unwrap_or("测试任务").to_string(),
            description: Some(description.unwrap_or("这是一个测试任务").to_string()),
            completed: false,
        }
    }

    /// 创建测试任务更新数据
    pub fn update_test_task(
        title: Option<&str>,
        description: Option<&str>,
        completed: Option<bool>
    ) -> UpdateTaskPayload {
        UpdateTaskPayload {
            title: title.map(|s| s.to_string()),
            description: description.map(|s| Some(s.to_string())),
            completed,
        }
    }

    /// 创建测试用户数据
    pub fn create_test_user(username: Option<&str>, email: Option<&str>) -> CreateUserPayload {
        let timestamp = Utc::now().timestamp();
        CreateUserPayload {
            username: username.unwrap_or(&format!("testuser_{}", timestamp)).to_string(),
            email: email.unwrap_or(&format!("test_{}@example.com", timestamp)).to_string(),
            password: "test_password_123".to_string(),
        }
    }

    /// 创建测试登录数据
    pub fn create_test_login(username: &str) -> LoginPayload {
        LoginPayload {
            username: username.to_string(),
            password: "test_password_123".to_string(),
        }
    }

    /// 生成测试UUID
    pub fn test_uuid() -> Uuid {
        Uuid::new_v4()
    }

    /// 生成无效的UUID字符串（用于测试错误处理）
    pub fn invalid_uuid() -> String {
        "invalid-uuid-string".to_string()
    }
}

/// 测试断言宏
#[macro_export]
macro_rules! assert_error_contains {
    ($result:expr, $expected:expr) => {
        match $result {
            Ok(_) => panic!("Expected error, but got Ok"),
            Err(e) => assert!(
                e.to_string().contains($expected),
                "Error '{}' does not contain '{}'",
                e.to_string(),
                $expected
            ),
        }
    };
}

/// 异步测试断言宏
#[macro_export]
macro_rules! assert_async_error_contains {
    ($result:expr, $expected:expr) => {
        match $result.await {
            Ok(_) => panic!("Expected error, but got Ok"),
            Err(e) => assert!(
                e.to_string().contains($expected),
                "Error '{}' does not contain '{}'",
                e.to_string(),
                $expected
            ),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = TestConfig::new();
        assert_eq!(config.database_url, "sqlite::memory:");
        assert_eq!(config.server_port, 0);
        assert!(!config.jwt_secret.is_empty());
    }

    #[test]
    fn test_config_with_temp_dir() {
        let config = TestConfig::with_temp_dir().unwrap();
        assert!(config.temp_dir_path.is_some());
    }

    #[test]
    fn test_app_config_conversion() {
        let test_config = TestConfig::new();
        let app_config = test_config.to_app_config();
        assert_eq!(app_config.database_url, test_config.database_url);
        assert_eq!(app_config.jwt_secret, test_config.jwt_secret);
    }

    #[test]
    fn test_data_factory() {
        let task = test_data::create_test_task(None, None);
        assert_eq!(task.title, "测试任务");
        assert_eq!(task.description, Some("这是一个测试任务".to_string()));

        let custom_task = test_data::create_test_task(Some("自定义任务"), Some("自定义描述"));
        assert_eq!(custom_task.title, "自定义任务");
        assert_eq!(custom_task.description, Some("自定义描述".to_string()));
    }

    #[test]
    fn test_uuid_generation() {
        let uuid1 = test_data::test_uuid();
        let uuid2 = test_data::test_uuid();
        assert_ne!(uuid1, uuid2);
    }
}
