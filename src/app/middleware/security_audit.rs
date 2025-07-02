//! 安全审计日志中间件模块
//!
//! 【安全审计日志系统】
//! 这个模块实现了专门的安全审计日志记录功能，用于记录所有安全相关的事件和操作。
//! 它提供了结构化的安全事件记录、审计日志中间件和安全事件分析功能。
//!
//! ## 核心功能
//! - **安全事件记录**: 记录认证、授权、敏感操作等安全事件
//! - **审计日志中间件**: 自动记录HTTP请求的安全相关信息
//! - **威胁检测**: 识别可疑的安全行为模式
//! - **合规性支持**: 满足企业级安全审计要求
//!
//! ## 安全事件类型
//! - 认证事件（登录成功/失败、令牌验证等）
//! - 授权事件（权限检查、访问控制等）
//! - 敏感操作（数据修改、配置变更等）
//! - 安全威胁（暴力破解、异常访问等）
//!
//! ## 设计原则
//! - **零信任**: 记录所有安全相关的操作
//! - **不可篡改**: 确保审计日志的完整性
//! - **实时监控**: 支持实时安全事件分析
//! - **合规性**: 符合企业安全审计标准

use axum::{
    extract::Request,
    http::{HeaderMap, Method, StatusCode, Uri},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{error, info, warn};
use uuid::Uuid;

/// 安全事件类型枚举
///
/// 【功能】：定义所有可能的安全事件类型，用于分类和分析
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SecurityEventType {
    /// 认证成功
    AuthenticationSuccess,
    /// 认证失败
    AuthenticationFailure,
    /// 令牌验证成功
    TokenValidationSuccess,
    /// 令牌验证失败
    TokenValidationFailure,
    /// 授权成功
    AuthorizationSuccess,
    /// 授权失败（权限不足）
    AuthorizationFailure,
    /// 敏感数据访问
    SensitiveDataAccess,
    /// 数据修改操作
    DataModification,
    /// 配置变更
    ConfigurationChange,
    /// 可疑活动检测
    SuspiciousActivity,
    /// 暴力破解尝试
    BruteForceAttempt,
    /// 异常访问模式
    AnomalousAccess,
    /// 会话管理事件
    SessionManagement,
    /// WebSocket连接事件
    WebSocketConnection,
    /// API访问事件
    ApiAccess,
}

/// 安全事件严重级别
///
/// 【功能】：定义安全事件的严重程度，用于优先级处理和告警
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecuritySeverity {
    /// 信息级别 - 正常的安全操作
    Info,
    /// 警告级别 - 需要关注的安全事件
    Warning,
    /// 错误级别 - 安全违规或失败
    Error,
    /// 严重级别 - 重大安全威胁
    Critical,
}

/// 安全审计事件结构体
///
/// 【功能】：记录完整的安全事件信息，包括时间、用户、操作、结果等
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditEvent {
    /// 事件唯一标识符
    pub event_id: String,
    /// 事件时间戳（Unix时间戳）
    pub timestamp: u64,
    /// 事件类型
    pub event_type: SecurityEventType,
    /// 严重级别
    pub severity: SecuritySeverity,
    /// 用户标识（如果已认证）
    pub user_id: Option<String>,
    /// 用户名（如果已认证）
    pub username: Option<String>,
    /// 客户端IP地址
    pub client_ip: Option<String>,
    /// 用户代理字符串
    pub user_agent: Option<String>,
    /// HTTP方法
    pub http_method: Option<String>,
    /// 请求URI
    pub request_uri: Option<String>,
    /// HTTP状态码
    pub status_code: Option<u16>,
    /// 事件描述
    pub description: String,
    /// 额外的上下文信息
    pub context: HashMap<String, String>,
    /// 会话标识符
    pub session_id: Option<String>,
    /// 请求标识符（用于关联请求）
    pub request_id: Option<String>,
}

impl SecurityAuditEvent {
    /// 创建新的安全审计事件
    ///
    /// # 参数
    /// * `event_type` - 事件类型
    /// * `severity` - 严重级别
    /// * `description` - 事件描述
    ///
    /// # 返回值
    /// * `SecurityAuditEvent` - 新创建的安全审计事件
    pub fn new(
        event_type: SecurityEventType,
        severity: SecuritySeverity,
        description: String,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            event_type,
            severity,
            user_id: None,
            username: None,
            client_ip: None,
            user_agent: None,
            http_method: None,
            request_uri: None,
            status_code: None,
            description,
            context: HashMap::new(),
            session_id: None,
            request_id: None,
        }
    }

    /// 设置用户信息
    pub fn with_user(mut self, user_id: String, username: String) -> Self {
        self.user_id = Some(user_id);
        self.username = Some(username);
        self
    }

    /// 设置客户端信息
    pub fn with_client_info(mut self, ip: Option<String>, user_agent: Option<String>) -> Self {
        self.client_ip = ip;
        self.user_agent = user_agent;
        self
    }

    /// 设置HTTP请求信息
    pub fn with_http_info(mut self, method: String, uri: String, status_code: Option<u16>) -> Self {
        self.http_method = Some(method);
        self.request_uri = Some(uri);
        self.status_code = status_code;
        self
    }

    /// 添加上下文信息
    pub fn with_context(mut self, key: String, value: String) -> Self {
        self.context.insert(key, value);
        self
    }

    /// 设置会话和请求标识符
    pub fn with_identifiers(
        mut self,
        session_id: Option<String>,
        request_id: Option<String>,
    ) -> Self {
        self.session_id = session_id;
        self.request_id = request_id;
        self
    }
}

