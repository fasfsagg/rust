// 文件路径: src/main.rs

// /--------------------------------------------------------------------------------------------------\
// |                                      【模块功能图示】 (main.rs)                                   |
// |--------------------------------------------------------------------------------------------------|
// |                                                                                                  |
// | [程序入口 main()]                                                                                  |
// |      |                                                                                           |
// |      V                                                                                           |
// | [加载配置 AppConfig::from_env()] (读取环境变量, 如服务器地址、数据库URL)                             |
// |      |                                                                                           |
// |      V                                                                                           |
// | [初始化应用 startup::init_app()] (设置日志, 初始化数据库连接, 创建 Axum 路由)                        |
// |      | (返回 Axum Router 实例)                                                                    |
// |      V                                                                                           |
// | [启动 HTTP/3 服务器 start_http3_server()] (配置 QUIC, TLS, 循环处理 H3 连接和请求)                 |
// |      |                                                                                           |
// |      V                                                                                           |
// | [服务器运行中...]                                                                                 |
// |                                                                                                  |
// \--------------------------------------------------------------------------------------------------/
//
// 【模块核心职责】 (Module Core Responsibilities)
// 1. **程序入口 (Program Entry Point)**: `main` 函数是整个应用程序的起点。
// 2. **配置加载 (Configuration Loading)**: 从环境变量或配置文件中加载应用运行所需的参数。
// 3. **应用初始化 (Application Initialization)**: 协调并执行应用核心组件的初始化，例如日志系统、数据库连接、Web 路由等。
// 4. **HTTP/3 服务器启动 (HTTP/3 Server Startup)**: 启动并管理基于 QUIC 的 HTTP/3 服务器，使其能够接收和处理客户端请求。
//
// 【关键技术点】 (Key Technologies)
// - **Tokio (`#[tokio::main]`)**: 一个异步运行时 (Asynchronous Runtime)，为 Rust 程序提供执行异步代码（`async/await`）的环境。它管理任务调度、I/O 操作（如网络）、定时器等。
// - **Axum (`axum::Router`)**: 一个现代、符合人体工程学的 Rust Web 框架，用于构建 Web 服务。本项目使用其路由功能。
// - **Quinn (`quinn::Endpoint`, `quinn::ServerConfig`)**: 一个纯 Rust 实现的 QUIC 协议库。QUIC 是 HTTP/3 的基础传输协议，提供加密、多路复用和更快的连接建立。
// - **H3 (`h3::server`, `h3_quinn`)**: HTTP/3 协议的 Rust 实现库，以及它与 Quinn 的集成层。
// - **Rustls (`rustls::ServerConfig`)**: 一个现代的 TLS (Transport Layer Security) 库，用于 QUIC 连接的加密。
// - **RCGen (`rcgen::Certificate`)**: 用于在开发环境中动态生成自签名 TLS 证书。
// - **HTTP/3 概念**: 基于 QUIC، 头部压缩 (QPACK), 多路复用流。
// - **异步编程 (`async/await`)**: Rust 中用于编写非阻塞代码的特性，对于网络服务器这类 I/O 密集型应用至关重要。
// - **错误处理 (`Result<T, E>`, `Box<dyn Error>`)**: Rust 标准的错误处理机制。`Box<dyn Error>` 用于处理多种不同类型的错误。
// - **模块系统 (`mod`)**: Rust 用于组织代码、控制可见性的方式。

// --- 导入标准库和第三方库的特定功能 ---
// `use` 关键字用于将特定路径下的项（如函数、结构体、枚举、trait 等）引入到当前作用域，方便直接使用。

// `std::net::SocketAddr`: 用于表示 IP 地址和端口号的组合，例如 "127.0.0.1:3001"。
use std::net::SocketAddr;
// `std::convert::Infallible`: 一个特殊的错误类型，表示某个操作永远不会失败。在某些 Axum API 中作为错误类型出现。
use std::convert::Infallible;
// `std::sync::Arc`: 原子引用计数指针 (Atomically Referenced Counter)。允许多个所有者安全地共享同一份数据。
// 当最后一个 Arc 指针被销毁时，数据才会被清理。对于在异步任务间共享数据非常有用。
use std::sync::Arc;


// `axum::Router`: Axum 框架的核心组件，用于定义路由规则，将 HTTP 请求映射到相应的处理函数。
use axum::Router;
// `axum::body::Body as AxumBody`: Axum 中用于表示 HTTP 请求体和响应体的类型。我们给它起个别名 `AxumBody` 以便区分。
use axum::body::Body as AxumBody;


// `bytes::Bytes`: 一个高效的字节数组类型，常用于网络编程中处理数据块。
use bytes::Bytes;


// `futures_util::stream::{Stream, StreamExt, TryStreamExt}`: 提供操作异步流 (Stream) 的工具函数和 trait。
// `Stream` trait 类似于同步代码中的迭代器 (Iterator)，但用于异步产生值。
// `StreamExt` 和 `TryStreamExt` 为实现了 `Stream` 的类型添加了许多有用的方法 (例如 `.map()`, `.filter()`, `.next()`, `.try_next()`)。
use futures_util::stream::{Stream, StreamExt, TryStreamExt};


// `h3::server::RequestStream as H3RequestStream`: `h3` 库中代表 HTTP/3 请求流的类型。
// 它用于接收请求体数据和发送响应数据。我们给它起个别名 `H3RequestStream`。
use h3::server::RequestStream as H3RequestStream;
// `h3_quinn::quinn as h3_quinn_compat`: `h3-quinn` crate 重新导出的 `quinn` crate。
// 使用别名 `h3_quinn_compat` 可以明确这是与 `h3` 兼容的 `quinn` 版本。
use h3_quinn::quinn as h3_quinn_compat;


// `http` crate 提供了 HTTP 相关的基本类型，如 `Request`, `Response`, `HeaderMap`, `Method`, `Uri`, `Version`。
// Axum 和其他 HTTP 相关的库都基于这个 `http` crate。
// 我们使用 `HttpRequest` 和 `HttpResponse` 作为别名，以区分它们是标准的 HTTP 类型。
use http::{Request as HttpRequest, Response as HttpResponse, HeaderMap, Method, Uri, Version as HttpVersion};
// `http_body_util::{BodyExt, StreamBody, Frame}`: `http-body-util` 提供处理 HTTP body 的实用工具。
// `BodyExt` 为 `http_body::Body` trait 的实现者添加了便利方法 (如 `.data()`, `.collect()`)。
// `StreamBody` 可以将一个实现了 `Stream` 的类型包装成 `http_body::Body`。
// `Frame` 代表 HTTP body 中的数据帧或 trailers 帧。
use http_body_util::{BodyExt, StreamBody, Frame};


// `quinn::{Endpoint, ServerConfig as QuinnServerConfig}`: `quinn` 库的核心类型。
// `Endpoint` 代表一个 QUIC 端点，可以监听传入连接或发起传出连接。
// `ServerConfig` 用于配置 QUIC 服务器的行为，例如 TLS 设置。我们将其重命名为 `QuinnServerConfig` 以避免与可能的其他 `ServerConfig` 冲突。
use quinn::{Endpoint, ServerConfig as QuinnServerConfig};


// `rcgen`: 用于生成自签名 X.509 证书，主要用于开发和测试。
use rcgen;
// `rustls`: 一个现代的 TLS (Transport Layer Security) 库，用于提供安全的通信。
use rustls;
// `rustls_pemfile`: 用于解析 PEM 格式的证书和私钥文件。
use rustls_pemfile;


// `tracing::info` 和 `tracing::error`: `tracing` crate 提供的日志宏，用于记录不同级别的应用事件。
// `info!` 用于记录常规信息，`error!` 用于记录错误事件。
use tracing::{info, error};


