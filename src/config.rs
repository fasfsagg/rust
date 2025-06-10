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
use std::net::SocketAddr; // 用于表示 IP 地址和端口号
use std::time::Duration; // 用于表示时间间隔

// --- 配置结构体定义 ---

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
}

// --- 配置加载实现 ---

impl AppConfig {
    /// 从环境变量加载配置，提供默认值 (Associated Function / Constructor)
    ///
    /// 【功能】: 这是创建 `AppConfig` 实例的主要方式。
    ///          它尝试从预定义的环境变量中读取配置值。
    ///          如果某个环境变量未设置，则使用硬编码的默认值。
    /// 【健壮性】: 当前实现使用了 `.expect()`，这在解析失败时会导致程序崩溃 (panic)。
    ///            在生产环境中，应该返回 `Result<AppConfig, ConfigError>` 并进行更优雅的错误处理。
    ///
    /// # 【返回值】
    /// * `-> Self`: 返回一个初始化好的 `AppConfig` 实例。`Self` 是 `AppConfig` 的类型别名。
    pub fn from_env() -> Self {
        println!("CONFIG: 正在从环境变量加载配置...");

        // --- 加载 HTTP 服务器地址 ---
        // 1. `std::env::var("HTTP_ADDR")`: 尝试读取名为 "HTTP_ADDR" 的环境变量。
        //    返回 `Result<String, VarError>`。
        // 2. `.unwrap_or_else(|_| ...)`: 如果 `var` 返回 `Err` (变量未找到)，则执行闭包。
        //    闭包返回默认的地址字符串 "127.0.0.1:3000"。
        // 3. `.parse::<SocketAddr>()`: 将获取到的字符串（来自环境变量或默认值）解析为 `SocketAddr` 类型。
        //    返回 `Result<SocketAddr, AddrParseError>`。
        // 4. `.expect("...")`: 如果 `parse` 返回 `Err` (解析失败)，则程序 panic 并显示消息。
        let http_addr = std::env::var("HTTP_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string()); // 默认 HTTP/1.1 端口
        let http_addr = http_addr
            .parse()
            .expect("环境变量 HTTP_ADDR 必须是有效的 SocketAddr (例如 '127.0.0.1:3000')");
        println!("  - HTTP 地址: {}", http_addr);

        // --- 加载数据库连接 URL ---
        // 逻辑与加载 HTTP 地址类似，但不需要 .parse()，因为我们直接使用 String。
        let database_url = std::env
            ::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:task_manager.db?mode=rwc".to_string());
        println!("  - 数据库 URL: {}", database_url);

        println!("CONFIG: 配置加载完成。");
        // --- 构建并返回 AppConfig 实例 ---
        Self {
            http_addr,
            database_url,
        }
    }
}
