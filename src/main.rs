// main.rs
//
// /-----------------------------------------------------------------------------\
// |                          【应用入口模块】 (main.rs)                         |
// |-----------------------------------------------------------------------------|
// |                                                                             |
// | 1. **设置异步运行时**: 使用 `#[tokio::main]` 宏准备异步环境。             |
// |                                                                             |
// | 2. **声明根模块**: 使用 `mod` 关键字引入项目的主要构建块：                |
// |    - `app`: 包含核心业务逻辑 (Controller, Service, Model)。              |
// |    - `config`: 管理应用程序配置。                                        |
// |    - `db`: 处理数据存储与访问。                                          |
// |    - `error`: 定义统一的错误处理机制。                                   |
// |    - `routes`: 集中定义所有 API 路由。                                   |
// |    - `startup`: 负责应用的初始化流程。                                   |
// |                                                                             |
// | 3. **执行 `main` 函数**:                                                    |
// |    a. 加载配置 (`config::AppConfig::from_env()`)。                         |
// |    b. 初始化应用 (`startup::init_app()`), 包括日志、数据库、路由等。       |
// |    c. 创建 TCP 监听器 (`TcpListener::bind()`)。                            |
// |    d. 启动 Axum HTTP/1.1 服务器 (`axum::serve()`), 开始接受请求。          |
// |                                                                             |
// | 4. **(可选) HTTP/3 支持**: 包含已注释的代码块，用于启动 HTTP/3 服务器。    |
// |    - 需要 TLS 证书。                                                      |
// |    - 使用 Quinn (QUIC) 和 h3 (HTTP/3) 库。                                |
// |    - 包含证书生成 (`generate_self_signed_cert`) 和服务器配置辅助函数。   |
// |                                                                             |
// \-----------------------------------------------------------------------------/
//
// 【核心职责】: 作为程序的起点，协调配置加载、应用初始化和服务器启动。
// 【关键技术】: `tokio` (异步运行时), `axum` (Web 框架), 模块系统 (`mod`), 配置管理。

use tokio::net::TcpListener; // Tokio 提供的异步 TCP Listener
use tracing::info; // 用于记录信息的日志宏
use anyhow::Result; // 引入 anyhow::Result 用于更简洁的错误处理

// --- 声明项目根模块 ---
// `mod` 关键字告诉 Rust 编译器查找并包含这些模块文件或目录。
// 这是 Rust 模块系统的基础，用于组织代码。
mod app; // 包含控制器、服务、模型等核心应用逻辑 (./app/mod.rs 或 ./app.rs)
mod config; // 应用配置加载与管理 (./config.rs)
// `mod db;` 已被移除，因为数据访问逻辑现在由 `app/repository` 和 `startup.rs` 处理。
mod error; // 自定义错误处理 (./error.rs)
mod routes; // API 路由定义 (./routes.rs)
mod startup; // 应用启动与初始化逻辑 (./startup.rs)

// --- 主函数 (程序入口) ---