// --- 声明项目根模块 ---
// `mod` 关键字用于声明模块。Rust 编译器会在相应的文件或目录中查找这些模块的定义。
// 例如，`mod app;` 会查找 `app.rs` 或 `app/mod.rs`。
// `mod` 关键字告诉 Rust 编译器查找并包含这些模块文件或目录。
// 这是 Rust 模块系统的基础，用于组织代码。
mod app;     // 核心应用逻辑模块 (./app/mod.rs or ./app.rs)
mod config;  // 应用配置模块 (./config.rs)
mod db;      // 数据库交互模块 (./db.rs)
mod error;   // 自定义错误处理模块 (./error.rs)
mod routes;  // API 路由定义模块 (./routes.rs)
mod startup; // 应用启动与初始化模块 (./startup.rs)

// --- 主函数 (程序入口) ---

// `#[tokio::main]` 是一个【属性宏 (attribute macro)】。
// 宏是一种在编译时扩展或修改代码的方式。`#[tokio::main]` 的作用是：
// 1. **转换 `main` 函数**: 它将 `async fn main()` 异步函数转换为一个普通的 `fn main()` 同步函数。
// 2. **设置 Tokio 运行时**: 在生成的 `fn main()` 内部，它会自动创建并启动一个 Tokio 多线程运行时环境 (`tokio::runtime::Runtime`)。
// 3. **执行异步代码**: 它调用 `runtime.block_on(async_main_code)` 来在运行时中执行我们原来的 `async fn main()` 中的代码。
//
// 简化概念上的扩展可能如下 (实际实现更复杂):
// fn main() {
//     let mut runtime = tokio::runtime::Builder::new_multi_thread()
//         .enable_all() // 启用所有 Tokio 功能 (I/O, time, etc.)
//         .build()
//         .unwrap(); // .unwrap() 表示如果构建运行时失败则程序 panic
//
//     runtime.block_on(async { // block_on 会阻塞当前线程直到异步代码完成
//         // ... 我们在 async fn main() 中写的代码 ...
//         // 例如:
//         // let config = config::AppConfig::from_env();
//         // let app = startup::init_app(config.clone()).await;
//         // start_http3_server(config.http3_addr, app.clone()).await;
//     });
// }
//
// 这样，我们就可以在 `main` 函数中直接使用 `async/await` 语法，而 Tokio 会负责管理底层的异步操作。
#[tokio::main]
async fn main() { // `async fn` 表示这是一个异步函数，其执行可以被暂停和恢复，而不会阻塞整个线程。
    // --- 步骤 1: 加载应用程序配置 ---
    // `config::AppConfig::from_env()`: 调用 `config` 模块中 `AppConfig` 结构体的关联函数 `from_env`。
    // 这个函数负责从环境变量中读取配置项 (例如服务器监听地址、数据库连接字符串等) 并返回一个 `AppConfig` 实例。
    // `let config = ...`: `let` 关键字用于声明一个变量 `config`，它将持有 `AppConfig` 实例。
    // Rust 是静态类型的，编译器会自动推断 `config` 的类型为 `AppConfig`。
    let config = config::AppConfig::from_env();
    // 此时，`config` 变量包含了程序运行所需的所有配置信息。
    // 例如 `config.http3_addr` 就是 HTTP/3 服务器要监听的地址。

    // --- 步骤 2: 初始化应用程序 ---
    // `startup::init_app(config.clone()).await`: 调用 `startup` 模块中的 `init_app` 异步函数。
    // - `config.clone()`: 如果 `AppConfig` 实现了 `Clone` trait (本项目中是这样)，这里会创建 `config` 的一个副本。
    //   这通常是为了所有权 (ownership)。`init_app` 函数可能会需要拥有配置数据，或者将其传递给其他需要所有权的组件。
    //   克隆 `AppConfig` (如果其内部字段也是可克隆的，如 `String`, `SocketAddr`) 通常是廉价的。
    // - `.await`: 因为 `init_app` 是一个 `async fn` (异步函数)，`.await` 用于暂停当前 `main` 函数的执行，
    //   直到 `init_app` 完成其异步操作（例如，异步地建立数据库连接）并返回结果。
    //   在等待期间，Tokio 运行时可以将当前线程用于执行其他异步任务。
    // `init_app` 函数的职责包括:
    //   - 设置日志系统 (例如使用 `tracing` crate)。
    //   - 初始化数据库连接池 (本项目中使用 SeaORM 连接 SQLite)。
    //   - 创建并配置 Axum 的 `Router`，定义所有 HTTP 路由及其处理函数。
    //   - 组装应用状态 `AppState` (例如包含数据库连接池的 `Arc<DatabaseConnection>`)。
    // `let app = ...`: `init_app` 返回一个配置好的 `axum::Router` 实例，赋值给变量 `app`。
    let app = startup::init_app(config.clone()).await;
    // `app` 现在是一个可以处理 HTTP 请求的 Axum 应用。

    // --- HTTP/1.1 服务器相关代码已被移除 ---
    // 原来的代码会在这里设置并启动一个基于 TCP 的 HTTP/1.1 服务器。
    // 例如:
    // info!("HTTP/1.1 服务器启动，监听地址: http://{}", config.http_addr);
    // let listener = TcpListener::bind(config.http_addr).await.unwrap();
    // axum::serve(listener, app.into_make_service()).await.unwrap();
    // 这部分逻辑已被移除，项目现在专注于纯 HTTP/3 服务。

    // --- 步骤 3: 启动 HTTP/3 服务器 ---
    // `start_http3_server(config.http3_addr, app.clone()).await`: 调用我们下面定义的 `start_http3_server` 异步函数。
    // - `config.http3_addr`: 从配置中获取 HTTP/3 服务器应监听的 `SocketAddr` (IP 地址和端口)。
    // - `app.clone()`: Axum 的 `Router` 类型通常设计为可以廉价克隆的 (内部使用 `Arc` 来共享路由表和状态)。
    //   这里克隆 `app` 是因为 `start_http3_server` 函数需要拥有一个 `Router` 实例。如果后续还需要在 `main` 中使用 `app`，
    //   或者如果 `start_http3_server` 被派生到另一个任务中，克隆是必要的。
    // - `.await`: 暂停 `main` 函数的执行，直到 `start_http3_server` 完成。
    //   实际上，`start_http3_server` 内部会进入一个无限循环来接受连接，所以 `main` 函数会一直在这里等待，
    //   直到服务器被显式停止或发生不可恢复的错误。
    info!("准备启动 HTTP/3 服务器..."); // 使用 tracing
    start_http3_server(config.http3_addr, app.clone()).await;
    // 服务器启动后，`main` 函数的生命周期基本就停在这里，由 `start_http3_server` 内部的循环来处理请求。
}

// --- HTTP/3 服务器实现 ---
// 下面的函数和结构体共同构成了 HTTP/3 服务器的实现。

// `SocketAddr` 已在文件顶部导入。
// `QuinnServerConfig` 和 `Endpoint` 已在文件顶部导入。
// `H3RequestStream` 已在文件顶部导入。
use h3_quinn::quinn as h3_quinn_compat; // Alias for h3_quinn's re-export of quinn
use http::{Request as HttpRequest, Response as HttpResponse, HeaderMap, Method, Uri, Version as HttpVersion};
use axum::Router;
use bytes::Bytes;
use axum::body::Body as AxumBody;
use http_body_util::{BodyExt, StreamBody, Frame}; // Added Frame
use futures_util::stream::{Stream, StreamExt, TryStreamExt}; // Added Stream
use std::convert::Infallible;
use tracing::error; // Ensure tracing::error is available

