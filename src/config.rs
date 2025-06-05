// 文件路径: src/config.rs

// /--------------------------------------------------------------------------------------------------\
// |                                      【模块功能图示】 (config.rs)                                  |
// |--------------------------------------------------------------------------------------------------|
// |                                                                                                  |
// | [环境变量 (Environment Variables)]                                                                 |
// |   - HTTP3_ADDR (可选, 例: "127.0.0.1:3001")                                                       |
// |   - DATABASE_URL (可选, 例: "sqlite:./db/app.db")                                                |
// |   - WS_PING_INTERVAL (可选, 例: "60")                                                             |
// |   - JWT_SECRET (可选, 例: "your-very-secure-secret")                                             |
// |      |                                                                                           |
// |      V                                                                                           |
// | [AppConfig::from_env() 函数]                                                                       |
// |   - 读取每个环境变量                                                                               |
// |   - 若未设置，则使用内部定义的默认值                                                                 |
// |   - 解析字符串为目标类型 (SocketAddr, String, u64)                                                 |
// |      |                                                                                           |
// |      V                                                                                           |
// | [AppConfig 结构体实例]                                                                              |
// |   - http3_addr: SocketAddr                                                                       |
// |   - database_url: String                                                                         |
// |   - ws_ping_interval: u64                                                                        |
// |   - jwt_secret: String                                                                           |
// |                                                                                                  |
// \--------------------------------------------------------------------------------------------------/
//
// 【模块核心职责】 (Module Core Responsibilities)
// 1. **定义配置结构 (Define Configuration Structure)**: `AppConfig` 结构体定义了应用程序运行所需的所有配置参数的类型和字段。
// 2. **加载配置值 (Load Configuration Values)**: `AppConfig::from_env()` 函数负责从环境变量中读取这些配置项。
//    如果环境变量未设置，则提供合理的默认值，确保程序总能获得一套完整的配置。
// 3. **提供配置访问 (Provide Configuration Access)**: 一旦 `AppConfig` 实例被创建，它就可以在应用程序的其他部分共享和使用，
//    使得各模块能够访问到它们需要的配置信息。
//
// 【关键技术点】 (Key Technologies)
// - **结构体 (`struct AppConfig`)**: Rust 的结构体用于创建自定义数据类型，将相关的配置字段组合在一起。
// - **数据类型 (`String`, `SocketAddr`, `u64`)**:
//   - `String`: 用于存储文本数据，如数据库 URL、JWT 密钥。它是一个可增长的、UTF-8 编码的字符串。
//     内存布局: `String` 在栈上存储一个指向堆上实际字符数据的指针、当前长度和容量。
//     (Stack: [ptr, len, capacity] -> Heap: [char data...])
//   - `SocketAddr`: 标准库类型，用于表示一个网络套接字地址 (IP 地址 + 端口号)。
//   - `u64`: 无符号64位整数，用于存储像时间间隔这样的数值。
// - **环境变量读取 (`std::env::var`)**: Rust 标准库提供的函数，用于从运行环境中读取环境变量的值。
// - **默认值提供**: 使用 `.unwrap_or_else()` 或 `.unwrap_or()` 方法为未设置的环境变量提供默认值。
// - **字符串到特定类型的解析 (`.parse()`)**:
//   - 许多标准库类型 (如 `SocketAddr`, `u64`) 实现了 `std::str::FromStr` trait。
//   - `.parse()` 方法利用 `FromStr` trait 将字符串转换为这些特定类型。如果解析失败，会返回一个错误。
// - **错误处理 (`.expect()`)**: 一种简单的错误处理方式。如果 `Result` 是 `Err` 或 `Option` 是 `None`，程序会立即 panic (崩溃) 并显示提供的消息。
//   在生产代码中，通常会用更优雅的方式处理错误 (如 `match` 或 `?` 配合返回 `Result`)，而不是直接 panic。
// - **派生宏 (`#[derive(Debug, Clone)]`)**:
//   - `Debug`: 自动实现 `std::fmt::Debug` trait，允许使用 `{:?}` 格式化符号打印结构体实例，方便调试。
//   - `Clone`: 自动实现 `Clone` trait，允许创建结构体实例的副本。

