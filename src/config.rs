// 文件路径: src/config.rs
//
// /---------------------------------------------------------------------------------------------\
// |                                【配置管理模块】 (config.rs)                                 |
// |---------------------------------------------------------------------------------------------|
// |                                                                                             |
// | 1. **导入依赖**: `std::net::SocketAddr`, `std::time::Duration`。                            |
// |                                                                                             |
// | 2. **`AppConfig` 结构体**: 定义应用程序的所有配置参数。                                     |
// |    - `pub http_addr: SocketAddr`: HTTP/1.1 服务器监听的地址和端口。                        |
// |    - `pub http3_addr: SocketAddr`: HTTP/3 服务器监听的地址和端口。                         |
// |    - `pub ws_ping_interval: u64`: WebSocket 心跳检测的间隔时间 (秒)。                      |
// |    - `#[derive(Clone, Debug)]`:                                                             |
// |      - `Clone`: 允许创建 `AppConfig` 的副本 (例如在 `startup.rs` 中传递给不同部分)。     |
// |      - `Debug`: 允许使用 `{:?}` 格式打印配置，方便调试。                                   |
// |                                                                                             |
// | 3. **`impl AppConfig`**: 为 `AppConfig` 实现关联函数和方法。                                |
// |    - **`pub fn from_env() -> Self` (关联函数/构造函数)**:                                     |
// |      - **职责**: 从环境变量加载配置，如果环境变量未设置，则使用预设的默认值。                |
// |      - **实现**:                                                                           |
// |        - `std::env::var("VAR_NAME")`: 尝试读取名为 "VAR_NAME" 的环境变量。              |
// |        - `.unwrap_or_else(|_| "default_value".to_string())`: 如果环境变量不存在，则返回默认值。 |
// |        - `.parse()`: 将读取到的字符串尝试解析为目标类型 (如 `SocketAddr` 或 `u64`)。       |
// |        - `.expect("错误消息")`: 如果 `.parse()` 失败，则程序会 panic 并显示指定的错误消息。   |
// |          【注意】: 在生产代码中，通常使用 `Result` 和更细致的错误处理，而不是 `expect`。    |
// |        - `.map(|v| v.parse()...)`: 用于处理 `var` 可能返回 `Ok(String)` 的情况。          |
// |        - `.unwrap_or(default_value)`: 如果环境变量存在但解析失败，或者环境变量不存在，       |
// |          则使用默认的 `u64` 值。                                                        |
// |      - **返回**: 一个包含最终配置值的 `AppConfig` 实例 (`Self`)。                            |
// |    - **`pub fn ws_ping_interval(&self) -> Duration` (方法)**:                                |
// |      - **职责**: 将存储为 `u64` 秒数的 `ws_ping_interval` 转换为 `std::time::Duration` 类型。 |
// |      - **实现**: `Duration::from_secs(self.ws_ping_interval)`。                           |
// |                                                                                             |
// \---------------------------------------------------------------------------------------------/
//
// 【核心职责】: 提供一个统一的地方来定义、加载和访问应用程序的配置参数。
// 【关键技术】: 结构体 (`struct`), 派生宏 (`derive`), 环境变量读取 (`std::env::var`), 错误处理 (`Result`, `Option`, `expect`), 类型解析 (`.parse()`), 类型转换 (`Duration::from_secs`).

// --- 导入依赖 ---
use std::fmt;
use std::net::SocketAddr; // 用于表示 IP 地址和端口号
use std::time::Duration; // 用于表示时间间隔 // 用于实现Display trait

// --- 配置错误类型定义 ---

