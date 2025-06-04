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

    /// HTTP/3 (QUIC) 服务器监听地址。
    /// (如果启用 HTTP/3 功能)
    pub http3_addr: SocketAddr,

    /// WebSocket 连接的心跳检测间隔时间 (单位: 秒)。
    /// 用于保持连接活跃并检测断开的连接。
    pub ws_ping_interval: u64,
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

        // --- 加载 HTTP/3 服务器地址 ---
        // 逻辑与加载 HTTP 地址类似。
        let http3_addr = std::env
            ::var("HTTP3_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:3001".to_string()); // 默认 HTTP/3 端口
        let http3_addr = http3_addr
            .parse()
            .expect("环境变量 HTTP3_ADDR 必须是有效的 SocketAddr (例如 '127.0.0.1:3001')");
        println!("  - HTTP/3 地址: {}", http3_addr);

        // --- 加载 WebSocket 心跳间隔 ---
        // 1. `std::env::var("WS_PING_INTERVAL")`: 尝试读取环境变量。
        // 2. `.map(|v| ...)`: 如果 `var` 返回 `Ok(v)` (其中 `v` 是 String)，则执行闭包。
        //    闭包尝试将字符串 `v` 解析为 `u64`。
        //    `.parse::<u64>().expect("...")`: 如果解析失败，程序 panic。
        //    `.map` 返回 `Option<Result<u64, _>>` 或类似，但这里的 `expect` 简化了它为 `Option<u64>` (如果成功)。
        // 3. `.unwrap_or(30)`: 如果 `map` 返回 `None` (因为 `var` 返回 `Err`，或者内部 `parse` 失败后被处理了，这里简化假设)，
        //    则使用默认值 `30`。
        // 【更健壮的方式】: 分开处理 `var` 的 `Err` 和 `parse` 的 `Err`。
        let ws_ping_interval = std::env
            ::var("WS_PING_INTERVAL") // 尝试读取环境变量
            .map(|v| { v.parse::<u64>().expect("环境变量 WS_PING_INTERVAL 必须是有效的正整数") })
            .unwrap_or(30); // 如果未设置或解析失败，默认为 30 秒
        println!("  - WebSocket Ping 间隔: {} 秒", ws_ping_interval);

        println!("CONFIG: 配置加载完成。");
        // --- 构建并返回 AppConfig 实例 ---
        Self {
            http_addr,
            http3_addr,
            ws_ping_interval,
        }
    }

    /// 获取 WebSocket 心跳间隔对应的 `Duration` (Method)
    ///
    /// 【功能】: 提供一个方便的方法，将配置中存储的 `u64` 秒数转换为标准库中的 `Duration` 类型。
    /// 【用途】: 在需要设置定时器或超时的地方（例如 WebSocket 的 ping 任务）使用。
    ///
    /// # 【参数】
    /// * `&self`: 接收一个对 `AppConfig` 实例的【不可变引用】。[[Rust语法特性/概念: 方法, &self]]
    ///
    /// # 【返回值】
    /// * `-> Duration`: 返回对应的 `Duration` 值。
    pub fn ws_ping_interval(&self) -> Duration {
        // `Duration::from_secs` 是 `std::time::Duration` 提供的关联函数，用于从秒数创建 Duration。
        Duration::from_secs(self.ws_ping_interval)
    }
}