/// 安全审计日志记录器
///
/// 【功能】：负责记录和管理安全审计事件，提供事件分析和威胁检测功能
#[derive(Debug, Clone)]
pub struct SecurityAuditor {
    /// 事件计数器（按类型统计）
    event_counters: Arc<RwLock<HashMap<SecurityEventType, u64>>>,
    /// 失败尝试计数器（按IP统计）
    failure_counters: Arc<RwLock<HashMap<String, u64>>>,
    /// 是否启用威胁检测
    threat_detection_enabled: bool,
    /// 暴力破解检测阈值
    brute_force_threshold: u64,
}

impl Default for SecurityAuditor {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityAuditor {
    /// 创建新的安全审计器
    pub fn new() -> Self {
        Self {
            event_counters: Arc::new(RwLock::new(HashMap::new())),
            failure_counters: Arc::new(RwLock::new(HashMap::new())),
            threat_detection_enabled: true,
            brute_force_threshold: 5, // 5次失败尝试后触发告警
        }
    }

    /// 创建带有自定义配置的安全审计器
    pub fn with_config(threat_detection: bool, brute_force_threshold: u64) -> Self {
        Self {
            event_counters: Arc::new(RwLock::new(HashMap::new())),
            failure_counters: Arc::new(RwLock::new(HashMap::new())),
            threat_detection_enabled: threat_detection,
            brute_force_threshold,
        }
    }

    /// 记录安全审计事件
    ///
    /// 【功能】：记录安全事件到日志系统，并进行威胁检测分析
    ///
    /// # 参数
    /// * `event` - 要记录的安全审计事件
    pub fn log_security_event(&self, event: &SecurityAuditEvent) {
        // 更新事件计数器
        if let Ok(mut counters) = self.event_counters.write() {
            *counters.entry(event.event_type.clone()).or_insert(0) += 1;
        }

        // 威胁检测逻辑
        if self.threat_detection_enabled {
            self.detect_threats(event);
        }

        // 根据严重级别选择合适的日志级别记录事件
        match event.severity {
            SecuritySeverity::Info => {
                info!(
                    event_id = %event.event_id,
                    event_type = ?event.event_type,
                    severity = ?event.severity,
                    user_id = ?event.user_id,
                    username = ?event.username,
                    client_ip = ?event.client_ip,
                    user_agent = ?event.user_agent,
                    http_method = ?event.http_method,
                    request_uri = ?event.request_uri,
                    status_code = ?event.status_code,
                    description = %event.description,
                    context = ?event.context,
                    session_id = ?event.session_id,
                    request_id = ?event.request_id,
                    timestamp = event.timestamp,
                    "Security audit event recorded"
                );
            }
            SecuritySeverity::Warning => {
                warn!(
                    event_id = %event.event_id,
                    event_type = ?event.event_type,
                    severity = ?event.severity,
                    user_id = ?event.user_id,
                    username = ?event.username,
                    client_ip = ?event.client_ip,
                    user_agent = ?event.user_agent,
                    http_method = ?event.http_method,
                    request_uri = ?event.request_uri,
                    status_code = ?event.status_code,
                    description = %event.description,
                    context = ?event.context,
                    session_id = ?event.session_id,
                    request_id = ?event.request_id,
                    timestamp = event.timestamp,
                    "Security warning event recorded"
                );
            }
            SecuritySeverity::Error | SecuritySeverity::Critical => {
                error!(
                    event_id = %event.event_id,
                    event_type = ?event.event_type,
                    severity = ?event.severity,
                    user_id = ?event.user_id,
                    username = ?event.username,
                    client_ip = ?event.client_ip,
                    user_agent = ?event.user_agent,
                    http_method = ?event.http_method,
                    request_uri = ?event.request_uri,
                    status_code = ?event.status_code,
                    description = %event.description,
                    context = ?event.context,
                    session_id = ?event.session_id,
                    request_id = ?event.request_id,
                    timestamp = event.timestamp,
                    "Security critical event recorded"
                );
            }
        }
    }