// --- 导入标准库中的类型 ---
// `use` 关键字用于将其他模块或库中的项引入当前作用域。
// `std::net::SocketAddr`: 从标准库 (`std`) 的网络模块 (`net`) 中导入 `SocketAddr` 类型。
//   `SocketAddr` 用于表示网络套接字地址，即 IP 地址和端口号的组合。
//   例如: "127.0.0.1:8080" (IPv4) 或 "[::1]:8080" (IPv6)。
use std::net::SocketAddr;
// `std::time::Duration`: 从标准库 (`std`) 的时间模块 (`time`) 中导入 `Duration` 类型。
//   `Duration` 用于表示一个时间段，例如几秒钟、几毫秒。
use std::time::Duration;

// --- 配置结构体定义 ---

// `#[derive(Debug, Clone)]` 是一个【属性宏 (attribute macro)】。
// 它会自动为 `AppConfig` 结构体实现一些标准的 trait (特性)。
// - `Debug`: 实现 `std::fmt::Debug` trait。
//   这允许我们使用调试格式化打印 `AppConfig` 实例，例如 `println!("{:?}", config_instance);`。
//   对于调试非常有用，可以方便地查看配置对象的当前值。
// - `Clone`: 实现 `Clone` trait。
//   这允许我们创建 `AppConfig` 实例的副本 (深拷贝)。例如 `let config_copy = original_config.clone();`。
//   在多线程或多模块共享配置时，如果不想通过引用共享 (例如使用 `Arc`)，或者需要修改副本而不影响原始配置时，克隆就很有用。
//   Axum 应用状态 `AppState` 通常需要实现 `Clone`，如果 `AppConfig` 是 `AppState` 的一部分，那么 `AppConfig` 也需要 `Clone`。
#[derive(Debug, Clone)]
// `pub struct AppConfig { ... }`: 定义一个公共的 (public) 结构体 `AppConfig`。
// - `pub`: 表示这个结构体可以被项目中的其他模块访问。
// - `struct`: 关键字，用于定义结构体。结构体是一种自定义数据类型，允许我们将多个相关的值组合成一个有意义的单元。
//   可以把它看作是一个对象的蓝图或模板。
pub struct AppConfig {
    // `http_addr` 字段已被移除，因为项目现在专注于纯 HTTP/3 服务。
    // 原注释:
    // /// HTTP 服务器监听地址。
    // /// `SocketAddr` 结构体结合了 IP 地址和端口号。
    // pub http_addr: SocketAddr,

    // `pub http3_addr: SocketAddr,`: 定义一个名为 `http3_addr` 的公共字段。
    // - `pub`: 表示这个字段可以从结构体外部直接访问 (例如 `config.http3_addr`)。
    // - `http3_addr`: 字段名，遵循 Rust 的 snake_case 命名约定。
    // - `SocketAddr`: 字段类型。这个字段将存储 HTTP/3 (基于 QUIC) 服务器监听的网络地址。
    //   例如，它可以是 "127.0.0.1:3001"。
    /// HTTP/3 (QUIC) 服务器监听地址。
    /// 这是应用程序提供服务的主要网络地址。
    pub http3_addr: SocketAddr,

    // `pub ws_ping_interval: u64,`: 定义一个名为 `ws_ping_interval` 的公共字段。
    // - `u64`: 字段类型，表示一个64位无符号整数。
    //   用于存储 WebSocket 连接的心跳检测间隔时间，单位是秒。
    //   心跳检测用于保持连接活跃，并在连接意外断开时及时发现。
    /// WebSocket 连接的心跳检测间隔时间 (单位: 秒)。
    /// 用于保持连接活跃并检测断开的连接。
    pub ws_ping_interval: u64,

    // `pub database_url: String,`: 定义一个名为 `database_url` 的公共字段。
    // - `String`: 字段类型，表示一个可增长的、UTF-8 编码的字符串。
    //   用于存储数据库的连接字符串/URL。
    //   例如: "sqlite:./db/app.db" (SQLite 文件数据库) 或 "postgres://user:pass@host/database" (PostgreSQL)。
    /// 数据库连接字符串。
    /// 例如: "sqlite:./db/app.db" 或 "postgres://user:pass@host/database"
    pub database_url: String,

