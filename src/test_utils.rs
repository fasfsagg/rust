// src/test_utils.rs
//! 测试工具模块
//!
//! 提供通用的测试工具函数和宏，包括：
//! - 测试服务器启动
//! - HTTP客户端工具
//! - WebSocket测试工具
//! - 数据库测试工具

#![cfg(any(test, feature = "testing"))]

use std::net::SocketAddr;
use axum::Router;
use tokio::net::TcpListener;
use reqwest::Client;
use sea_orm::DatabaseConnection;
use crate::test_config::TestConfig;

/// 测试服务器结构体
pub struct TestServer {
    pub addr: SocketAddr,
    pub client: Client,
    pub db: DatabaseConnection,
    pub config: TestConfig,
}

impl TestServer {
    /// 启动测试服务器
    pub async fn new() -> anyhow::Result<Self> {
        let config = TestConfig::new();
        Self::with_config(config).await
    }

    /// 使用指定配置启动测试服务器
    pub async fn with_config(mut config: TestConfig) -> anyhow::Result<Self> {
        // 获取数据库连接
        let db = config.get_database_connection().await?;

        // 创建简单的测试路由（避免完整的应用初始化）
        let app = Router::new()
            .route(
                "/health",
                axum::routing::get(|| async { "OK" })
            )
            .route(
                "/api/health",
                axum::routing::get(|| async { "API OK" })
            );

        // 绑定到随机端口
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        config.server_port = addr.port();

        // 启动服务器
        tokio::spawn(async move {
            axum::serve(listener, app.into_make_service()).await.expect(
                "Failed to start test server"
            );
        });

        // 等待服务器启动
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let client = Client::new();

        Ok(Self {
            addr,
            client,
            db,
            config,
        })
    }

    /// 获取服务器基础URL
    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// 获取API基础URL
    pub fn api_url(&self) -> String {
        format!("{}/api", self.base_url())
    }

    /// 获取WebSocket URL
    pub fn ws_url(&self) -> String {
        format!("ws://{}/ws", self.addr)
    }

    /// 创建带认证头的HTTP客户端
    pub fn authenticated_client(&self, token: &str) -> Client {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().unwrap()
        );

        Client::builder().default_headers(headers).build().unwrap()
    }
}

/// HTTP测试工具
pub mod http {
    use super::*;
    use serde::Serialize;
    use reqwest::Response;

    /// 发送POST请求
    pub async fn post<T: Serialize>(
        client: &Client,
        url: &str,
        payload: &T
    ) -> anyhow::Result<Response> {
        let response = client.post(url).json(payload).send().await?;
        Ok(response)
    }

    /// 发送GET请求
    pub async fn get(client: &Client, url: &str) -> anyhow::Result<Response> {
        let response = client.get(url).send().await?;
        Ok(response)
    }

    /// 发送PUT请求
    pub async fn put<T: Serialize>(
        client: &Client,
        url: &str,
        payload: &T
    ) -> anyhow::Result<Response> {
        let response = client.put(url).json(payload).send().await?;
        Ok(response)
    }

    /// 发送DELETE请求
    pub async fn delete(client: &Client, url: &str) -> anyhow::Result<Response> {
        let response = client.delete(url).send().await?;
        Ok(response)
    }

    /// 检查响应状态码
    pub fn assert_status(response: &Response, expected: reqwest::StatusCode) {
        assert_eq!(
            response.status(),
            expected,
            "Expected status {}, got {}. Response: {:?}",
            expected,
            response.status(),
            response
        );
    }

    /// 检查响应包含指定内容
    pub async fn assert_contains(response: Response, expected: &str) {
        let body = response.text().await.unwrap();
        assert!(
            body.contains(expected),
            "Response body '{}' does not contain '{}'",
            body,
            expected
        );
    }
}

/// WebSocket测试工具
pub mod websocket {
    use tokio_tungstenite::{ connect_async, tungstenite::Message };
    use futures_util::StreamExt;
    use url::Url;

    /// WebSocket测试客户端
    pub struct WebSocketTestClient {
        pub url: String,
    }

    impl WebSocketTestClient {
        pub fn new(base_url: &str) -> Self {
            Self {
                url: base_url.replace("http://", "ws://") + "/ws",
            }
        }

        /// 连接WebSocket（无认证）
        pub async fn connect(
            &self
        ) -> anyhow::Result<
            (
                futures_util::stream::SplitSink<
                    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
                    Message
                >,
                futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
            )
        > {
            let url = Url::parse(&self.url)?;
            let (ws_stream, _) = connect_async(url).await?;
            let (write, read) = ws_stream.split();
            Ok((write, read))
        }

        /// 连接WebSocket（带JWT认证）
        pub async fn connect_with_token(
            &self,
            token: &str
        ) -> anyhow::Result<
            (
                futures_util::stream::SplitSink<
                    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
                    Message
                >,
                futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
            )
        > {
            let url = format!("{}?token={}", self.url, token);
            let url = Url::parse(&url)?;
            let (ws_stream, _) = connect_async(url).await?;
            let (write, read) = ws_stream.split();
            Ok((write, read))
        }
    }
}

/// 数据库测试工具
pub mod database {
    use sea_orm::DatabaseConnection;

    /// 清理测试数据
    pub async fn cleanup_test_data(db: &DatabaseConnection) -> anyhow::Result<()> {
        // 注意：由于当前项目结构，我们暂时跳过实际的数据清理
        // 在实际使用中，这里应该删除所有测试数据
        // 例如：task::Entity::delete_many().exec(db).await?;
        tracing::info!("清理测试数据（当前为占位符实现）");
        Ok(())
    }

    /// 创建测试用户（占位符实现）
    pub async fn create_test_user(
        _db: &DatabaseConnection,
        username: &str,
        email: &str
    ) -> anyhow::Result<String> {
        // 注意：这是一个占位符实现
        // 在实际使用中，这里应该调用真实的用户服务
        tracing::info!("创建测试用户: {} ({})", username, email);
        Ok(format!("test_user_id_for_{}", username))
    }

    /// 创建测试任务（占位符实现）
    pub async fn create_test_task(
        _db: &DatabaseConnection,
        _user_id: uuid::Uuid,
        title: &str,
        description: &str
    ) -> anyhow::Result<String> {
        // 注意：这是一个占位符实现
        // 在实际使用中，这里应该调用真实的任务服务
        tracing::info!("创建测试任务: {} - {}", title, description);
        Ok(format!("test_task_id_for_{}", title))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_config;

    #[tokio::test]
    async fn test_server_startup() {
        test_config::init_test_logging();

        let server = TestServer::new().await.unwrap();
        assert!(server.addr.port() > 0);
        assert!(!server.base_url().is_empty());
        assert!(!server.api_url().is_empty());
        assert!(!server.ws_url().is_empty());
    }

    #[tokio::test]
    async fn test_http_tools() {
        test_config::init_test_logging();

        let server = TestServer::new().await.unwrap();

        // 测试GET请求
        let response = http
            ::get(&server.client, &format!("{}/health", server.api_url())).await
            .unwrap();
        http::assert_status(&response, reqwest::StatusCode::OK);
    }
}
