# Axum 项目安全加固实施计划

## 📋 计划概述

基于安全架构评估报告，本计划提供具体的安全加固实施步骤，将项目从当前的 B+ 安全等级提升到企业级 A 级标准。

**目标**: 在 4 周内完成核心安全加固，达到生产环境部署标准  
**优先级**: 高风险项目优先，关键安全功能先行

## 🎯 第一阶段：紧急安全加固 (第1周)

### 1.1 安全 HTTP 头配置 (优先级: 🔴 高)

**目标**: 防止常见的 Web 攻击（XSS、点击劫持、MIME 嗅探）

**实施步骤**:

1. **创建安全头中间件**
```rust
// src/app/middleware/security_headers.rs
use axum::{
    http::{header, HeaderValue, Request},
    middleware::Next,
    response::Response,
};

pub async fn security_headers_middleware<B>(
    request: Request<B>,
    next: Next<B>,
) -> Response {
    let mut response = next.run(request).await;
    
    let headers = response.headers_mut();
    
    // 防止 XSS 攻击
    headers.insert(
        header::HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    
    // 防止点击劫持
    headers.insert(
        header::HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    
    // 强制 HTTPS (生产环境)
    headers.insert(
        header::HeaderName::from_static("strict-transport-security"),
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    
    // 内容安全策略
    headers.insert(
        header::HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'"),
    );
    
    response
}
```

2. **集成到应用中**
```rust
// src/startup.rs 中添加
let middleware_stack = ServiceBuilder::new()
    .layer(middleware::from_fn(security_headers_middleware))
    .layer(middleware::trace_layer())
    .layer(CorsLayer::new()...);
```

**验证方法**:
```bash
curl -I http://localhost:3000/api/tasks
# 检查响应头是否包含安全头
```

### 1.2 CORS 安全配置 (优先级: 🔴 高)

**目标**: 限制跨域访问，防止 CSRF 攻击

**实施步骤**:

1. **更新 CORS 配置**
```rust
// src/startup.rs
use tower_http::cors::{CorsLayer, AllowOrigin};

let cors_layer = if cfg!(debug_assertions) {
    // 开发环境：允许本地开发
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            origin.as_bytes().starts_with(b"http://localhost") ||
            origin.as_bytes().starts_with(b"http://127.0.0.1")
        }))
} else {
    // 生产环境：严格限制
    CorsLayer::new()
        .allow_origin(AllowOrigin::exact("https://yourdomain.com".parse().unwrap()))
};

let cors_layer = cors_layer
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_headers([AUTHORIZATION, CONTENT_TYPE, header::ACCEPT])
    .allow_credentials(true)
    .max_age(Duration::from_secs(3600));
```

### 1.3 速率限制实现 (优先级: 🟡 中)

**目标**: 防止 DDoS 攻击和暴力破解

**实施步骤**:

1. **添加依赖**
```toml
# Cargo.toml
[dependencies]
tower-governor = "0.4"
```

2. **实现速率限制中间件**
```rust
// src/app/middleware/rate_limit.rs
use tower_governor::{
    governor::GovernorConfigBuilder,
    key_extractor::{SmartIpKeyExtractor, KeyExtractor},
    GovernorLayer,
};

pub fn create_rate_limit_layer() -> GovernorLayer<SmartIpKeyExtractor> {
    let governor_conf = GovernorConfigBuilder::default()
        .per_second(10)  // 每秒最多 10 个请求
        .burst_size(20)  // 突发请求最多 20 个
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .unwrap();
    
    GovernorLayer::new(&governor_conf)
}
```

3. **应用到路由**
```rust
// src/routes.rs
pub fn create_routes(app_state: AppState) -> Router {
    let api_routes = Router::new()
        .route("/tasks", post(create_task).get(get_all_tasks))
        .layer(create_rate_limit_layer()) // 只对 API 路由应用速率限制
        .layer(middleware::from_fn_with_state(
            app_state.jwt_secret.clone(),
            auth_middleware::jwt_auth_middleware,
        ));
    
    // ... 其他路由
}
```

## 🛡️ 第二阶段：认证增强 (第2周)

### 2.1 JWT 增强功能 (优先级: 🔴 高)

**目标**: 实现 Token 撤销和刷新机制

**实施步骤**:

1. **扩展 JWT Claims**
```rust
// src/app/utils/jwt_utils.rs
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnhancedClaims {
    pub sub: String,
    pub username: String,
    pub exp: i64,
    pub iat: i64,
    pub jti: String,      // JWT ID，用于撤销
    pub token_type: String, // "access" 或 "refresh"
    pub session_id: String, // 会话 ID
}
```

2. **实现 Token 黑名单**
```rust
// src/app/service/token_service.rs
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct TokenBlacklist {
    blacklisted_tokens: Arc<RwLock<HashSet<String>>>,
}

impl TokenBlacklist {
    pub fn new() -> Self {
        Self {
            blacklisted_tokens: Arc::new(RwLock::new(HashSet::new())),
        }
    }
    
    pub async fn revoke_token(&self, jti: String) {
        let mut blacklist = self.blacklisted_tokens.write().await;
        blacklist.insert(jti);
    }
    
    pub async fn is_token_revoked(&self, jti: &str) -> bool {
        let blacklist = self.blacklisted_tokens.read().await;
        blacklist.contains(jti)
    }
}
```

3. **实现 Refresh Token 机制**
```rust
// src/app/controller/auth_controller.rs
pub async fn refresh_token_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<RefreshTokenRequest>
) -> Result<ResponseJson<AuthResponse>> {
    // 验证 refresh token
    let claims = validate_refresh_token(&payload.refresh_token, &app_state.jwt_secret)?;
    
    // 生成新的 access token
    let new_access_token = create_access_token(&claims.sub, &claims.username, &app_state.jwt_secret)?;
    
    Ok(ResponseJson(AuthResponse {
        access_token: new_access_token,
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        refresh_token: payload.refresh_token, // 保持原 refresh token
    }))
}
```

### 2.2 会话管理 (优先级: 🟡 中)

**目标**: 实现并发会话控制和会话撤销

**实施步骤**:

1. **会话存储**
```rust
// src/app/service/session_service.rs
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, UserSession>>>,
}

#[derive(Debug, Clone)]
pub struct UserSession {
    pub user_id: Uuid,
    pub session_id: String,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub ip_address: String,
    pub user_agent: String,
}

impl SessionManager {
    pub async fn create_session(&self, user_id: Uuid, ip: String, user_agent: String) -> String {
        let session_id = Uuid::new_v4().to_string();
        let session = UserSession {
            user_id,
            session_id: session_id.clone(),
            created_at: Utc::now(),
            last_activity: Utc::now(),
            ip_address: ip,
            user_agent,
        };
        
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session);
        session_id
    }
    
    pub async fn revoke_session(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id).is_some()
    }
    
    pub async fn get_user_sessions(&self, user_id: Uuid) -> Vec<UserSession> {
        let sessions = self.sessions.read().await;
        sessions.values()
            .filter(|s| s.user_id == user_id)
            .cloned()
            .collect()
    }
}
```

## 🔍 第三阶段：监控和审计 (第3周)

### 3.1 安全事件监控 (优先级: 🟡 中)

**目标**: 实时监控安全事件，及时发现异常

**实施步骤**:

1. **安全事件定义**
```rust
// src/app/model/security_event.rs
#[derive(Debug, Serialize, Deserialize)]
pub enum SecurityEvent {
    LoginSuccess { user_id: String, ip: String },
    LoginFailure { username: String, ip: String, reason: String },
    TokenExpired { user_id: String, token_id: String },
    UnauthorizedAccess { ip: String, endpoint: String },
    RateLimitExceeded { ip: String, endpoint: String },
    SuspiciousActivity { user_id: String, description: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityLog {
    pub event: SecurityEvent,
    pub timestamp: DateTime<Utc>,
    pub severity: SecuritySeverity,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}
```

2. **安全日志记录器**
```rust
// src/app/service/security_logger.rs
pub struct SecurityLogger {
    // 可以集成到外部日志系统，如 ELK Stack
}

impl SecurityLogger {
    pub async fn log_security_event(&self, event: SecurityEvent, severity: SecuritySeverity) {
        let log_entry = SecurityLog {
            event,
            timestamp: Utc::now(),
            severity,
            metadata: HashMap::new(),
        };
        
        // 记录到结构化日志
        tracing::warn!(
            security_event = ?log_entry.event,
            severity = ?log_entry.severity,
            timestamp = %log_entry.timestamp,
            "Security event detected"
        );
        
        // 如果是高危事件，可以发送告警
        if matches!(log_entry.severity, SecuritySeverity::High | SecuritySeverity::Critical) {
            self.send_alert(&log_entry).await;
        }
    }
    
    async fn send_alert(&self, log_entry: &SecurityLog) {
        // 实现告警机制（邮件、Slack、短信等）
        println!("🚨 SECURITY ALERT: {:?}", log_entry);
    }
}
```