    // `pub jwt_secret: String,`: 定义一个名为 `jwt_secret` 的公共字段。
    // - `String`: 字段类型。
    //   用于存储 JWT (JSON Web Token) 签名和验证所使用的密钥。
    //   这个密钥非常重要，必须保密，并且应该具有足够的复杂度以确保安全。
    //   在生产环境中，强烈建议从安全的环境变量或配置文件中加载，而不是硬编码。
    /// JWT (JSON Web Token) 签名和验证所使用的密钥。
    /// **警告**: 这是一个敏感值，务必保证其安全！
    pub jwt_secret: String,
}

// --- 配置加载实现 ---

// `impl AppConfig { ... }`: 为 `AppConfig` 结构体实现方法和关联函数。
// `impl` 是 "implementation" (实现) 的缩写。
impl AppConfig {
    // `pub fn from_env() -> Self`: 定义一个公共的关联函数 `from_env`。
    // - `pub fn`: 表示这是一个公共函数。
    // - `from_env`: 函数名。关联函数通常用作构造函数或工厂方法。
    // - `-> Self`: 返回类型。`Self` (大写S) 是一个特殊的类型别名，指代当前 `impl` 块所针对的类型，即 `AppConfig`。
    //   所以这个函数返回一个 `AppConfig` 的实例。
    //   这个函数不接收 `&self` 或 `&mut self` 作为第一个参数，因此它是一个关联函数，而不是方法。
    //   调用方式是 `AppConfig::from_env()`。
    /// 从环境变量加载配置，如果环境变量未设置，则使用预设的默认值。
    /// 这是创建 `AppConfig` 实例的主要方式。
    ///
    /// 【健壮性说明】:
    /// 当前实现中，如果环境变量存在但其值无法被正确解析 (例如，期望一个数字但提供了一个非数字字符串)，
    /// `.expect()` 会导致程序立即崩溃 (panic)。
    /// 在生产级应用中，通常会返回 `Result<AppConfig, YourConfigErrorType>` 并进行更优雅的错误处理，
    /// 而不是直接 panic，以便程序可以记录错误、尝试恢复或以更可控的方式退出。
    pub fn from_env() -> Self {
        // `println!` 是一个宏，用于向控制台输出信息。这里用于指示配置加载过程的开始。
        println!("CONFIG: 正在从环境变量加载配置...");

        // HTTP/1.1 服务器地址 (`HTTP_ADDR`) 的加载逻辑已被移除，因为项目现在专注于纯 HTTP/3。

        // --- 加载 HTTP/3 服务器地址 (`HTTP3_ADDR`) ---
        // `std::env::var("HTTP3_ADDR")`: 尝试从环境中读取名为 "HTTP3_ADDR" 的环境变量。
        //   - `std::env::var` 返回一个 `Result<String, std::env::VarError>`。
        //     - `Ok(String)`: 如果环境变量存在，返回其值的字符串形式。
        //     - `Err(VarError)`: 如果环境变量不存在 (或者由于权限等原因无法读取)。
        let http3_addr_str = std::env::var("HTTP3_ADDR")
            // `.unwrap_or_else(|err| ...)`: 处理 `Result`。
            //   - 如果 `std::env::var` 返回 `Ok(value)`，则 `unwrap_or_else` 返回此 `value`。
            //   - 如果返回 `Err(err)`，则执行闭包 `|err| ...`。
            //     - `|err|`: 闭包参数，`err` 是 `std::env::VarError`。这里我们用 `_` 忽略它，因为我们只关心它不存在的情况。
            //     - `"127.0.0.1:3001".to_string()`: 如果环境变量未设置，则返回这个默认的 IP 地址和端口字符串。
            //       `.to_string()` 将字符串字面量 (类型 `&str`) 转换为 `String` 类型。
            .unwrap_or_else(|_| "127.0.0.1:3001".to_string()); // HTTP/3 默认监听地址和端口

        // `http3_addr_str.parse()`: 将加载到的字符串 (`http3_addr_str`) 解析为 `SocketAddr` 类型。
        //   - `.parse()` 方法是 `std::str::FromStr` trait 的一部分。许多类型 (如 `SocketAddr`, `i32`, `f64`) 都实现了 `FromStr`。
        //   - 它返回一个 `Result<SocketAddr, AddrParseError>` (其中 `AddrParseError` 是解析失败时返回的错误类型)。
        let http3_addr: SocketAddr = http3_addr_str
            .parse()
            // `.expect("...")`: 处理 `Result`。
            //   - 如果 `.parse()` 返回 `Ok(value)`，则 `expect` 返回此 `value` (`SocketAddr` 实例)。
            //   - 如果 `.parse()` 返回 `Err(error_details)`，则程序会立即 panic (崩溃)，并显示我们提供的错误消息字符串。
            //     这是一种简单粗暴的错误处理，适用于开发或示例，但在生产中应避免。
            .expect("环境变量 HTTP3_ADDR 必须是有效的 SocketAddr (例如 '127.0.0.1:3001')");
        // `println!` 输出加载到的 HTTP/3 地址。`{}` 是格式化占位符。
        println!("  - HTTP/3 地址: {}", http3_addr);


        // --- 加载 WebSocket 心跳间隔 (`WS_PING_INTERVAL`) ---
        // `std::env::var("WS_PING_INTERVAL")`: 尝试读取 "WS_PING_INTERVAL" 环境变量。
        let ws_ping_interval_str_opt = std::env::var("WS_PING_INTERVAL"); // 返回 Result<String, VarError>

        // `.map(|v_str| ...)`: 如果 `ws_ping_interval_str_opt` 是 `Ok(v_str)` (即环境变量存在且值为 `v_str`)，
        // 则对 `v_str` 执行闭包内的代码。如果环境变量不存在 (`Err`)，则 `.map` 不会执行闭包，并保持 `Err` 状态。
        //   - `v_str.parse::<u64>()`: 尝试将字符串 `v_str` 解析为 `u64` (无符号64位整数) 类型。
        //     返回 `Result<u64, std::num::ParseIntError>`。
        //   - `.expect("...")`: 如果解析失败 (例如字符串不是有效数字)，则 panic。
        // 这里的 `.map(...).unwrap_or(30)` 组合有些微妙：
        // - 如果 `WS_PING_INTERVAL` 未设置, `var()` 返回 `Err`, `map` 不执行, `unwrap_or(30)` 对 `Err` 无效 (类型不匹配)。
        //   正确的做法是先处理 `var()` 的 `Result`，再处理 `parse()` 的 `Result`，或者使用更复杂的组合。
        //   一个更清晰的方式是:
        //   ```rust
        //   let ws_ping_interval = match std::env::var("WS_PING_INTERVAL") {
        //       Ok(val_str) => val_str.parse::<u64>().expect("WS_PING_INTERVAL must be a positive integer"),
        //       Err(_) => 30, // Default value if env var is not set
        //   };
        //   ```
        //   当前代码 `map().unwrap_or()` 依赖于 `Result` 和 `Option` 之间的某种转换或特定行为，
        //   如果 `var` 返回 `Err`，则 `map` 不会被调用，而 `unwrap_or` 如果用在 `Result` 上，通常期望 `Result<T, E>` 转换为 `T`。
        //   更标准的做法是：
        let ws_ping_interval = std::env::var("WS_PING_INTERVAL")
            .ok() // 将 Result<String, VarError> 转换为 Option<String>，忽略错误细节
            .and_then(|val_str| val_str.parse::<u64>().ok()) // 如果 Some(val_str)，尝试解析，将 Result<u64, ParseError> 转为 Option<u64>
            .unwrap_or(30); // 如果任何步骤产生 None (未设置或解析失败)，则使用默认值 30。
        println!("  - WebSocket Ping 间隔: {} 秒", ws_ping_interval);


        // --- 加载数据库连接字符串 (`DATABASE_URL`) ---
        // 逻辑与加载 `HTTP3_ADDR` 类似。
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:./db/app.db".to_string()); // 默认使用 SQLite 文件数据库
        println!("  - 数据库 URL: {}", database_url);

        // --- 加载 JWT 密钥 (`JWT_SECRET`) ---
        // 这个密钥对于应用的安全性至关重要。
        // **警告**: 在生产环境中，绝不应使用硬编码的默认密钥。必须通过环境变量或安全的配置文件提供一个强密钥。
        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| {
                // 在开发环境中，如果未设置 JWT_SECRET，打印一个强烈的警告并使用一个预定义的、不安全的密钥。
                // 这样做是为了方便本地运行和测试，但强调了其不适用于生产。
                println!("⚠️  警告: 环境变量 JWT_SECRET 未设置! 使用了一个【极不安全】的默认开发密钥。请在生产环境中务必设置强密钥!");
                "unsafe-default-dev-jwt-secret-replace-this-!@#$%^&*()".to_string()
            });
        // 不直接打印 jwt_secret 的值到日志中，以避免意外泄露。
        // 可以打印一个指示它是否被设置的消息。
        if jwt_secret == "unsafe-default-dev-jwt-secret-replace-this-!@#$%^&*()" && std::env::var("JWT_SECRET").is_err() {
             println!("  - JWT 密钥: 使用了开发模式下的默认密钥 (不安全!)");
        } else {
             println!("  - JWT 密钥: 已从环境变量加载。");
        }