/// 启动 HTTP/3 服务器 (需要 TLS 证书)
// `async fn start_http3_server(...)`: 定义一个异步函数 `start_http3_server`。
// - `async fn`: 表示这是一个异步函数，其执行可以被暂停和恢复。
// - `addr: SocketAddr`: 第一个参数 `addr`，类型为 `SocketAddr`。这是服务器监听的 IP 地址和端口。
// - `app: Router`: 第二个参数 `app`，类型为 `axum::Router`。这是我们配置好的 Axum 应用，用于处理实际的 HTTP 请求逻辑。
// - `-> Result<(), Box<dyn std::error::Error + Send + Sync>>`: 函数的返回类型。 (此函数修改为不返回Result，错误在内部处理并打印)
//   - `Result<T, E>`: Rust 中标准的错误处理枚举，`Ok(T)` 表示成功并带有值 `T`，`Err(E)` 表示失败并带有错误 `E`。
//   - `()`: 空元组，表示成功时此函数不返回任何有意义的值 (类似 `void` in C/C++)。
//   - `Box<dyn std::error::Error + Send + Sync>`: 这是一个动态错误类型。
//     - `Box<...>`: 将错误存储在堆上，允许不同大小的错误类型被统一处理。
//     - `dyn std::error::Error`: 表示任何实现了 `std::error::Error` trait (Rust 标准错误接口) 的类型。
//     - `Send + Sync`: 这两个是【标记 trait (marker traits)】。
//       - `Send`: 表示该类型的值可以安全地从一个线程发送到另一个线程。
//       - `Sync`: 表示该类型的引用 (`&T`) 可以安全地在多个线程之间共享。
//       - 在异步代码中，特别是当错误可能跨越 `.await` 点 (可能导致任务在不同线程上恢复) 或在多任务间共享时，
//         错误类型通常需要满足 `Send + Sync`。
// (此函数修改为不显式返回Result，错误在内部处理并打印，如果发生致命错误则直接panic或return)
async fn start_http3_server(addr: SocketAddr, app: Router) {
    // 使用 tracing::info! 宏记录服务器启动信息。
    // `https://{}` 中的 `{}` 会被 `addr` 的值替换。
    info!("🚀 启动 HTTP/3 服务器: https://{}", addr);
    info!("   确保浏览器支持 HTTP/3 并且信任所使用的证书 (自签名证书通常需要手动信任)。");

    // --- 步骤 H3-1: 生成或加载 TLS 证书 ---
    // HTTP/3 (QUIC) 强制使用 TLS 加密。
    // `generate_self_signed_cert()` 是我们定义的一个辅助函数，用于在本地开发时动态生成一个自签名证书。
    // `match` 表达式用于处理 `generate_self_signed_cert()` 可能返回的 `Result`。
    let (cert, key) = match generate_self_signed_cert() {
        // `Ok((cert_data, key_data))` 表示证书生成成功。
        // `cert_data` 和 `key_data` 分别是证书和私钥的 PEM 编码字节。
        Ok((cert_data, key_data)) => (cert_data, key_data), // 将提取的值赋给外层作用域的 cert 和 key
        // `Err(e)` 表示证书生成失败，`e` 是错误对象。
        Err(e) => {
            // 使用 tracing::error! 宏记录错误。
            error!("❌ 生成自签名证书失败: {}. HTTP/3 服务器无法启动。", e);
            return; // 提前从函数返回，因为没有证书服务器无法启动。
        }
    };
    info!("   ✓ 已生成自签名 TLS 证书 (用于 HTTP/3)"); // 日志: cert 和 key 变量现在持有证书数据。

    // --- 步骤 H3-2: 配置 Quinn 服务器 (QUIC 层) ---
    // `configure_quinn_server(cert, key)` 是我们定义的辅助函数，用于创建 QUIC 服务器配置。
    // 它需要之前生成的证书和私钥。
    let server_config = match configure_quinn_server(cert, key) {
        Ok(config) => config, // 成功则将配置赋值给 server_config
        Err(e) => {
            error!("❌ 配置 Quinn (QUIC) 服务器失败: {}. HTTP/3 服务器无法启动。", e);
            return; // 配置失败，服务器无法启动。
        }
    };
    info!("   ✓ Quinn (QUIC) 服务器配置完成。"); // server_config 现在持有 Quinn 服务器配置。

    // --- 步骤 H3-3: 创建 Quinn 端点 ---
    // `quinn::Endpoint` 代表一个 QUIC 端点，可以用来接受或发起连接。
    // `Endpoint::server(server_config, addr)` 使用之前的服务器配置和监听地址来创建一个服务器端点。
    // 这个操作也可能失败 (例如地址已被占用)，所以也使用 `match` 处理 `Result`。
    let endpoint = match Endpoint::server(server_config, addr) {
         Ok(ep) => ep, // 成功则将端点赋值给 endpoint
         Err(e) => {
             error!("❌ 创建 Quinn (QUIC) 端点失败: {}. HTTP/3 服务器无法启动。 (地址可能已被占用)", e);
             return; // 创建端点失败，服务器无法启动。
         }
    };
    info!("   ✓ Quinn (QUIC) 端点创建成功，监听地址: {}", addr);
    info!("⚠️  浏览器可能会因自签名证书而显示安全警告。"); // 提示用户自签名证书的问题。

    // --- 步骤 H3-4: 循环接受 QUIC 连接并处理 HTTP/3 请求 ---
    // `endpoint.accept().await`: 异步等待新的传入 QUIC 连接。
    // 这是一个循环 (`while let Some(...)`)，会持续接受新连接，直到端点被关闭或发生错误。
    // `Some(connecting)`: 当一个新的 QUIC 连接尝试建立时，`accept()` 返回 `Some(connecting)`。
    // `connecting` 是一个 `quinn::Connecting` 类型的对象，代表一个正在进行的连接尝试。
    while let Some(connecting) = endpoint.accept().await {
        // 为每个新连接尝试记录日志。`connecting.remote_address()` 获取客户端地址。
        info!("   🔌 接收到新的 QUIC 连接 from {}", connecting.remote_address());

        // `tokio::spawn`: 创建一个新的异步任务 (绿色线程) 来处理这个连接。
        // 这样主循环 (`while let Some(...)`) 就不会被单个连接的处理过程阻塞，可以继续接受其他新连接，实现并发处理。
        // `{ ... }` 是一个代码块，`async move { ... }` 定义了一个异步闭包。
        // `move` 关键字表示闭包会获取其引用的外部变量的所有权。
        // `app.clone()`: Axum 的 `Router` (即 `app`) 需要在每个任务中独立使用。
        // Router 通常是 `Arc` 包裹的，所以 `clone()` 是廉价的（只增加引用计数）。
        tokio::spawn({
            let app_clone = app.clone(); // 克隆 Router 以在任务中使用。
            async move { // `async move` 闭包
                // `connecting.await`: 等待 QUIC 握手完成，建立真正的 `quinn::Connection`。
                // 这个操作也可能失败。
                match connecting.await {
                    // `Ok(connection)`: QUIC 连接成功建立。`connection` 是 `quinn::Connection`。
                    Ok(connection) => {
                        info!("      🤝 QUIC 连接建立: {}", connection.remote_address());
                        
                        // --- 步骤 H3-5: 创建 h3 连接处理程序 ---
                        // `h3::server::builder()`: 创建一个 HTTP/3 服务器连接的构建器。
                        // `.enable_webtransport(true)`, `.enable_connect(true)`, `.enable_datagram(true)`: 启用可选的 HTTP/3 特性。
                        // `.max_webtransport_sessions(10)`, `.send_grease(true)`: 配置一些 H3 参数。
                        // `.build(h3_quinn_compat::Connection::new(connection))`:
                        //   - `h3_quinn_compat::Connection::new(connection)`: 将 `quinn::Connection` 包装成 `h3` 库可以使用的传输层连接。
                        //   - `.build(...)`: 使用此传输连接构建 HTTP/3 服务器逻辑。
                        // `.await`: 构建过程是异步的。
                        let h3_conn_result = h3::server::builder()
                            .enable_webtransport(true)      // 启用 WebTransport (可选)
                            .enable_connect(true)           // 启用 CONNECT 方法 (可选)
                            .enable_datagram(true)          // 启用 HTTP Datagrams (可选)
                            .max_webtransport_sessions(10)  // 示例: 限制并发 WebTransport 会话数
                            .send_grease(true)              // 示例: 启用 QUIC GREASE
                            .build(h3_quinn_compat::Connection::new(connection)) // 将 Quinn 连接包装为 H3 连接
                            .await;

                        // 检查 H3 连接构建是否成功。
                        if let Err(e) = &h3_conn_result {
                            error!("      ❌ 建立 H3 连接失败: {:?}", e);
                            return; // 构建 H3 连接失败，此任务结束。
                        }
                        // `.unwrap()` 在这里是安全的，因为我们已经检查了错误。
                        let mut h3_conn = h3_conn_result.unwrap();
                        // `h3_conn.peer_settings()` 获取对端（客户端）的 HTTP/3 设置信息。
                        // `.map_or_else(...)` 用于处理 `Option`，如果设置存在则格式化，否则显示未知。
                        info!("      📡 H3 连接初始化成功 for peer, max field section size: {}.",
                              h3_conn.peer_settings().map_or_else(|| "<unknown>".to_string(), |s| s.max_field_section_size().to_string()));

                        // --- 步骤 H3-6: 循环接受此 H3 连接上的 HTTP/3 请求流 ---
                        // 一个 H3 连接可以承载多个并行的请求/响应流。
                        // `h3_conn.accept().await`: 异步等待此 H3 连接上的下一个传入请求。
                        loop { // 持续处理此 H3 连接上的请求
                            match h3_conn.accept().await {
                                // `Ok(Some((h3_req, h3_stream)))`: 成功接收到一个新的 HTTP/3 请求。
                                // - `h3_req`: `http::Request<()>` 类型，包含请求的元数据 (方法, URI, 头部)，但请求体是空的 `()`。
                                //   实际的请求体数据通过 `h3_stream` 单独接收。
                                // - `h3_stream`: `H3RequestStream<S, Bytes>` 类型，用于双向流数据 (接收请求体，发送响应体)。
                                Ok(Some((h3_req, h3_stream))) => {
                                    // 记录接收到的 H3 请求。`h3_conn.shared_state().peer_addr` 获取客户端地址。
                                    info!("         📥 接收到 H3 请求: {} {} from {}", h3_req.method(), h3_req.uri(), h3_conn.shared_state().peer_addr);

                                    // 为每个具体的 H3 请求再派生一个 Tokio 任务来处理。
                                    // 这样可以并发处理同一 H3 连接上的多个请求流。
                                    tokio::spawn({
                                        let app_clone_for_handler = app_clone.clone(); // 再次克隆 Router
                                        async move {
                                            // 调用 `handle_h3_request` 函数来处理这个请求。
                                            // 这个函数负责将 H3 请求适配给 Axum Router，并将 Axum 响应适配回 H3。
                                            if let Err(e) = handle_h3_request(h3_req, h3_stream, app_clone_for_handler).await {
                                                error!("         ❌ 处理 H3 请求失败: {:?}", e);
                                            }
                                        }
                                    });
                                }
                                // `Ok(None)`: 表示 H3 连接已正常关闭 (例如客户端关闭了连接)。
                                Ok(None) => {
                                    info!("      🚪 H3 连接正常关闭 by peer {}.", h3_conn.shared_state().peer_addr);
                                    break; // 跳出内部的请求接受循环，这个 H3 连接的处理结束。
                                }
                                // `Err(e)`: 在接受 H3 请求时发生错误。
                                Err(e) => {
                                    error!("      ❌ H3 连接错误 for peer {}: {:?}", h3_conn.shared_state().peer_addr, e);
                                    break; // 发生错误，跳出内部循环，结束此 H3 连接的处理。
                                }
                            }
                        }
                    }
                    // `Err(e)`: QUIC 连接建立失败。
                    Err(e) => {
                        error!("❌ 接受 QUIC 连接失败: {:?}", e);
                    }
                }
            }
        });
    }
    // 如果 `endpoint.accept()` 返回 `None` (通常意味着端点已关闭)，则主循环结束。
    info!("🛑 HTTP/3 服务器主循环结束 (可能由于端点关闭)");
}