    /// 威胁检测逻辑
    ///
    /// 【功能】：分析安全事件，检测潜在的安全威胁
    ///
    /// # 参数
    /// * `event` - 要分析的安全审计事件
    fn detect_threats(&self, event: &SecurityAuditEvent) {
        match event.event_type {
            SecurityEventType::AuthenticationFailure
            | SecurityEventType::TokenValidationFailure => {
                if let Some(client_ip) = &event.client_ip {
                    self.check_brute_force_attempt(client_ip, event);
                }
            }
            SecurityEventType::AnomalousAccess => {
                // 记录异常访问模式
                warn!(
                    event_id = %event.event_id,
                    client_ip = ?event.client_ip,
                    "Anomalous access pattern detected"
                );
            }
            SecurityEventType::SuspiciousActivity => {
                // 记录可疑活动
                error!(
                    event_id = %event.event_id,
                    client_ip = ?event.client_ip,
                    user_id = ?event.user_id,
                    "Suspicious activity detected - requires immediate attention"
                );
            }
            _ => {
                // 其他事件类型的威胁检测逻辑可以在这里添加
            }
        }
    }

    /// 检测暴力破解尝试
    ///
    /// 【功能】：监控来自同一IP的认证失败次数，检测暴力破解攻击
    ///
    /// # 参数
    /// * `client_ip` - 客户端IP地址
    /// * `event` - 当前的安全审计事件
    fn check_brute_force_attempt(&self, client_ip: &str, event: &SecurityAuditEvent) {
        if let Ok(mut counters) = self.failure_counters.write() {
            let count = counters.entry(client_ip.to_string()).or_insert(0);
            *count += 1;

            if *count >= self.brute_force_threshold {
                // 检测到暴力破解尝试
                let brute_force_event = SecurityAuditEvent::new(
                    SecurityEventType::BruteForceAttempt,
                    SecuritySeverity::Critical,
                    format!(
                        "Brute force attack detected from IP: {} ({} failed attempts)",
                        client_ip, count
                    ),
                )
                .with_client_info(Some(client_ip.to_string()), event.user_agent.clone())
                .with_context("failure_count".to_string(), count.to_string())
                .with_context(
                    "threshold".to_string(),
                    self.brute_force_threshold.to_string(),
                );

                // 记录暴力破解事件（避免递归调用detect_threats）
                error!(
                    event_id = %brute_force_event.event_id,
                    event_type = ?brute_force_event.event_type,
                    severity = ?brute_force_event.severity,
                    client_ip = ?brute_force_event.client_ip,
                    description = %brute_force_event.description,
                    context = ?brute_force_event.context,
                    timestamp = brute_force_event.timestamp,
                    "CRITICAL: Brute force attack detected"
                );
            }
        }
    }

    /// 重置失败计数器（用于成功认证后清除计数）
    ///
    /// 【功能】：当用户成功认证后，清除该IP的失败计数
    ///
    /// # 参数
    /// * `client_ip` - 客户端IP地址
    pub fn reset_failure_count(&self, client_ip: &str) {
        if let Ok(mut counters) = self.failure_counters.write() {
            counters.remove(client_ip);
        }
    }