// `#[tokio::main]` [[关键语法要素: 属性宏]]
// 这是一个由 `tokio` crate 提供的【过程宏】。
// 它将 `async fn main()` 函数转换为一个【同步】的 `fn main()`，
// 并在内部设置并启动 Tokio 异步运行时。
// 这使得我们可以在 `main` 函数内部直接使用 `.await` 语法来等待异步操作完成。
// 本质上是简化异步程序入口设置的语法糖。
#[tokio::main]
async fn main() -> Result<()> {
    // --- 步骤 1: 加载应用程序配置 ---
    // 从环境变量或配置文件加载配置信息 (例如服务器地址、数据库连接信息等)。
    // `config::AppConfig::from_env()` 是我们自定义的配置加载函数。
    let config = config::AppConfig::from_env();

    // --- 步骤 2: 初始化应用程序 ---
    // 调用 `startup` 模块的 `init_app` 函数执行应用的初始化序列。
    // 这通常包括: 设置日志系统 (tracing)、初始化数据库连接或内存存储、
    // 创建并配置 Axum Router (定义路由和中间件)。
    // 返回配置好的 Axum 应用实例 (`axum::Router`)。
    let app = startup::init_app(config.clone()).await?;

    // --- 步骤 3: 创建 TCP 监听器 ---
    // 使用从配置中获取的 HTTP 地址 (`config.http_addr`)。
    // `TcpListener::bind` 创建一个监听指定地址和端口的 TCP 套接字。
    let http_addr = config.http_addr;
    let listener = TcpListener::bind(http_addr).await?;
    info!("HTTP/1.1 服务器启动，监听地址: http://{}", http_addr);

    // --- 步骤 4: (可选) 启动 HTTP/3 服务器 ---
    // 这部分代码默认被注释掉，因为 HTTP/3 需要额外的设置 (TLS 证书)。
    // 如果需要启用，需要设置环境变量 `ENABLE_HTTP3=true` 并确保证书可用。
    /*
    if std::env::var("ENABLE_HTTP3").is_ok() {
        // `tokio::spawn` 在 Tokio 运行时中创建一个新的【异步任务】(类似线程)。
        // 这允许 HTTP/3 服务器与 HTTP/1.1 服务器【并发】运行。
        tokio::spawn(start_http3_server(config.http3_addr, app.clone()));
    }
    */

    // --- 步骤 5: 启动 HTTP/1.1 服务器 ---
    // `axum::serve` 是 Axum 提供的函数，用于将 TCP 监听器 (`listener`) 和
    // 配置好的 Axum 应用 (`app`) 绑定起来，并开始处理传入的 HTTP/1.1 请求。
    axum::serve(listener, app.into_make_service()).await?;

    // --- 步骤 6: 返回成功 ---
    // 如果服务器正常关闭（例如通过 Ctrl+C），main 函数会执行到这里并返回 Ok。
    Ok(())
}

// --- (可选) HTTP/3 服务器实现 ---
// 以下是启动 HTTP/3 服务器的辅助函数，默认被注释掉。