// `async fn handle_h3_request<S_STREAM>(...)`: 定义异步函数 `handle_h3_request`。
// - `<S_STREAM>`: 这是一个泛型参数 (Generic Parameter)。表示此函数可以处理任何实现了特定条件的流类型 `S_STREAM`。
//   - 泛型使得代码更具通用性，可以复用于不同类型的 QUIC 流实现。
// - `h3_req: HttpRequest<()>`: HTTP/3 请求的元数据部分 (方法, URI, 头部)。类型是 `http::Request<()>`，表示请求体类型是空元组 `()`，
//   因为实际的请求体数据通过 `h3_stream` 单独处理。
// - `mut h3_stream: H3RequestStream<S_STREAM, Bytes>`: HTTP/3 请求/响应流。
//   - `mut`: 表示这个变量 `h3_stream` 在函数内部是可变的，因为我们需要从中读取请求体并向其写入响应。
//   - `H3RequestStream` 是 `h3::server::RequestStream` 的别名。
//   - `S_STREAM`: QUIC 底层双向流的类型。
//   - `Bytes`: 流中数据块的类型，来自 `bytes` crate，表示一块连续的内存。
// - `app: Router`: Axum 应用的 `Router` 实例，用于处理转换后的请求。
// - `-> Result<(), Box<dyn std::error::Error + Send + Sync>>`: 函数返回一个 `Result`。
//   - `()`: 成功时返回空元组，表示操作完成。
//   - `Box<dyn std::error::Error + Send + Sync>`: 失败时返回一个动态错误，原因同 `start_http3_server`。
// `where S_STREAM: h3::quic::RecvStream + Send + Unpin + 'static`: 对泛型参数 `S_STREAM` 的约束 (Trait Bounds)。
//   - `h3::quic::RecvStream`: `S_STREAM` 必须实现 `h3` 库定义的 `RecvStream` trait，表示它是一个可以接收数据的 QUIC 流。
//   - `Send`: `S_STREAM` 类型的值可以安全地在线程间发送。
//   - `Unpin`: `S_STREAM` 类型的值在内存中可以被移动后仍然保持固定 (对于某些异步操作是必需的)。
//   - `'static`: `S_STREAM` 类型本身不包含任何有生命周期限制的引用 (或者其包含的引用都是 `'static` 的)。
//     这在将异步任务派生到 Tokio 运行时中时很常见，因为任务可能比创建它的作用域活得更久。
async fn handle_h3_request<S_STREAM>(
    h3_req: HttpRequest<()>,
    mut h3_stream: H3RequestStream<S_STREAM, Bytes>,
    app: Router,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S_STREAM: h3::quic::RecvStream + Send + Unpin + 'static,
{
    // 日志记录：开始处理 H3 请求，包括方法和 URI。
    info!("         ⚙️  Handling H3 request: {} {}", h3_req.method(), h3_req.uri());

    // --- 步骤 1: 将 H3 请求转换为 Axum HTTP 请求 (http::Request<axum::body::Body>) ---

    // `h3_req.into_parts()`: 将 `http::Request<()>` 分解为其组成部分：`Parts` (包含头部, 方法, URI, 版本等) 和 `()` (空body)。
    // `let (h3_parts, _)`: 使用模式匹配解构元组。`_` 表示我们不关心这个空 body。
    let (h3_parts, _) = h3_req.into_parts();
    // `h3_parts` 现在是 `http::request::Parts` 类型。

    // `HttpRequest::builder()`: 使用 `http` crate 的 `Builder`模式来逐步构建一个新的 `http::Request`。
    // 这是创建请求的标准方式。
    let mut axum_request_builder = HttpRequest::builder()
        // `.method(h3_parts.method)`: 设置请求方法 (GET, POST, etc.)，从原始 H3 请求的 `Parts` 中获取。
        .method(h3_parts.method)
        // `.uri(h3_parts.uri)`: 设置请求 URI (路径和查询参数)，从原始 H3 请求的 `Parts` 中获取。
        .uri(h3_parts.uri)
        // `.version(HttpVersion::HTTP_2)`: 设置 HTTP 版本。
        // 尽管这是 HTTP/3 请求，但 Axum 内部通常将请求视为 HTTP/1.1 或 HTTP/2 的抽象。
        // H3 本身的版本信息在 `h3_parts.version` 中，但它可能不是 `http::Version` 枚举的标准成员。
        // 将其表示为 HTTP/2 是一个常见的做法，因为 Axum (基于 Hyper) 对 HTTP/2 有良好支持。
        .version(HttpVersion::HTTP_2); // 在 Axum 内部将其视为 HTTP/2 请求。

    // 遍历原始 H3 请求的所有头部。
    // `h3_parts.headers` 是一个 `http::HeaderMap`。
    // `.iter()` 返回一个迭代器，每次迭代产生一个 `(&HeaderName, &HeaderValue)` 元组。
    for (name, value) in h3_parts.headers.iter() {
        // `.header(name, value)`: 将每个头部添加到新的 Axum 请求构建器中。
        axum_request_builder = axum_request_builder.header(name, value);
    }

    // --- 处理 HTTP/3 请求体 ---
    // HTTP/3 的请求体数据通过 `h3_stream` (类型 `H3RequestStream`) 异步接收。
    // Axum 的 `Body` (即 `AxumBody`) 需要一个实现了 `Stream<Item = Result<Frame<Bytes>, Error>>` 的流。
    // 我们使用 `async_stream::stream!` 宏来创建一个这样的流。
    // `async_stream::stream!` 宏允许我们像写同步代码一样来定义一个异步流的产生逻辑。
    let h3_body_bytes_stream = async_stream::stream! {
        // `loop` 创建一个无限循环来持续接收数据块。
        loop {
            // `h3_stream.recv_data().await`: 异步等待从 H3 流中接收下一个数据块 (`Bytes`)。
            // - `.await` 表示这是一个异步操作，会暂停当前流的执行直到数据到达或发生错误。
            // - `recv_data()` 返回 `Result<Option<Bytes>, h3::Error>`。
            match h3_stream.recv_data().await {
                // `Ok(Some(bytes))`: 成功接收到一个数据块 `bytes`。
                Ok(Some(bytes)) => {
                    // `yield Ok(Frame::data(bytes))`: `yield` 是 `async_stream` 提供的关键字，用于从流中产生一个值。
                    // 我们将 `Bytes` 包装在 `http_body_util::Frame::data()` 中，因为 Axum Body 的流期望 `Frame` 类型。
                    // `Ok(...)` 表示这个数据帧是成功的。
                    yield Ok(Frame::data(bytes));
                }
                // `Ok(None)`: 表示 H3 请求体流已正常结束，没有更多数据了。
                Ok(None) => {
                    info!("         ➡️ H3 request body stream ended.");
                    break; // 跳出循环，结束这个流。
                }
                // `Err(e)`: 从 H3 流接收数据时发生错误。
                Err(e) => {
                    error!("         ❌ Error receiving H3 request body chunk: {:?}", e);
                    // `yield Err(...)`: 在流中产生一个错误。
                    // 这个错误需要是 `StreamBody` 所期望的错误类型，即实现了 `Into<Box<dyn std::error::Error + Send + Sync + 'static>>`。
                    // `h3::Error` 可能不直接满足此要求，所以我们将其包装在 `Box::new(e)` 中。
                    let boxed_err = Box::new(e) as Box<dyn std::error::Error + Send + Sync + 'static>;
                    yield Err(boxed_err);
                    break; // 发生错误，也跳出循环，结束流。
                }
            }
        }
    };

    // `AxumBody::from_stream(h3_body_bytes_stream)`: 将上面创建的异步流 `h3_body_bytes_stream` 转换为 Axum 可以使用的 `Body` 类型。
    let axum_req_body = AxumBody::from_stream(h3_body_bytes_stream);

    // `axum_request_builder.body(axum_req_body)`: 将构造好的请求体设置到请求构建器中，并最终构建出 `http::Request<AxumBody>`。
    // `?` 操作符: 如果 `.body()` 方法返回错误 (例如，头部构造有问题，虽然在这里不太可能)，则错误会从 `handle_h3_request` 函数提前返回。
    // 错误会被转换为 `Box<dyn std::error::Error + Send + Sync>`。
    let axum_request = axum_request_builder.body(axum_req_body)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    // `axum_request` 现在是一个完整的、Axum 可以处理的 HTTP 请求。

    // 日志记录：即将调用 Axum Router。
    info!("         🚀 Calling Axum Router for: {} {}", axum_request.method(), axum_request.uri());
    // --- 步骤 2: 调用 Axum Router 处理请求 ---
    // `app.oneshot(axum_request).await`: 将转换后的 `axum_request` 发送给 Axum 的 `Router` (`app`) 进行处理。
    // - `app` 是之前从 `main` 函数传递过来的 `Router` 实例。
    // - `.oneshot()` 是 `tower::ServiceExt` trait 提供的方法 (Axum Router 实现了 `Service`)，
    //   用于发送单个请求并获取单个响应，非常适合这种桥接场景。
    // - `.await`: 因为路由处理本身是异步的。
    // - `oneshot` 返回 `Result<http::Response<AxumBody>, Infallible>`。`Infallible` 表示理论上这个调用不应该失败 (除非服务 panic)。
    //   我们使用 `.map_err` 将这个不太可能发生的 `Infallible` 错误转换成我们函数签名期望的动态错误类型，以防万一。
    let axum_response = app.oneshot(axum_request).await
        .map_err(|e: Infallible| -> Box<dyn std::error::Error + Send + Sync> {
            error!("Axum oneshot call resulted in Infallible error: {:?}", e); // 记录这个意外情况
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Axum oneshot infallible error"))
        })?;
    // `axum_response` 现在是 `http::Response<AxumBody>` 类型，包含了 Axum 处理器生成的响应。

    // 日志记录：Axum Router 已返回响应。
    info!("         ↪️ Axum Router responded with status: {}", axum_response.status());

    // --- 步骤 3: 将 Axum HTTP 响应转换回 HTTP/3 响应 ---
    // `axum_response.into_parts()`: 将 `http::Response<AxumBody>` 分解为 `Parts` (包含状态码, 版本, 头部) 和 `AxumBody` (响应体)。
    let (axum_parts, mut axum_body) = axum_response.into_parts();
    // `mut axum_body`: 响应体需要是可变的，因为我们要从中读取数据。

    // `HttpResponse::builder()`: 开始构建一个用于 H3 的 `http::Response`。
    let mut h3_response_builder = HttpResponse::builder()
        // `.status(axum_parts.status)`: 设置响应状态码，从 Axum 响应的 `Parts` 中获取。
        .status(axum_parts.status)
        // `.version(HttpVersion::HTTP_3)`: 明确设置版本为 HTTP/3。虽然对于 H3 协议本身，这个字段可能不那么重要，
        // 但在 `http` 类型中设置它可以保持一致性。
        .version(HttpVersion::HTTP_3);

    // 遍历 Axum 响应的所有头部。
    for (name, value) in axum_parts.headers.iter() {
        // 将每个头部添加到 H3 响应构建器中。
        h3_response_builder = h3_response_builder.header(name, value);
    }
    // `.body(())`: 构建 H3 响应的头部部分。对于 `h3::server::RequestStream::send_response`，
    // 它期望一个 `http::Response<()>` 类型，即 body 部分是空元组 `()`，因为实际的 body 数据会通过 `send_data` 单独发送。
    let h3_response_head = h3_response_builder.body(())
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    // `h3_response_head` 现在是 `http::Response<()>`。

    // `h3_stream.send_response(h3_response_head).await`: 异步发送构造好的响应头部给客户端。
    // 这个操作也可能失败 (例如连接已断开)。
    match h3_stream.send_response(h3_response_head).await {
        Ok(_) => info!("         📤 Sent H3 response headers."), // 成功发送头部
        Err(e) => {
            error!("         ❌ Failed to send H3 response headers: {:?}", e);
            return Err(Box::new(e)); // 发送头部失败，返回错误。
        }
    }

    // --- 流式传输 Axum 响应体数据 ---
    // `while let Some(chunk_result) = axum_body.data().await`: 循环异步读取 Axum 响应体的数据块。
    // - `axum_body.data()`: (来自 `BodyExt` trait) 返回一个 `Option<Result<Bytes, Error>>`。
    //   - `Some(Ok(chunk))`: 成功读取到一个数据块 `chunk` (类型 `Bytes`)。
    //   - `Some(Err(e))`: 读取数据块时发生错误。
    //   - `None`: 响应体数据已全部读取完毕。
    while let Some(chunk_result) = axum_body.data().await {
        match chunk_result {
            Ok(chunk) => {
                // `!chunk.is_empty()`: 只有在数据块非空时才发送。空数据块通常不需要发送。
                if !chunk.is_empty() {
                    info!("         ➡️ Sending H3 response data chunk ({} bytes)", chunk.len());
                    // `h3_stream.send_data(chunk).await`: 异步发送数据块给客户端。
                    // 这个操作也可能失败。
                    if let Err(e) = h3_stream.send_data(chunk).await {
                        error!("         ❌ Failed to send H3 data chunk: {:?}", e);
                        // 如果发送数据块失败，尝试结束流 (可能也会失败，但尽力而为)，然后返回错误。
                        let _ = h3_stream.finish().await; // 尽力尝试结束流
                        return Err(Box::new(e));
                    }
                }
            }
            Err(e) => { // Axum 响应体流本身发生错误
                error!("         ❌ Error receiving Axum response body chunk: {:?}", e);
                let _ = h3_stream.finish().await; // 尽力尝试结束流
                // 将 Axum body 错误转换为我们函数签名期望的动态错误类型。
                return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
            }
        }
    }

    // `h3_stream.finish().await`: 所有数据发送完毕后，正常结束 HTTP/3 响应流。
    // 这会通知客户端响应已完整发送。
    match h3_stream.finish().await {
        Ok(_) => info!("         ✅ H3 stream finished successfully."), // 成功结束
        Err(e) => {
            error!("         ❌ Failed to finish H3 stream: {:?}", e);
            return Err(Box::new(e)); // 结束流失败，返回错误。
        }
    }
    
    // 所有操作成功完成。
    Ok(())
}


/// 生成自签名 TLS 证书 (用于开发环境)
///
/// 【目的】: 使用 `rcgen` 库动态生成一个临时的、自签名的 TLS 证书和私钥。
/// 【用途】: 主要用于本地开发和测试 HTTPS/HTTP3，避免手动创建证书的麻烦。
// `fn generate_self_signed_cert() -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error + Send + Sync>>`:
// 定义一个名为 `generate_self_signed_cert` 的函数。
// - 它不接收参数。
// - 返回类型是 `Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error + Send + Sync>>`
//   - `Ok((Vec<u8>, Vec<u8>))`: 成功时返回一个元组，包含两个 `Vec<u8>` (字节向量)。
//     第一个是 PEM 格式的证书，第二个是 PEM 格式的私钥。
//   - `Err(Box<dyn std::error::Error + Send + Sync>)`: 失败时返回一个动态错误。
//     `Send` 和 `Sync` trait bounds 确保错误可以安全地在线程间传递，这在异步和多线程代码中很重要。
/// 生成自签名 TLS 证书 (用于开发环境)
///
/// 【目的】: 使用 `rcgen` 库动态生成一个临时的、自签名的 TLS 证书和私钥。
/// 【用途】: 主要用于本地开发和测试 HTTPS/HTTP3，避免手动创建证书的麻烦。
/// 【安全警告】: 自签名证书不被浏览器信任，会触发安全警告。**绝不能用于生产环境！**
///
/// # 返回值
/// * `Ok((Vec<u8>, Vec<u8>))` - 成功时返回 (PEM 格式的证书, PEM 格式的私钥)。
/// * `Err(Box<dyn std::error::Error + Send + Sync>)` - 生成过程中发生任何错误.
fn generate_self_signed_cert() -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error + Send + Sync>> {
    // 日志记录：开始生成证书。
    info!("      🔑 Generating self-signed TLS certificate (rcgen)...");

    // --- 步骤 Cert-1: 定义证书参数 ---
    // `subject_alt_names` (SANs): 主体备用名称。指定证书适用的域名或 IP 地址。
    // 对于本地开发，通常使用 "localhost"。如果需要通过 IP 访问，也应添加 IP 地址。
    let subject_alt_names = vec!["localhost".to_string()]; // 证书将对 "localhost" 有效

    // `rcgen::CertificateParams::new(subject_alt_names)`: 创建证书参数对象。
    // `rcgen` 允许详细配置证书的各个方面。
    let mut params = rcgen::CertificateParams::new(subject_alt_names);
    // `params.alg = &rcgen::PKCS_ECDSA_P256_SHA256;`: 设置证书的签名算法。
    // 这里选择 ECDSA P-256 with SHA-256，这是一种常见且安全的椭圆曲线加密算法。
    params.alg = &rcgen::PKCS_ECDSA_P256_SHA256;

    // --- 步骤 Cert-2: 生成证书对象 ---
    // `rcgen::Certificate::from_params(params)`: 根据配置的参数生成证书。
    // `?` 操作符: 如果 `from_params` 失败并返回 `Err`，`?` 会立即将这个错误从 `generate_self_signed_cert` 函数返回。
    //   这是一种简洁的错误传播方式。
    let cert = rcgen::Certificate::from_params(params)?;
    // `cert` 现在是一个 `rcgen::Certificate` 对象。

    // --- 步骤 Cert-3: 导出 PEM 格式 ---
    // `.serialize_pem()`: 将证书对象序列化为 PEM (Privacy-Enhanced Mail) 文本格式。
    // PEM 是存储和交换加密密钥、证书等数据的常用格式。它使用 Base64 编码，并有特定的页眉和页脚。
    // `?`: 同样用于错误传播。
    let cert_pem = cert.serialize_pem()?; // 证书的 PEM 字符串
    // `.serialize_private_key_pem()`: 将证书关联的私钥序列化为 PEM 文本格式。
    let key_pem = cert.serialize_private_key_pem(); // 私钥的 PEM 字符串

    // 日志记录：证书生成成功。
    info!("      ✓ Self-signed certificate generated.");

    // --- 步骤 Cert-4: 返回结果 ---
    // `Ok((cert_pem.into_bytes(), key_pem.into_bytes()))`:
    //   - `cert_pem.into_bytes()`: 将 PEM 格式的 `String` 转换为 `Vec<u8>` (字节向量)。
    //   - `key_pem.into_bytes()`: 同上。
    //   - 将这两个字节向量包装在一个元组中，并通过 `Ok(...)` 返回，表示操作成功。
    Ok((cert_pem.into_bytes(), key_pem.into_bytes()))
}


// `fn configure_quinn_server(...) -> Result<QuinnServerConfig, Box<dyn std::error::Error + Send + Sync>>`:
// 定义一个名为 `configure_quinn_server` 的函数。
// - `cert_pem: Vec<u8>`: 第一个参数，PEM 格式的证书链字节。
// - `key_pem: Vec<u8>`: 第二个参数，PEM 格式的私钥字节。
// - 返回类型是 `Result<QuinnServerConfig, Box<dyn std::error::Error + Send + Sync>>`
//   - `Ok(QuinnServerConfig)`: 成功时返回配置好的 `quinn::ServerConfig` 对象。
//   - `Err(...)`: 失败时返回动态错误。
/// 配置 Quinn 服务器 (QUIC 层)
///
/// 【目的】: 使用提供的 TLS 证书和私钥来配置 Quinn QUIC 服务器。
/// 【关键配置】:
///   - 加载证书链和私钥。
///   - 设置 ALPN (Application-Layer Protocol Negotiation) 协议为 "h3"，
///     这是 QUIC 连接协商使用 HTTP/3 的标准方式。
///
/// # 参数
/// * `cert_pem`: PEM 格式的 TLS 证书链 (通常只有一个证书)。
/// * `key_pem`: PEM 格式的私钥。
///
/// # 返回值
/// * `Ok(QuinnServerConfig)` - 配置成功的 Quinn 服务器配置对象。
/// * `Err(Box<dyn std::error::Error + Send + Sync>)` - 配置过程中发生任何错误.
fn configure_quinn_server(
    cert_pem: Vec<u8>,
    key_pem: Vec<u8>,
) -> Result<QuinnServerConfig, Box<dyn std::error::Error + Send + Sync>> {
    // 日志记录：开始配置 Quinn 服务器。
    info!("      ⚙️  Configuring Quinn (QUIC) server...");

    // --- 步骤 QuinnCfg-1: 解析 PEM 证书 ---
    // `rustls_pemfile::certs(&mut cert_pem.as_slice())`: 从 PEM 编码的字节切片中解析证书。
    //   - `cert_pem.as_slice()`: 将 `Vec<u8>` 转换为 `&[u8]` (字节切片)。`&mut` 表示可变借用，因为解析器可能会修改切片位置。
    //   - `certs()` 返回一个迭代器，每个元素是 `Result<CertificateDer, _>`。
    //   - `.collect::<Result<Vec<_>, _>>()?`: 将迭代器中的所有 `Result` 收集到一个 `Vec<CertificateDer>` 中。
    //     如果任何一个元素是 `Err`，则整个 `collect` 操作会失败，并通过 `?` 将错误传播出去。
    //   - `.into_iter().map(rustls::pki_types::CertificateDer::into_owned).collect()`:
    //     这一步确保 `CertificateDer` 具有 `'static` 生命周期，方法是将其转换为拥有所有权的版本。
    //     `rustls_pemfile` 解析出的 `CertificateDer` 可能借用于输入数据，而 `rustls::ServerConfig` 需要 `'static` 生命周期。
    let certs_der: Vec<rustls::pki_types::CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_pem.as_slice())
        .collect::<Result<Vec<_>, _>>()? // 如果解析任何证书失败，则返回错误
        .into_iter()
        .map(rustls::pki_types::CertificateDer::into_owned) // 转换为拥有所有权的 CertificateDer<'static>
        .collect();

    // 检查是否至少解析出一个证书。
    if certs_der.is_empty() {
        // 如果没有找到证书，返回一个自定义错误。
        // `"".into()` 将字符串字面量转换为 `Box<dyn Error...>`。
        return Err("No certificates found in PEM file.".into());
    }
    info!("         - Certificate chain loaded ({} certs).", certs_der.len());

    // --- 步骤 QuinnCfg-2: 解析 PEM 私钥 ---
    // `rustls_pemfile::private_key(&mut key_pem.as_slice())?`: 从 PEM 编码的字节切片中解析私钥。
    //   - 返回 `Result<Option<PrivateKeyDerEnum>, _>`。`Option` 是因为文件中可能没有私钥。
    //   - `?` 处理 `rustls_pemfile` 可能返回的 I/O 错误。
    // `.ok_or_else(|| "No private key found in PEM file".to_string())?`:
    //   - 如果 `private_key` 返回 `Ok(None)` (即文件中没有找到私钥)，则将其转换成一个错误。
    //   - `.to_string()` 创建一个 `String`，然后 `.into()` (隐式) 或显式 `Box::new(...)` 将其转换为动态错误。
    //   - `?` 再次用于错误传播。
    let key_der_unowned = rustls_pemfile::private_key(&mut key_pem.as_slice())?
        .ok_or_else(|| "No private key found in PEM file".to_string())?;

    // 将解析出的 `PrivateKeyDer` (可能是借用的) 转换为拥有所有权的 `'static` 版本。
    // `rustls::ServerConfig` 需要 `'static` 生命周期的私钥。
    // `match` 表达式处理不同类型的私钥格式 (PKCS1, PKCS8, SEC1)。
    let key_der_owned = match key_der_unowned {
        rustls::pki_types::PrivateKeyDer::Pkcs1(key) => rustls::pki_types::PrivateKeyDer::Pkcs1(key.into_owned()),
        rustls::pki_types::PrivateKeyDer::Pkcs8(key) => rustls::pki_types::PrivateKeyDer::Pkcs8(key.into_owned()),
        rustls::pki_types::PrivateKeyDer::Sec1(key) => rustls::pki_types::PrivateKeyDer::Sec1(key.into_owned()),
        // `_` 是一个通配符，匹配任何其他未明确列出的私钥格式。
        // 如果遇到不支持的格式，返回错误。
        _ => return Err("Unknown or unsupported private key format".into()),
    };

    // `rustls::crypto::ring::sign::any_supported_type(&key_der_owned)`:
    //   - 尝试将解析出的私钥 (DER 格式) 转换为 `rustls` 内部可以使用的签名密钥对象。
    //   - `ring`是 `rustls` 使用的加密库之一。
    //   - `.map_err(|_e| ...)`: 如果转换失败 (例如密钥类型不受支持或格式错误)，则映射为一个自定义错误。
    //     `_e` 忽略原始错误细节，只返回我们的消息。
    let key = rustls::crypto::ring::sign::any_supported_type(&key_der_owned)
        .map_err(|_e| "Unsupported private key type or malformed key".into())?;
    info!("         - Private key loaded and parsed.");

    // --- 步骤 QuinnCfg-3: 创建 rustls 服务器配置 (`rustls::ServerConfig`) ---
    // `rustls::ServerConfig::builder()`: 获取一个 `ServerConfigBuilder` 用于链式配置。
    let mut server_crypto = rustls::ServerConfig::builder()
        // `.with_no_client_auth()`: 配置服务器不要求客户端进行证书认证。
        // 对于公共 Web 服务器，这通常是标准做法。
        .with_no_client_auth()
        // `.with_single_cert(certs_der, key)?`: 设置服务器的 TLS 证书链和对应的私钥。
        // `certs_der` 是之前解析的证书列表，`key` 是解析的私钥。
        // `?` 用于错误传播。
        .with_single_cert(certs_der, key)?;

    // --- 步骤 QuinnCfg-4: 设置 ALPN 协议 ---
    // **ALPN (Application-Layer Protocol Negotiation)**: TLS 的一个扩展，允许客户端和服务器在 TLS 握手期间协商应用层协议。
    // 对于 HTTP/3，客户端和服务器必须都同意使用 "h3" 协议。
    // `server_crypto.alpn_protocols = vec![b"h3".to_vec()];`:
    //   - `alpn_protocols` 字段设置服务器支持的 ALPN 协议列表。
    //   - `b"h3"` 是 HTTP/3 的标准 ALPN 标识符 (字节字符串)。
    //   - `.to_vec()` 将其转换为 `Vec<u8>`。
    // 这是启用 HTTP/3 的关键步骤。
    server_crypto.alpn_protocols = vec![b"h3".to_vec()];
    info!("         - ALPN protocol set to 'h3'.");

    // --- 步骤 QuinnCfg-5: 创建 Quinn 服务器配置 (`QuinnServerConfig`) ---
    // `QuinnServerConfig::with_crypto(std::sync::Arc::new(server_crypto))`:
    //   - 将 `rustls::ServerConfig` (即 `server_crypto`) 包装到 `QuinnServerConfig` 中。
    //   - `std::sync::Arc::new(...)`: 将 `server_crypto` 包装在 `Arc` (原子引用计数指针) 中。
    //     这是因为 Quinn 服务器配置可能会在多个 QUIC 连接或任务之间共享，`Arc` 允许多所有者安全共享。
    let mut server_config = QuinnServerConfig::with_crypto(std::sync::Arc::new(server_crypto));

    // 可选：配置 Quinn 传输参数。
    // `std::sync::Arc::make_mut(&mut server_config.transport)`: 获取对 `transport` 配置的可变引用。
    // 如果 `Arc` 有多个强引用，`make_mut` 会克隆内部数据以确保唯一可变访问。
    let transport_config = std::sync::Arc::make_mut(&mut server_config.transport);
    // `transport_config.max_idle_timeout(...)`: 设置 QUIC 连接的最大空闲超时时间。
    // `Some(...)` 表示设置一个值。`std::time::Duration::from_secs(60)` 创建一个60秒的持续时间。
    // `.try_into().unwrap()`: 将 `std::time::Duration` 转换为 Quinn 所需的内部超时类型。
    //   `unwrap()` 在这里假设转换总是成功的 (对于合理的 Duration 值)。
    transport_config.max_idle_timeout(Some(std::time::Duration::from_secs(60).try_into().unwrap()));
    info!("         - QUIC transport parameters configured (e.g., max_idle_timeout).");

    // 日志记录：Quinn 服务器配置成功构建。
    info!("      ✓ Quinn server configuration built successfully.");
    // 返回配置好的 `QuinnServerConfig`。
    Ok(server_config)
}