    /// 获取事件统计信息
    ///
    /// 【功能】：返回各类安全事件的统计计数
    ///
    /// # 返回值
    /// * `HashMap<SecurityEventType, u64>` - 事件类型和对应的计数
    pub fn get_event_statistics(&self) -> HashMap<SecurityEventType, u64> {
        self.event_counters
            .read()
            .map(|counters| counters.clone())
            .unwrap_or_default()
    }

    /// 获取失败尝试统计信息
    ///
    /// 【功能】：返回各IP地址的失败尝试计数
    ///
    /// # 返回值
    /// * `HashMap<String, u64>` - IP地址和对应的失败计数
    pub fn get_failure_statistics(&self) -> HashMap<String, u64> {
        self.failure_counters
            .read()
            .map(|counters| counters.clone())
            .unwrap_or_default()
    }
}

/// 全局安全审计器实例
static GLOBAL_SECURITY_AUDITOR: std::sync::OnceLock<SecurityAuditor> = std::sync::OnceLock::new();

/// 获取全局安全审计器实例
///
/// 【功能】：获取全局唯一的安全审计器实例，确保整个应用程序使用同一个审计器
///
/// # 返回值
/// * `&'static SecurityAuditor` - 全局安全审计器的引用
pub fn get_security_auditor() -> &'static SecurityAuditor {
    GLOBAL_SECURITY_AUDITOR.get_or_init(SecurityAuditor::new)
}

/// 初始化全局安全审计器
///
/// 【功能】：使用自定义配置初始化全局安全审计器
///
/// # 参数
/// * `threat_detection` - 是否启用威胁检测
/// * `brute_force_threshold` - 暴力破解检测阈值
///
/// # 返回值
/// * `Result<(), &'static str>` - 初始化结果
pub fn init_security_auditor(
    threat_detection: bool,
    brute_force_threshold: u64,
) -> Result<(), &'static str> {
    GLOBAL_SECURITY_AUDITOR
        .set(SecurityAuditor::with_config(
            threat_detection,
            brute_force_threshold,
        ))
        .map_err(|_| "Security auditor already initialized")
}

/// 便捷函数：记录认证成功事件
///
/// # 参数
/// * `user_id` - 用户ID
/// * `username` - 用户名
/// * `client_ip` - 客户端IP
/// * `user_agent` - 用户代理
pub fn log_authentication_success(
    user_id: &str,
    username: &str,
    client_ip: Option<&str>,
    user_agent: Option<&str>,
) {
    let event = SecurityAuditEvent::new(
        SecurityEventType::AuthenticationSuccess,
        SecuritySeverity::Info,
        format!("User '{}' authenticated successfully", username),
    )
    .with_user(user_id.to_string(), username.to_string())
    .with_client_info(
        client_ip.map(|s| s.to_string()),
        user_agent.map(|s| s.to_string()),
    );

    let auditor = get_security_auditor();
    auditor.log_security_event(&event);

    // 成功认证后重置失败计数
    if let Some(ip) = client_ip {
        auditor.reset_failure_count(ip);
    }
}

/// 便捷函数：记录认证失败事件
///
/// # 参数
/// * `username` - 尝试认证的用户名
/// * `reason` - 失败原因
/// * `client_ip` - 客户端IP
/// * `user_agent` - 用户代理
pub fn log_authentication_failure(
    username: Option<&str>,
    reason: &str,
    client_ip: Option<&str>,
    user_agent: Option<&str>,
) {
    let description = if let Some(user) = username {
        format!("Authentication failed for user '{}': {}", user, reason)
    } else {
        format!("Authentication failed: {}", reason)
    };

    let mut event = SecurityAuditEvent::new(
        SecurityEventType::AuthenticationFailure,
        SecuritySeverity::Warning,
        description,
    )
    .with_client_info(
        client_ip.map(|s| s.to_string()),
        user_agent.map(|s| s.to_string()),
    )
    .with_context("failure_reason".to_string(), reason.to_string());

    if let Some(user) = username {
        event = event.with_context("attempted_username".to_string(), user.to_string());
    }

    get_security_auditor().log_security_event(&event);
}