/// 配置错误类型
///
/// 【目的】: 提供详细的配置加载和验证错误信息
/// 【设计】: 使用枚举类型覆盖各种配置错误场景
#[derive(Debug, Clone)]
pub enum ConfigError {
    /// 环境变量解析错误
    EnvVarParseError {
        var_name: String,
        value: String,
        expected_type: String,
        suggestion: String,
    },
    /// 配置验证错误
    ValidationError {
        field: String,
        value: String,
        reason: String,
        suggestion: String,
    },
    /// 配置依赖关系错误
    DependencyError {
        field: String,
        dependent_field: String,
        reason: String,
    },
    /// 环境变量缺失错误
    MissingEnvVar {
        var_name: String,
        suggestion: String,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::EnvVarParseError {
                var_name,
                value,
                expected_type,
                suggestion,
            } => {
                write!(
                    f,
                    "环境变量 '{}' 的值 '{}' 无法解析为 {}。建议: {}",
                    var_name, value, expected_type, suggestion
                )
            }
            ConfigError::ValidationError {
                field,
                value,
                reason,
                suggestion,
            } => {
                write!(
                    f,
                    "配置字段 '{}' 的值 '{}' 验证失败: {}。建议: {}",
                    field, value, reason, suggestion
                )
            }
            ConfigError::DependencyError {
                field,
                dependent_field,
                reason,
            } => {
                write!(
                    f,
                    "配置字段 '{}' 与 '{}' 存在依赖关系错误: {}",
                    field, dependent_field, reason
                )
            }
            ConfigError::MissingEnvVar {
                var_name,
                suggestion,
            } => {
                write!(f, "缺少必需的环境变量 '{}'。建议: {}", var_name, suggestion)
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// 配置结果类型别名
pub type ConfigResult<T> = Result<T, ConfigError>;

// --- 配置结构体定义 ---

/// 数据库连接池配置结构体
///
/// 【目的】: 专门用于配置SeaORM数据库连接池的参数，支持百万并发连接优化
/// 【设计】: 基于SeaORM ConnectOptions的最佳实践配置
#[derive(Clone, Debug)]
pub struct DatabasePoolConfig {
    /// 最大连接数 - 支持百万并发的关键参数
    pub max_connections: u32,
    /// 最小连接数 - 保持基础连接池大小
    pub min_connections: u32,
    /// 连接超时时间 - 防止连接建立过慢
    pub connect_timeout: Duration,
    /// 空闲超时时间 - 自动清理空闲连接
    pub idle_timeout: Duration,
    /// 连接最大生命周期 - 防止连接过期
    pub max_lifetime: Option<Duration>,
    /// 获取连接超时时间 - 防止应用阻塞
    pub acquire_timeout: Duration,
    /// 启用TCP_NODELAY - 减少网络延迟
    pub tcp_nodelay: bool,
    /// 启用连接保活 - 检测断开的连接
    pub tcp_keepalive: bool,
}

/// WebSocket连接池配置结构体
///
/// 【目的】: 配置WebSocket连接池和复用参数，优化实时通信性能
/// 【设计】: 支持连接复用、负载均衡和故障转移
#[derive(Clone, Debug)]
pub struct WebSocketPoolConfig {
    /// 最大WebSocket连接数
    pub max_connections: u32,
    /// 连接池大小 - 用于连接复用
    pub pool_size: u32,
    /// 心跳间隔 - 保持连接活跃
    pub heartbeat_interval: Duration,
    /// 连接超时时间
    pub connection_timeout: Duration,
    /// 重连最大尝试次数
    pub max_reconnect_attempts: u32,
    /// 重连间隔
    pub reconnect_interval: Duration,
    /// 启用连接负载均衡
    pub enable_load_balancing: bool,
    /// 启用故障转移
    pub enable_failover: bool,
}

/// 应用程序配置结构体 (Application Configuration Struct)
///
/// 【目的】: 集中存储应用程序运行所需的所有配置参数。
/// 【设计】: 结构体的字段是公开的 (`pub`)，允许其他模块直接访问。
///
/// # 【`#[derive(Clone, Debug)]`】 [[关键语法要素: derive 宏]]
///   - `Clone`: 自动为 `AppConfig` 实现 `Clone` trait。[[Rust语法特性/概念: Clone Trait]]
///     这允许我们创建 `AppConfig` 实例的【深拷贝】。
///     对于配置对象来说，通常需要能够克隆它以便在应用的不同部分（如不同线程或模块）中使用独立的副本或共享引用（通过 `Arc`）。
///   - `Debug`: 自动实现 `std::fmt::Debug` trait。[[Rust语法特性/概念: Debug Trait]]
///     这允许我们使用调试格式化符号 (`{:?}` 或 `{:#?}`) 来打印 `AppConfig` 实例的内容，非常有助于调试。
#[derive(Clone, Debug)]
pub struct AppConfig {
    /// HTTP 服务器监听地址。
    /// `SocketAddr` 结构体结合了 IP 地址和端口号。
    pub http_addr: SocketAddr,

    /// 数据库连接 URL。
    /// 用于连接到应用程序的数据库 (例如 SQLite, PostgreSQL)。
    pub database_url: String,

    /// JWT 签名密钥。
    /// 用于签名和验证 JWT 令牌的密钥。
    /// 在生产环境中，这应该是一个强随机字符串。
    pub jwt_secret: String,

    /// 数据库连接池配置
    /// 【任务13.4新增】: 专门的数据库连接池优化配置
    pub database_pool: DatabasePoolConfig,

    /// WebSocket连接池配置
    /// 【任务13.4新增】: WebSocket连接池和复用配置
    pub websocket_pool: WebSocketPoolConfig,
}

// --- 配置加载实现 ---

impl DatabasePoolConfig {
    /// 创建生产环境优化的数据库连接池配置
    ///
    /// 【功能】: 为百万并发场景优化的数据库连接池配置
    /// 【参数】: 基于SeaORM最佳实践和企业级应用需求
    pub fn production() -> Self {
        Self {
            max_connections: 100,                          // 支持高并发的连接数
            min_connections: 10,                           // 保持基础连接池
            connect_timeout: Duration::from_secs(30),      // 连接建立超时
            idle_timeout: Duration::from_secs(600),        // 10分钟空闲超时
            max_lifetime: Some(Duration::from_secs(3600)), // 1小时最大生命周期
            acquire_timeout: Duration::from_secs(10),      // 获取连接超时
            tcp_nodelay: true,                             // 启用TCP_NODELAY减少延迟
            tcp_keepalive: true,                           // 启用TCP保活检测
        }
    }

    /// 创建开发环境的数据库连接池配置
    ///
    /// 【功能】: 适合开发和测试的轻量级配置
    pub fn development() -> Self {
        Self {
            max_connections: 20,                           // 开发环境较少连接
            min_connections: 2,                            // 最小连接数
            connect_timeout: Duration::from_secs(10),      // 较短连接超时
            idle_timeout: Duration::from_secs(300),        // 5分钟空闲超时
            max_lifetime: Some(Duration::from_secs(1800)), // 30分钟最大生命周期
            acquire_timeout: Duration::from_secs(5),       // 获取连接超时
            tcp_nodelay: true,                             // 仍然启用TCP_NODELAY
            tcp_keepalive: true,                           // 启用TCP保活检测
        }
    }

    /// 验证数据库连接池配置的有效性
    ///
    /// 【功能】: 检查配置参数的合理性和一致性
    /// 【返回】: 验证成功返回Ok(())，失败返回ConfigError
    pub fn validate(&self) -> ConfigResult<()> {
        // 验证连接数配置
        if self.max_connections == 0 {
            return Err(ConfigError::ValidationError {
                field: "max_connections".to_string(),
                value: self.max_connections.to_string(),
                reason: "最大连接数不能为0".to_string(),
                suggestion: "设置为至少1个连接，推荐生产环境100个，开发环境20个".to_string(),
            });
        }

        if self.min_connections > self.max_connections {
            return Err(ConfigError::ValidationError {
                field: "min_connections".to_string(),
                value: format!(
                    "min: {}, max: {}",
                    self.min_connections, self.max_connections
                ),
                reason: "最小连接数不能大于最大连接数".to_string(),
                suggestion: "确保min_connections <= max_connections".to_string(),
            });
        }

        // 验证超时配置
        if self.connect_timeout.as_secs() == 0 {
            return Err(ConfigError::ValidationError {
                field: "connect_timeout".to_string(),
                value: format!("{}秒", self.connect_timeout.as_secs()),
                reason: "连接超时时间不能为0".to_string(),
                suggestion: "设置合理的超时时间，推荐10-30秒".to_string(),
            });
        }

        if self.acquire_timeout.as_secs() == 0 {
            return Err(ConfigError::ValidationError {
                field: "acquire_timeout".to_string(),
                value: format!("{}秒", self.acquire_timeout.as_secs()),
                reason: "获取连接超时时间不能为0".to_string(),
                suggestion: "设置合理的超时时间，推荐5-10秒".to_string(),
            });
        }

        // 验证生命周期配置
        if let Some(max_lifetime) = self.max_lifetime {
            if max_lifetime.as_secs() < 60 {
                return Err(ConfigError::ValidationError {
                    field: "max_lifetime".to_string(),
                    value: format!("{}秒", max_lifetime.as_secs()),
                    reason: "连接最大生命周期过短，可能导致频繁重连".to_string(),
                    suggestion: "设置至少60秒，推荐30分钟到1小时".to_string(),
                });
            }
        }

        Ok(())
    }
}

impl WebSocketPoolConfig {
    /// 创建生产环境优化的WebSocket连接池配置
    ///
    /// 【功能】: 支持百万并发WebSocket连接的配置
    /// 【特性】: 包含负载均衡和故障转移机制
    pub fn production() -> Self {
        Self {
            max_connections: 1_000_000,                  // 支持百万并发连接
            pool_size: 1000,                             // 连接池大小
            heartbeat_interval: Duration::from_secs(30), // 30秒心跳
            connection_timeout: Duration::from_secs(10), // 连接超时
            max_reconnect_attempts: 5,                   // 最大重连次数
            reconnect_interval: Duration::from_secs(2),  // 重连间隔
            enable_load_balancing: true,                 // 启用负载均衡
            enable_failover: true,                       // 启用故障转移
        }
    }

    /// 创建开发环境的WebSocket连接池配置
    ///
    /// 【功能】: 适合开发和测试的配置
    pub fn development() -> Self {
        Self {
            max_connections: 1000,                       // 开发环境较少连接
            pool_size: 50,                               // 小连接池
            heartbeat_interval: Duration::from_secs(60), // 1分钟心跳
            connection_timeout: Duration::from_secs(5),  // 连接超时
            max_reconnect_attempts: 3,                   // 重连次数
            reconnect_interval: Duration::from_secs(1),  // 重连间隔
            enable_load_balancing: false,                // 开发环境关闭负载均衡
            enable_failover: false,                      // 开发环境关闭故障转移
        }
    }

    /// 验证WebSocket连接池配置的有效性
    ///
    /// 【功能】: 检查WebSocket配置参数的合理性
    /// 【返回】: 验证成功返回Ok(())，失败返回ConfigError
    pub fn validate(&self) -> ConfigResult<()> {
        // 验证连接数配置
        if self.max_connections == 0 {
            return Err(ConfigError::ValidationError {
                field: "max_connections".to_string(),
                value: self.max_connections.to_string(),
                reason: "最大WebSocket连接数不能为0".to_string(),
                suggestion: "设置为至少1个连接，推荐开发环境1000个，生产环境1,000,000个"
                    .to_string(),
            });
        }

        if self.pool_size == 0 {
            return Err(ConfigError::ValidationError {
                field: "pool_size".to_string(),
                value: self.pool_size.to_string(),
                reason: "连接池大小不能为0".to_string(),
                suggestion: "设置合理的连接池大小，推荐开发环境50个，生产环境1000个".to_string(),
            });
        }

        // 验证心跳间隔
        if self.heartbeat_interval.as_secs() == 0 {
            return Err(ConfigError::ValidationError {
                field: "heartbeat_interval".to_string(),
                value: format!("{}秒", self.heartbeat_interval.as_secs()),
                reason: "心跳间隔不能为0".to_string(),
                suggestion: "设置合理的心跳间隔，推荐30-60秒".to_string(),
            });
        }

        // 验证连接超时
        if self.connection_timeout.as_secs() == 0 {
            return Err(ConfigError::ValidationError {
                field: "connection_timeout".to_string(),
                value: format!("{}秒", self.connection_timeout.as_secs()),
                reason: "连接超时时间不能为0".to_string(),
                suggestion: "设置合理的超时时间，推荐5-10秒".to_string(),
            });
        }

        // 验证重连配置
        if self.max_reconnect_attempts == 0 {
            return Err(ConfigError::ValidationError {
                field: "max_reconnect_attempts".to_string(),
                value: self.max_reconnect_attempts.to_string(),
                reason: "最大重连次数不能为0".to_string(),
                suggestion: "设置合理的重连次数，推荐3-5次".to_string(),
            });
        }

        if self.reconnect_interval.as_secs() == 0 {
            return Err(ConfigError::ValidationError {
                field: "reconnect_interval".to_string(),
                value: format!("{}秒", self.reconnect_interval.as_secs()),
                reason: "重连间隔不能为0".to_string(),
                suggestion: "设置合理的重连间隔，推荐1-2秒".to_string(),
            });
        }

        Ok(())
    }
}

impl AppConfig {
    /// 从环境变量加载配置，提供默认值 (Associated Function / Constructor)
    ///
    /// 【功能】: 这是创建 `AppConfig` 实例的主要方式。
    ///          它尝试从预定义的环境变量中读取配置值。
    ///          如果某个环境变量未设置，则使用硬编码的默认值。
    /// 【健壮性】: 使用Result类型进行优雅的错误处理，避免panic。
    ///
    /// # 【返回值】
    /// * `-> ConfigResult<Self>`: 返回配置加载结果，成功时包含AppConfig实例，失败时包含详细错误信息。
    pub fn from_env() -> ConfigResult<Self> {
        println!("CONFIG: 正在从环境变量加载配置...");

        // --- 加载 HTTP 服务器地址 ---
        // 1. `std::env::var("HTTP_ADDR")`: 尝试读取名为 "HTTP_ADDR" 的环境变量。
        //    返回 `Result<String, VarError>`。
        // 2. `.unwrap_or_else(|_| ...)`: 如果 `var` 返回 `Err` (变量未找到)，则执行闭包。
        //    闭包返回默认的地址字符串 "127.0.0.1:3000"。
        // 3. `.parse::<SocketAddr>()`: 将获取到的字符串（来自环境变量或默认值）解析为 `SocketAddr` 类型。
        //    返回 `Result<SocketAddr, AddrParseError>`。
        // 4. `.expect("...")`: 如果 `parse` 返回 `Err` (解析失败)，则程序 panic 并显示消息。
        let http_addr_str =
            std::env::var("HTTP_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string()); // 默认 HTTP/1.1 端口
        let http_addr = http_addr_str
            .parse()
            .map_err(|_| ConfigError::EnvVarParseError {
                var_name: "HTTP_ADDR".to_string(),
                value: http_addr_str.clone(),
                expected_type: "SocketAddr (例如: 127.0.0.1:3000)".to_string(),
                suggestion: "请确保格式为 'IP地址:端口号'，例如 '127.0.0.1:3000' 或 '0.0.0.0:8080'"
                    .to_string(),
            })?;
        println!("  - HTTP 地址: {}", http_addr);

        // --- 加载数据库连接 URL ---
        // 逻辑与加载 HTTP 地址类似，但不需要 .parse()，因为我们直接使用 String。
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:task_manager.db?mode=rwc".to_string());

        // 验证数据库URL格式
        if database_url.is_empty() {
            return Err(ConfigError::ValidationError {
                field: "database_url".to_string(),
                value: database_url,
                reason: "数据库URL不能为空".to_string(),
                suggestion: "设置有效的数据库URL，例如 'sqlite:app.db' 或 'postgresql://user:pass@localhost/db'".to_string(),
            });
        }

        println!("  - 数据库 URL: {}", database_url);

        // --- 加载 JWT 密钥 ---
        // 从环境变量获取 JWT 密钥，如果未设置则使用默认值
        // 注意：在生产环境中，应该使用强随机字符串作为 JWT 密钥
        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "your-secret-key-change-in-production".to_string());
        println!("  - JWT 密钥: [已设置]"); // 不打印实际密钥以保证安全

        // --- 【任务13.4新增】加载连接池配置 ---
        // 检测运行环境，决定使用生产环境还是开发环境配置
        let is_production = std::env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase()
            == "production";

        let database_pool = if is_production {
            println!("  - 数据库连接池: 生产环境配置 (最大连接数: 100)");
            DatabasePoolConfig::production()
        } else {
            println!("  - 数据库连接池: 开发环境配置 (最大连接数: 20)");
            DatabasePoolConfig::development()
        };

        let websocket_pool = if is_production {
            println!("  - WebSocket连接池: 生产环境配置 (最大连接数: 1,000,000)");
            WebSocketPoolConfig::production()
        } else {
            println!("  - WebSocket连接池: 开发环境配置 (最大连接数: 1,000)");
            WebSocketPoolConfig::development()
        };

        // 验证配置
        database_pool.validate()?;
        websocket_pool.validate()?;

        // 生产环境安全检查
        if is_production {
            Self::validate_production_security(&jwt_secret, &database_url)?;
        }

        println!("CONFIG: 配置加载完成。");
        // --- 构建并返回 AppConfig 实例 ---
        Ok(Self {
            http_addr,
            database_url,
            jwt_secret,
            database_pool,
            websocket_pool,
        })
    }

    /// 验证生产环境的安全配置
    ///
    /// 【功能】: 检查生产环境必须的安全配置项
    /// 【参数】: jwt_secret - JWT密钥, database_url - 数据库URL
    /// 【返回】: 验证成功返回Ok(())，失败返回ConfigError
    fn validate_production_security(jwt_secret: &str, database_url: &str) -> ConfigResult<()> {
        // 检查JWT密钥强度
        if jwt_secret == "your-secret-key-change-in-production" {
            return Err(ConfigError::ValidationError {
                field: "jwt_secret".to_string(),
                value: "[默认值]".to_string(),
                reason: "生产环境不能使用默认JWT密钥".to_string(),
                suggestion: "设置强随机JWT密钥，至少32个字符，包含字母、数字和特殊字符".to_string(),
            });
        }

        if jwt_secret.len() < 32 {
            return Err(ConfigError::ValidationError {
                field: "jwt_secret".to_string(),
                value: format!("长度: {}", jwt_secret.len()),
                reason: "生产环境JWT密钥长度不足".to_string(),
                suggestion: "JWT密钥至少需要32个字符以确保安全性".to_string(),
            });
        }

        // 检查数据库URL安全性
        if database_url.starts_with("sqlite:") && !database_url.contains("memory") {
            return Err(ConfigError::ValidationError {
                field: "database_url".to_string(),
                value: "SQLite文件数据库".to_string(),
                reason: "生产环境建议使用PostgreSQL等企业级数据库".to_string(),
                suggestion: "考虑使用PostgreSQL: postgresql://user:pass@localhost/dbname"
                    .to_string(),
            });
        }

        Ok(())
    }

    /// 创建带有默认配置的AppConfig实例（用于测试）
    ///
    /// 【功能】: 提供一个安全的默认配置，主要用于测试环境
    /// 【返回】: 返回配置好的AppConfig实例
    pub fn default_for_tests() -> ConfigResult<Self> {
        Ok(Self {
            http_addr: "127.0.0.1:0".parse().unwrap(), // 使用随机端口
            database_url: "sqlite::memory:".to_string(),
            jwt_secret: "test-secret-key-32-characters-long".to_string(),
            database_pool: DatabasePoolConfig::development(),
            websocket_pool: WebSocketPoolConfig::development(),
        })
    }
}