/*
// Commenting out the old version of configure_quinn_server
// to ensure the new one is used. The old content was:
fn configure_quinn_server(
    cert_pem: Vec<u8>,
    key_pem: Vec<u8>,
) -> Result<quinn::ServerConfig, Box<dyn std::error::Error>> {
    info!("      ⚙️  正在配置 Quinn (QUIC) 服务器...");
    // --- 步骤 QuinnCfg-1: 解析 PEM 证书 ---
    // `rustls_pemfile::certs` 从 PEM 文本中解析出 DER 编码的证书。
    // 对于本地测试，通常使用 "localhost" 或 "127.0.0.1"。
    let subject_alt_names = vec!["localhost".to_string()];
    let mut params = rcgen::CertificateParams::new(subject_alt_names);
    
    // --- 步骤 Cert-2: 选择签名算法 --- 
    // 这里使用 ECDSA P-256 with SHA-256，是一种常见的现代算法。
    params.alg = &rcgen::PKCS_ECDSA_P256_SHA256;
    
    // --- 步骤 Cert-3: 生成证书对象 --- 
    // `Certificate::from_params` 根据参数生成证书。
    // `?` 操作符在出错时提前返回 Err。
    let cert = rcgen::Certificate::from_params(params)?;
    
    // --- 步骤 Cert-4: 导出 PEM 格式 --- 
    // `.serialize_pem()` 导出证书为 PEM 文本格式。
    // `.serialize_private_key_pem()` 导出私钥为 PEM 文本格式。
    let cert_pem = cert.serialize_pem()?;
    let key_pem = cert.serialize_private_key_pem();
    
    info!("      ✓ 自签名证书生成成功。");
    // --- 步骤 Cert-5: 返回结果 --- 
    Ok((cert_pem.into_bytes(), key_pem.into_bytes()))
}

/// 配置 Quinn 服务器 (QUIC 层)
/// 
/// 【目的】: 使用提供的 TLS 证书和私钥来配置 Quinn QUIC 服务器。
    // `.pop().unwrap()` 取出第一个证书 (假设只有一个)。
    // `CertificateDer::from(...)` 创建 `rustls` 库使用的证书类型。
    /*
    let cert_chain = vec![rustls::pki_types::CertificateDer::from(
        rustls_pemfile::certs(&mut &cert_pem[..])?.remove(0)
    )];
    info!("         - 证书链加载成功。");

    // --- 步骤 QuinnCfg-2: 解析 PEM 私钥 --- 
    // `rustls_pemfile::private_key` (或 `pkcs8_private_keys`) 解析私钥。
    // `PrivateKeyDer::from(...)` 创建 `rustls` 使用的私钥类型。
    let key_der = rustls::pki_types::PrivateKeyDer::try_from(
        rustls_pemfile::private_key(&mut &key_pem[..])?.unwrap()
    )?;
    let key = rustls::crypto::ring::sign::any_supported_type(&key_der)?;
    info!("         - 私钥加载并解析成功。");

    // --- 步骤 QuinnCfg-3: 创建 rustls 服务器配置 --- 
    // `rustls::ServerConfig::builder()` 开始构建 TLS 配置。
    // `.with_no_client_auth()` 表示服务器不要求客户端提供证书。
    // `.with_single_cert(cert_chain, key)?` 设置服务器的证书链和私钥。
    let mut server_crypto = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)?;

    // --- 步骤 QuinnCfg-4: 设置 ALPN 协议 --- 
    // 关键步骤：告诉客户端此 QUIC 连接支持 HTTP/3 协议。
    // `b"h3".to_vec()` 是 HTTP/3 的标准 ALPN 标识符。
    server_crypto.alpn_protocols = vec![b"h3".to_vec()];
    info!("         - ALPN 协议设置为 'h3'。");

    // --- 步骤 QuinnCfg-5: 创建 Quinn 服务器配置 --- 
    // `quinn::ServerConfig::with_crypto` 将 `rustls` 的 TLS 配置包装成 Quinn 配置。
    // `std::sync::Arc::new` 用于在可能的多线程环境中安全共享配置。
    let mut server_config = quinn::ServerConfig::with_crypto(std::sync::Arc::new(server_crypto));
    
    // 可选：配置 Quinn 传输参数 (例如最大空闲超时)
    let transport_config = std::sync::Arc::make_mut(&mut server_config.transport);
    transport_config.max_idle_timeout(Some(std::time::Duration::from_secs(60).try_into()?));
    info!("         - QUIC 传输参数配置完成 (e.g., max_idle_timeout)。");

    info!("      ✓ Quinn 服务器配置构建成功。");
    Ok(server_config)
    */
}