/// 便捷函数：记录令牌验证事件
///
/// # 参数
/// * `success` - 验证是否成功
/// * `user_id` - 用户ID（如果验证成功）
/// * `username` - 用户名（如果验证成功）
/// * `reason` - 失败原因（如果验证失败）
/// * `client_ip` - 客户端IP
/// * `user_agent` - 用户代理
pub fn log_token_validation(
    success: bool,
    user_id: Option<&str>,
    username: Option<&str>,
    reason: Option<&str>,
    client_ip: Option<&str>,
    user_agent: Option<&str>,
) {
    let (event_type, severity, description) = if success {
        (
            SecurityEventType::TokenValidationSuccess,
            SecuritySeverity::Info,
            format!(
                "Token validation successful for user '{}'",
                username.unwrap_or("unknown")
            ),
        )
    } else {
        (
            SecurityEventType::TokenValidationFailure,
            SecuritySeverity::Warning,
            format!(
                "Token validation failed: {}",
                reason.unwrap_or("unknown reason")
            ),
        )
    };

    let mut event = SecurityAuditEvent::new(event_type, severity, description).with_client_info(
        client_ip.map(|s| s.to_string()),
        user_agent.map(|s| s.to_string()),
    );

    if success {
        if let (Some(uid), Some(uname)) = (user_id, username) {
            event = event.with_user(uid.to_string(), uname.to_string());
        }
    } else if let Some(r) = reason {
        event = event.with_context("failure_reason".to_string(), r.to_string());
    }

    get_security_auditor().log_security_event(&event);
}

/// 便捷函数：记录授权事件
///
/// # 参数
/// * `success` - 授权是否成功
/// * `user_id` - 用户ID
/// * `username` - 用户名
/// * `resource` - 访问的资源
/// * `action` - 执行的操作
/// * `client_ip` - 客户端IP
pub fn log_authorization_event(
    success: bool,
    user_id: &str,
    username: &str,
    resource: &str,
    action: &str,
    client_ip: Option<&str>,
) {
    let (event_type, severity, description) = if success {
        (
            SecurityEventType::AuthorizationSuccess,
            SecuritySeverity::Info,
            format!(
                "User '{}' authorized to {} on resource '{}'",
                username, action, resource
            ),
        )
    } else {
        (
            SecurityEventType::AuthorizationFailure,
            SecuritySeverity::Warning,
            format!(
                "User '{}' denied access to {} on resource '{}'",
                username, action, resource
            ),
        )
    };

    let event = SecurityAuditEvent::new(event_type, severity, description)
        .with_user(user_id.to_string(), username.to_string())
        .with_client_info(client_ip.map(|s| s.to_string()), None)
        .with_context("resource".to_string(), resource.to_string())
        .with_context("action".to_string(), action.to_string());

    get_security_auditor().log_security_event(&event);
}

/// 便捷函数：记录敏感数据访问事件
///
/// # 参数
/// * `user_id` - 用户ID
/// * `username` - 用户名
/// * `data_type` - 数据类型
/// * `operation` - 操作类型（读取、修改、删除等）
/// * `client_ip` - 客户端IP
pub fn log_sensitive_data_access(
    user_id: &str,
    username: &str,
    data_type: &str,
    operation: &str,
    client_ip: Option<&str>,
) {
    let event = SecurityAuditEvent::new(
        SecurityEventType::SensitiveDataAccess,
        SecuritySeverity::Info,
        format!(
            "User '{}' performed '{}' operation on sensitive data type '{}'",
            username, operation, data_type
        ),
    )
    .with_user(user_id.to_string(), username.to_string())
    .with_client_info(client_ip.map(|s| s.to_string()), None)
    .with_context("data_type".to_string(), data_type.to_string())
    .with_context("operation".to_string(), operation.to_string());

    get_security_auditor().log_security_event(&event);
}