        // 日志记录：配置加载完成。
        println!("CONFIG: 配置加载完成。");

        // --- 构建并返回 AppConfig 实例 ---
        // `Self { ... }`: 使用结构体字面量 (struct literal) 创建 `AppConfig` (即 `Self`) 的一个新实例。
        //   - `http3_addr: http3_addr,` (可以简写为 `http3_addr,` 如果字段名和变量名相同)
        //   - 将之前加载或默认设置的变量值赋给相应的结构体字段。
        Self {
            // http_addr, // 已移除
            http3_addr,
            ws_ping_interval,
            database_url,
            jwt_secret, // 新增 jwt_secret 字段
        }
    }

    // `pub fn ws_ping_interval(&self) -> Duration`: 定义一个名为 `ws_ping_interval` 的公共方法。
    // - `&self`: 第一个参数是 `&self`，表示这是一个方法，它借用 (borrows) 当前 `AppConfig` 实例的不可变引用。
    //   调用方式是 `config_instance.ws_ping_interval()`。
    // - `-> Duration`: 返回类型是 `std::time::Duration`。
    /// 获取 WebSocket 心跳间隔对应的 `Duration` 类型。
    ///
    /// 【功能】: 提供一个方便的方法，将配置中以 `u64` 秒数存储的 `ws_ping_interval`
    ///          转换为标准库中表示精确时间跨度的 `Duration` 类型。
    /// 【用途】: 在需要设置定时器或超时的地方（例如 WebSocket 的 ping 任务）使用，
    ///          因为这些功能通常需要 `Duration`类型的参数。
    ///
    /// # 【参数】
    /// * `&self`: 接收一个对 `AppConfig` 实例的【不可变引用】。
    ///   - `&`: 表示借用，函数不会获取 `AppConfig` 实例的所有权。
    ///   - `self`: 指代调用此方法的 `AppConfig` 实例。
    ///   [[Rust语法特性/概念: 方法, &self, 不可变借用]]
    ///
    /// # 【返回值】
    /// * `-> Duration`: 返回一个 `std::time::Duration` 值。
    pub fn ws_ping_interval(&self) -> Duration {
        // `Duration::from_secs(seconds)` 是 `std::time::Duration` 类型提供的一个关联函数 (构造函数)，
        // 用于从一个 `u64` 类型的秒数值创建一个 `Duration` 实例。
        // `self.ws_ping_interval`: 通过 `self` 访问当前 `AppConfig` 实例的 `ws_ping_interval` 字段值。
        Duration::from_secs(self.ws_ping_interval)
    }
}

[end of src/config.rs]