/*
use std::net::SocketAddr;
use quinn::{ Endpoint, ServerConfig }; // QUIC 协议实现
use h3::server::RequestStream;        // HTTP/3 协议实现
use h3_quinn::quinn;                  // h3 对 quinn 的集成
use http::Request;
use bytes::Bytes;
use axum::Router;

/// 启动 HTTP/3 服务器 (需要 TLS 证书)
/// 
/// 【目的】: 在指定的 `SocketAddr` 上启动一个基于 QUIC 的 HTTP/3 服务器。
/// 【注意】: 
///   - 这是可选功能，默认不启用。
///   - 需要有效的 TLS 证书 (开发时可使用自签名证书)。
///   - 这是一个简化的示例，生产环境需要更复杂的配置。
///   - 将 Axum 应用 (`Router`) 集成进来以处理请求。
/// 
/// # 参数
/// * `addr`: HTTP/3 服务器绑定的 `SocketAddr`。
/// * `app`: 配置好的 Axum `Router`，用于处理请求。
async fn start_http3_server(addr: SocketAddr, app: Router) {
    info!("⚠️  尝试启动 HTTP/3 服务器 (需要 TLS 证书): https://{}", addr);
    info!("   这是一个基础示例，生产环境需要更完善的证书管理和错误处理。");

    // --- 步骤 H3-1: 生成或加载 TLS 证书 --- 
    // 在实际应用中，你可能从文件加载证书或使用证书管理服务 (如 Let's Encrypt)。
    // 这里使用辅助函数生成自签名证书，仅用于本地开发和测试。
    let (cert, key) = match generate_self_signed_cert() {
        Ok((cert, key)) => (cert, key),
        Err(e) => {
            tracing::error!("❌ 生成自签名证书失败: {}. HTTP/3 服务器无法启动。", e);
            return;
        }
    };
    info!("   ✓ 已生成自签名 TLS 证书 (用于 HTTP/3)");

    // --- 步骤 H3-2: 配置 Quinn 服务器 (QUIC 层) --- 
    // 使用生成的证书和私钥配置 Quinn QUIC 服务器。
    let server_config = match configure_quinn_server(cert, key) {
        Ok(config) => config,
        Err(e) => {
            tracing::error!("❌ 配置 Quinn (QUIC) 服务器失败: {}. HTTP/3 服务器无法启动。", e);
            return;
        }
    };
    info!("   ✓ Quinn (QUIC) 服务器配置完成。");

    // --- 步骤 H3-3: 创建 Quinn 端点 --- 
    // `quinn::Endpoint` 代表一个 QUIC 端点，可以用来接受连接。
    // `.unwrap()` 在端点创建失败时会 panic。
    let endpoint = match quinn::Endpoint::server(server_config, addr) {
         Ok(ep) => ep,
         Err(e) => {
             tracing::error!("❌ 创建 Quinn (QUIC) 端点失败: {}. HTTP/3 服务器无法启动。 (地址可能已被占用)", e);
             return;
         }
    };
    info!("   ✓ Quinn (QUIC) 端点创建成功，监听地址: {}", addr);
    info!("⚠️  浏览器可能会因自签名证书而显示安全警告。");

    // --- 步骤 H3-4: 循环接受 QUIC 连接并处理 HTTP/3 请求 --- 
    // `endpoint.accept().await` 异步等待新的 QUIC 连接。
    while let Some(connecting) = endpoint.accept().await {
        info!("   🔌 接收到新的 QUIC 连接 from {}", connecting.remote_address());
        // 对每个新连接，创建一个新的 Tokio 任务来处理它，避免阻塞主循环。
        tokio::spawn({
            let app = app.clone(); // 克隆 Axum Router 以在任务中使用
            async move {
                match connecting.await {
                    Ok(connection) => {
                        info!("      🤝 QUIC 连接建立: {}", connection.remote_address());
                        // --- 步骤 H3-5: 创建 h3 连接处理程序 --- 
                        // `h3_quinn::Connection::new(connection)` 将 QUIC 连接包装成 h3 连接。
                        // `h3::server::builder()` 创建 HTTP/3 服务器逻辑。
                        let mut h3_conn = h3::server::builder()
                            .enable_webtransport(true) // 可选: 启用 WebTransport
                            .enable_connect(true)      // 可选: 启用 CONNECT 方法
                            .enable_datagram(true)     // 可选: 启用 HTTP Datagrams
                            .max_concurrent_streams(100) // 可选: 配置参数
                            .build(h3_quinn::Connection::new(connection))
                            .await;
                        
                        if let Err(e) = &h3_conn {
                            tracing::error!("      ❌ 建立 H3 连接失败: {:?}", e);
                            return;
                        }
                        let mut h3_conn = h3_conn.unwrap();
                        info!("      📡 H3 连接初始化成功: {}", h3_conn.peer_settings().unwrap_or_default().max_field_section_size());

                        // --- 步骤 H3-6: 循环接受 HTTP/3 请求 --- 
                        // `h3_conn.accept().await` 异步等待此连接上的新 HTTP/3 请求流。
                        loop {
                            match h3_conn.accept().await {
                                Ok(Some((request, stream))) => {
                                    info!("         📥 接收到 H3 请求: {} {}", request.method(), request.uri());
                                    // 对每个请求，也创建一个新 Tokio 任务处理。
                                    tokio::spawn({
                                        let app = app.clone();
                                        async move {
                                            // --- 步骤 H3-7: 将 H3 请求转换为 Axum 能处理的格式 --- 
                                            // (这部分逻辑比较复杂，需要适配 Request/Response 类型)
                                            // 这是一个简化的示例，实际可能需要更复杂的转换
                                            let response = handle_h3_request(request, stream, app).await;
                                            // (发送响应的逻辑也需要适配 H3)
                                            // info!("         📤 发送 H3 响应");
                                        }
                                    });
                                }
                                Ok(None) => {
                                    // 连接关闭
                                    info!("      🚪 H3 连接正常关闭");
                                    break;
                                }
                                Err(e) => {
                                    // 发生错误
                                    tracing::error!("      ❌ 处理 H3 请求/流错误: {:?}", e);
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("❌ 接受 QUIC 连接失败: {:?}", e);
                    }
                }
            }
        });
    }
    info!("🛑 HTTP/3 服务器主循环结束 (可能由于端点关闭)");
}

/// (简化) 处理单个 H3 请求的函数
/// 这里应该将 H3 请求转换为 Axum `Request`，调用 `app.call()`，
/// 然后将 Axum `Response` 转换回 H3 响应。
/// 出于演示目的，这里返回一个固定响应。
async fn handle_h3_request<S>(
    _request: Request<()>, // H3 请求元数据
    mut _stream: RequestStream<S, Bytes>, // H3 请求流
    _app: Router // Axum 应用
) where S: h3::quic::RecvStream + Send + 'static
{
     info!("         ⚙️ (简化) 处理 H3 请求...");
    // 真实的实现会涉及:
    // 1. 从 _request 和 _stream 读取完整的 HTTP 请求 (headers, body)
    // 2. 将其转换为 http::Request<axum::body::Body>
    // 3. let response = app.oneshot(axum_request).await.unwrap();
    // 4. 将 response (http::Response<axum::body::Body>) 转换回 H3 响应
    // 5. 使用 _stream.send_response(...) 发送 H3 响应
    // 6. 处理请求体和响应体流
    
    // 简化: 直接发送固定响应 (这部分未完整实现发送)
    let response = http::Response::builder()
        .status(http::StatusCode::OK)
        .header("content-type", "text/plain")
        .body(())
        .unwrap();
    // match stream.send_response(response).await { ... }
    // match stream.send_data(...).await { ... }
    // stream.finish().await
     info!("         ✅ (简化) H3 请求处理完成 (未实际发送响应)");
}

/// 生成自签名 TLS 证书 (用于开发环境)
/// 
/// 【目的】: 使用 `rcgen` 库动态生成一个临时的、自签名的 TLS 证书和私钥。
/// 【用途】: 主要用于本地开发和测试 HTTPS/HTTP3，避免手动创建证书的麻烦。
/// 【安全警告】: 自签名证书不被浏览器信任，会触发安全警告。**绝不能用于生产环境！**
/// 
/// # 返回值
/// * `Ok((Vec<u8>, Vec<u8>))` - 成功时返回 (PEM 格式的证书, PEM 格式的私钥)。
/// * `Err(Box<dyn std::error::Error>)` - 生成过程中发生任何错误。
fn generate_self_signed_cert() -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
    info!("      🔑 正在生成自签名 TLS 证书 (rcgen)...", );
    // --- 步骤 Cert-1: 定义证书参数 --- 
    // `subject_alt_names` 指定证书适用的域名或 IP 地址。
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
/// * `Ok(quinn::ServerConfig)` - 配置成功的 Quinn 服务器配置对象。
/// * `Err(Box<dyn std::error::Error>)` - 配置过程中发生任何错误。
fn configure_quinn_server(
    cert_pem: Vec<u8>,
    key_pem: Vec<u8>,
) -> Result<quinn::ServerConfig, Box<dyn std::error::Error>> {
    info!("      ⚙️  正在配置 Quinn (QUIC) 服务器...");
    // --- 步骤 QuinnCfg-1: 解析 PEM 证书 --- 
    // `rustls_pemfile::certs` 从 PEM 文本中解析出 DER 编码的证书。
    // `.pop().unwrap()` 取出第一个证书 (假设只有一个)。
    // `CertificateDer::from(...)` 创建 `rustls` 库使用的证书类型。
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
}
*/