### 3.2 审计日志 (优先级: 🟢 低)

**目标**: 记录所有重要操作，支持合规审计

**实施步骤**:

1. **审计中间件**
```rust
// src/app/middleware/audit_middleware.rs
pub async fn audit_middleware<B>(
    request: Request<B>,
    next: Next<B>,
) -> Response {
    let start_time = Utc::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    let user_id = request.extensions()
        .get::<AuthenticatedUser>()
        .map(|u| u.user_id.to_string())
        .unwrap_or_else(|| "anonymous".to_string());
    
    let response = next.run(request).await;
    
    let duration = Utc::now().signed_duration_since(start_time);
    
    tracing::info!(
        user_id = %user_id,
        method = %method,
        uri = %uri,
        status = %response.status(),
        duration_ms = duration.num_milliseconds(),
        "API request audit"
    );
    
    response
}
```

## 🧪 第四阶段：安全测试和验证 (第4周)

### 4.1 自动化安全测试 (优先级: 🟡 中)

**目标**: 确保安全措施有效性

**实施步骤**:

1. **安全测试套件**
```rust
// tests/security_tests.rs
#[tokio::test]
async fn test_rate_limiting() {
    let app = create_test_app().await;
    
    // 发送超过限制的请求
    for i in 0..25 {
        let response = app
            .oneshot(Request::builder()
                .uri("/api/tasks")
                .method("GET")
                .body(Body::empty())
                .unwrap())
            .await
            .unwrap();
        
        if i < 20 {
            assert_eq!(response.status(), StatusCode::OK);
        } else {
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        }
    }
}

#[tokio::test]
async fn test_jwt_security() {
    let app = create_test_app().await;
    
    // 测试过期 token
    let expired_token = create_expired_token();
    let response = app
        .oneshot(Request::builder()
            .uri("/api/tasks")
            .method("GET")
            .header("Authorization", format!("Bearer {}", expired_token))
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_cors_security() {
    let app = create_test_app().await;
    
    // 测试不允许的来源
    let response = app
        .oneshot(Request::builder()
            .uri("/api/tasks")
            .method("GET")
            .header("Origin", "https://malicious-site.com")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();
    
    // 应该被 CORS 策略阻止
    assert!(!response.headers().contains_key("access-control-allow-origin"));
}
```

### 4.2 渗透测试检查清单

**手动测试项目**:

- [ ] **认证绕过测试**: 尝试无 token 访问受保护资源
- [ ] **权限提升测试**: 尝试访问其他用户的资源
- [ ] **SQL 注入测试**: 在各个输入点尝试 SQL 注入
- [ ] **XSS 测试**: 测试脚本注入防护
- [ ] **CSRF 测试**: 测试跨站请求伪造防护
- [ ] **暴力破解测试**: 测试登录速率限制
- [ ] **会话管理测试**: 测试会话固定和劫持防护

## 📊 实施进度跟踪

### 第1周目标
- [x] 安全 HTTP 头配置
- [x] CORS 安全配置  
- [x] 基础速率限制

### 第2周目标
- [ ] JWT 增强功能
- [ ] Token 撤销机制
- [ ] 会话管理

### 第3周目标
- [ ] 安全事件监控
- [ ] 审计日志系统
- [ ] 告警机制

### 第4周目标
- [ ] 自动化安全测试
- [ ] 渗透测试
- [ ] 安全文档更新

## 🎯 成功标准

**技术指标**:
- 安全评分从 B+ 提升到 A
- 所有高风险项目得到解决
- 安全测试覆盖率达到 95%

**业务指标**:
- 通过基础安全合规检查
- 支持生产环境部署
- 满足企业级安全要求

通过以上 4 周的系统性安全加固，项目将具备企业级的安全防护能力，为百万并发移动聊天室应用提供坚实的安全基础。