/// 便捷函数：记录可疑活动
///
/// # 参数
/// * `description` - 可疑活动描述
/// * `user_id` - 用户ID（如果已知）
/// * `username` - 用户名（如果已知）
/// * `client_ip` - 客户端IP
/// * `context` - 额外的上下文信息
pub fn log_suspicious_activity(
    description: &str,
    user_id: Option<&str>,
    username: Option<&str>,
    client_ip: Option<&str>,
    context: HashMap<String, String>,
) {
    let mut event = SecurityAuditEvent::new(
        SecurityEventType::SuspiciousActivity,
        SecuritySeverity::Error,
        description.to_string(),
    )
    .with_client_info(client_ip.map(|s| s.to_string()), None);

    if let (Some(uid), Some(uname)) = (user_id, username) {
        event = event.with_user(uid.to_string(), uname.to_string());
    }

    // 添加所有上下文信息
    for (key, value) in context {
        event = event.with_context(key, value);
    }

    get_security_auditor().log_security_event(&event);
}

/// 安全审计中间件
///
/// 【功能】：自动记录所有HTTP请求的安全相关信息，包括：
/// - 请求方法、URI、状态码
/// - 客户端IP和用户代理
/// - 认证用户信息（如果存在）
/// - 请求处理时间
/// - 异常访问模式检测
///
/// # 使用方法
/// ```rust,no_run
/// use axum::Router;
/// use tower::ServiceBuilder;
/// use axum_tutorial::app::middleware::security_audit::security_audit_middleware;
///
/// let app: Router = Router::new()
///     .layer(ServiceBuilder::new().layer(axum::middleware::from_fn(security_audit_middleware)));
/// ```
pub async fn security_audit_middleware(req: Request, next: Next) -> Result<Response, StatusCode> {
    let start_time = std::time::Instant::now();

    // 提取请求信息
    let method = req.method().clone();
    let uri = req.uri().clone();
    let headers = req.headers().clone();

    // 提取客户端IP（从多个可能的头部中尝试）
    let client_ip = extract_client_ip(&headers);

    // 提取用户代理
    let user_agent = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // 检查是否有认证用户信息
    let auth_user = req
        .extensions()
        .get::<crate::app::middleware::auth_middleware::AuthenticatedUser>()
        .cloned();

    // 生成请求ID用于跟踪
    let request_id = Uuid::new_v4().to_string();

    // 执行请求处理
    let response = next.run(req).await;

    // 计算处理时间
    let processing_time = start_time.elapsed();
    let status_code = response.status();

    // 记录API访问事件
    let mut event = SecurityAuditEvent::new(
        SecurityEventType::ApiAccess,
        SecuritySeverity::Info,
        format!("{} {} - {}", method, uri, status_code),
    )
    .with_client_info(client_ip.clone(), user_agent.clone())
    .with_http_info(
        method.to_string(),
        uri.to_string(),
        Some(status_code.as_u16()),
    )
    .with_context(
        "processing_time_ms".to_string(),
        processing_time.as_millis().to_string(),
    )
    .with_identifiers(None, Some(request_id));

    // 如果有认证用户，添加用户信息
    if let Some(user) = &auth_user {
        event = event.with_user(user.user_id.clone(), user.username.clone());
    }

    // 检测异常访问模式
    if should_flag_as_suspicious(&method, &uri, status_code, &processing_time) {
        let suspicious_event = SecurityAuditEvent::new(
            SecurityEventType::AnomalousAccess,
            SecuritySeverity::Warning,
            format!(
                "Anomalous access pattern detected: {} {} - {}",
                method, uri, status_code
            ),
        )
        .with_client_info(client_ip.clone(), user_agent.clone())
        .with_http_info(
            method.to_string(),
            uri.to_string(),
            Some(status_code.as_u16()),
        )
        .with_context(
            "processing_time_ms".to_string(),
            processing_time.as_millis().to_string(),
        );

        get_security_auditor().log_security_event(&suspicious_event);
    }

    // 记录主要的API访问事件
    get_security_auditor().log_security_event(&event);

    Ok(response)
}

/// 从HTTP头部提取客户端IP地址
///
/// 【功能】：尝试从多个可能的HTTP头部中提取真实的客户端IP地址
/// 优先级：X-Forwarded-For > X-Real-IP > X-Client-IP > 连接IP
///
/// # 参数
/// * `headers` - HTTP请求头部
///
/// # 返回值
/// * `Option<String>` - 客户端IP地址（如果找到）
fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    // 尝试从 X-Forwarded-For 头部获取（最常见的代理头部）
    if let Some(forwarded_for) = headers.get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded_for.to_str() {
            // X-Forwarded-For 可能包含多个IP，取第一个
            if let Some(first_ip) = forwarded_str.split(',').next() {
                return Some(first_ip.trim().to_string());
            }
        }
    }

    // 尝试从 X-Real-IP 头部获取
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            return Some(ip_str.to_string());
        }
    }

    // 尝试从 X-Client-IP 头部获取
    if let Some(client_ip) = headers.get("x-client-ip") {
        if let Ok(ip_str) = client_ip.to_str() {
            return Some(ip_str.to_string());
        }
    }

    // 如果都没有找到，返回None（实际的连接IP需要从连接信息中获取）
    None
}

/// 检测是否应该标记为可疑访问
///
/// 【功能】：基于请求特征判断是否为异常访问模式
///
/// # 参数
/// * `method` - HTTP方法
/// * `uri` - 请求URI
/// * `status_code` - 响应状态码
/// * `processing_time` - 处理时间
///
/// # 返回值
/// * `bool` - 是否应该标记为可疑
fn should_flag_as_suspicious(
    method: &Method,
    uri: &Uri,
    status_code: StatusCode,
    processing_time: &std::time::Duration,
) -> bool {
    // 检测可疑的状态码模式
    if matches!(
        status_code,
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
    ) {
        return true;
    }

    // 检测异常长的处理时间（超过5秒）
    if processing_time.as_secs() > 5 {
        return true;
    }

    // 检测对敏感端点的访问
    let path = uri.path();
    let sensitive_paths = [
        "/admin",
        "/config",
        "/debug",
        "/internal",
        "/.env",
        "/backup",
        "/logs",
    ];

    if sensitive_paths
        .iter()
        .any(|&sensitive| path.contains(sensitive))
    {
        return true;
    }

    // 检测可疑的HTTP方法组合
    if method == Method::DELETE && !path.starts_with("/api/") {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderValue, Method, StatusCode, Uri};
    use std::time::Duration;

    #[test]
    fn test_security_event_creation() {
        let event = SecurityAuditEvent::new(
            SecurityEventType::AuthenticationSuccess,
            SecuritySeverity::Info,
            "Test authentication success".to_string(),
        );

        assert_eq!(event.event_type, SecurityEventType::AuthenticationSuccess);
        assert_eq!(event.severity, SecuritySeverity::Info);
        assert_eq!(event.description, "Test authentication success");
        assert!(event.user_id.is_none());
        assert!(event.username.is_none());
        assert!(event.client_ip.is_none());
    }

    #[test]
    fn test_security_event_with_user_info() {
        let event = SecurityAuditEvent::new(
            SecurityEventType::AuthenticationSuccess,
            SecuritySeverity::Info,
            "Test authentication success".to_string(),
        )
        .with_user("user123".to_string(), "testuser".to_string())
        .with_client_info(
            Some("192.168.1.1".to_string()),
            Some("Mozilla/5.0".to_string()),
        );

        assert_eq!(event.user_id, Some("user123".to_string()));
        assert_eq!(event.username, Some("testuser".to_string()));
        assert_eq!(event.client_ip, Some("192.168.1.1".to_string()));
        assert_eq!(event.user_agent, Some("Mozilla/5.0".to_string()));
    }

    #[test]
    fn test_security_event_with_context() {
        let event = SecurityAuditEvent::new(
            SecurityEventType::AuthenticationFailure,
            SecuritySeverity::Warning,
            "Test authentication failure".to_string(),
        )
        .with_context("reason".to_string(), "invalid_password".to_string())
        .with_context("attempt_count".to_string(), "3".to_string());

        assert_eq!(
            event.context.get("reason"),
            Some(&"invalid_password".to_string())
        );
        assert_eq!(event.context.get("attempt_count"), Some(&"3".to_string()));
    }

    #[test]
    fn test_security_auditor_creation() {
        let auditor = SecurityAuditor::new();
        assert!(auditor.threat_detection_enabled);
        assert_eq!(auditor.brute_force_threshold, 5);
    }

    #[test]
    fn test_security_auditor_with_config() {
        let auditor = SecurityAuditor::with_config(false, 10);
        assert!(!auditor.threat_detection_enabled);
        assert_eq!(auditor.brute_force_threshold, 10);
    }

    #[test]
    fn test_security_auditor_event_logging() {
        let auditor = SecurityAuditor::new();
        let event = SecurityAuditEvent::new(
            SecurityEventType::AuthenticationSuccess,
            SecuritySeverity::Info,
            "Test event".to_string(),
        );

        // 记录事件
        auditor.log_security_event(&event);

        // 检查统计信息
        let stats = auditor.get_event_statistics();
        assert_eq!(
            stats.get(&SecurityEventType::AuthenticationSuccess),
            Some(&1)
        );
    }

    #[test]
    fn test_brute_force_detection() {
        let auditor = SecurityAuditor::with_config(true, 3);
        let client_ip = "192.168.1.100";

        // 模拟多次失败尝试
        for i in 1..=5 {
            let event = SecurityAuditEvent::new(
                SecurityEventType::AuthenticationFailure,
                SecuritySeverity::Warning,
                format!("Failed attempt {}", i),
            )
            .with_client_info(Some(client_ip.to_string()), None);

            auditor.log_security_event(&event);
        }

        // 检查失败计数
        let failure_stats = auditor.get_failure_statistics();
        assert_eq!(failure_stats.get(client_ip), Some(&5));
    }

    #[test]
    fn test_reset_failure_count() {
        let auditor = SecurityAuditor::new();
        let client_ip = "192.168.1.200";

        // 模拟失败尝试
        let event = SecurityAuditEvent::new(
            SecurityEventType::AuthenticationFailure,
            SecuritySeverity::Warning,
            "Failed attempt".to_string(),
        )
        .with_client_info(Some(client_ip.to_string()), None);

        auditor.log_security_event(&event);

        // 检查失败计数存在
        let failure_stats = auditor.get_failure_statistics();
        assert_eq!(failure_stats.get(client_ip), Some(&1));

        // 重置失败计数
        auditor.reset_failure_count(client_ip);

        // 检查失败计数已清除
        let failure_stats = auditor.get_failure_statistics();
        assert_eq!(failure_stats.get(client_ip), None);
    }

    #[test]
    fn test_extract_client_ip_from_x_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("203.0.113.1, 198.51.100.1, 192.168.1.1"),
        );

        let client_ip = extract_client_ip(&headers);
        assert_eq!(client_ip, Some("203.0.113.1".to_string()));
    }

    #[test]
    fn test_extract_client_ip_from_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", HeaderValue::from_static("203.0.113.2"));

        let client_ip = extract_client_ip(&headers);
        assert_eq!(client_ip, Some("203.0.113.2".to_string()));
    }

    #[test]
    fn test_extract_client_ip_no_headers() {
        let headers = HeaderMap::new();
        let client_ip = extract_client_ip(&headers);
        assert_eq!(client_ip, None);
    }

    #[test]
    fn test_should_flag_as_suspicious_unauthorized() {
        let method = Method::GET;
        let uri: Uri = "/api/users".parse().unwrap();
        let status_code = StatusCode::UNAUTHORIZED;
        let processing_time = Duration::from_millis(100);

        assert!(should_flag_as_suspicious(
            &method,
            &uri,
            status_code,
            &processing_time
        ));
    }

    #[test]
    fn test_should_flag_as_suspicious_long_processing_time() {
        let method = Method::GET;
        let uri: Uri = "/api/users".parse().unwrap();
        let status_code = StatusCode::OK;
        let processing_time = Duration::from_secs(6);

        assert!(should_flag_as_suspicious(
            &method,
            &uri,
            status_code,
            &processing_time
        ));
    }

    #[test]
    fn test_should_flag_as_suspicious_sensitive_path() {
        let method = Method::GET;
        let uri: Uri = "/admin/config".parse().unwrap();
        let status_code = StatusCode::OK;
        let processing_time = Duration::from_millis(100);

        assert!(should_flag_as_suspicious(
            &method,
            &uri,
            status_code,
            &processing_time
        ));
    }

    #[test]
    fn test_should_not_flag_normal_request() {
        let method = Method::GET;
        let uri: Uri = "/api/users".parse().unwrap();
        let status_code = StatusCode::OK;
        let processing_time = Duration::from_millis(100);

        assert!(!should_flag_as_suspicious(
            &method,
            &uri,
            status_code,
            &processing_time
        ));
    }
}
