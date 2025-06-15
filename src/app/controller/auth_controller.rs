//! src/app/controller/auth_controller.rs
//!
//! 【认证控制器模块】
//! 这个模块实现了用户认证相关的HTTP请求处理函数（Axum Handlers）。
//! 它是控制器层的一部分，负责处理认证相关的HTTP请求和响应。
//!
//! ## 核心功能
//! - **用户注册处理器**: 处理 POST /auth/register 请求
//! - **用户登录处理器**: 处理 POST /auth/login 请求
//!
//! ## 设计原则
//! - **单一职责**: 专注于HTTP请求/响应处理
//! - **薄控制器**: 业务逻辑委托给服务层
//! - **统一错误处理**: 使用项目统一的错误处理机制

use axum::{ extract::{ Json, State }, http::StatusCode, response::Json as ResponseJson };
use serde_json::{ json, Value };
use validator::Validate;

use crate::app::model::auth::{ LoginRequest, RegisterRequest };
use crate::app::repository::user_repository::UserRepository;
use crate::app::service::auth_service::{ login_user, register_user, AuthResponse };
use crate::error::{ AppError, Result };
use crate::startup::AppState;
use std::sync::Arc;

/// 用户注册处理器
///
/// 处理 POST /auth/register 请求，实现用户注册功能。
///
/// # 请求格式
/// ```json
/// {
///   "username": "用户名",
///   "password": "密码",
///   "confirmPassword": "确认密码"
/// }
/// ```
///
/// # 响应格式
/// 成功时返回 201 Created 和用户信息：
/// ```json
/// {
///   "message": "用户注册成功",
///   "user": {
///     "id": "用户ID",
///     "username": "用户名"
///   }
/// }
/// ```
///
/// # 错误处理
/// - 400 Bad Request: 输入验证失败
/// - 409 Conflict: 用户名已存在
/// - 500 Internal Server Error: 服务器内部错误
pub async fn register_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<RegisterRequest>
) -> Result<(StatusCode, ResponseJson<Value>)> {
    println!("AUTH_CONTROLLER: 收到用户注册请求，用户名: {}", payload.username);

    // 1. 输入验证
    if let Err(validation_errors) = payload.validate() {
        println!("AUTH_CONTROLLER: 注册请求验证失败: {:?}", validation_errors);
        return Err(AppError::BadRequest(format!("输入验证失败: {}", validation_errors)));
    }

    // 2. 创建用户仓库实例
    // 从 AppState 中获取数据库连接来创建用户仓库
    let user_repo = Arc::new(UserRepository::new(app_state.db.clone()));

    // 3. 调用服务层处理业务逻辑
    let user_response = register_user(user_repo, payload).await?;

    println!("AUTH_CONTROLLER: 用户注册成功，用户名: {}", user_response.username);

    // 4. 构建成功响应
    let response_body =
        json!({
        "message": "用户注册成功",
        "user": user_response
    });

    Ok((StatusCode::CREATED, ResponseJson(response_body)))
}

/// 用户登录处理器
///
/// 处理 POST /auth/login 请求，实现用户登录功能。
///
/// # 请求格式
/// ```json
/// {
///   "username": "用户名",
///   "password": "密码"
/// }
/// ```
///
/// # 响应格式
/// 成功时返回 200 OK 和认证信息：
/// ```json
/// {
///   "access_token": "JWT令牌",
///   "token_type": "Bearer",
///   "expires_in": 86400,
///   "user": {
///     "id": "用户ID",
///     "username": "用户名"
///   }
/// }
/// ```
///
/// # 错误处理
/// - 400 Bad Request: 输入验证失败
/// - 401 Unauthorized: 用户名或密码错误
/// - 500 Internal Server Error: 服务器内部错误
pub async fn login_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<LoginRequest>
) -> Result<ResponseJson<AuthResponse>> {
    println!("AUTH_CONTROLLER: 收到用户登录请求，用户名: {}", payload.username);

    // 1. 输入验证
    if let Err(validation_errors) = payload.validate() {
        println!("AUTH_CONTROLLER: 登录请求验证失败: {:?}", validation_errors);
        return Err(AppError::BadRequest(format!("输入验证失败: {}", validation_errors)));
    }

    // 2. 创建用户仓库实例
    // 从 AppState 中获取数据库连接来创建用户仓库
    let user_repo = Arc::new(UserRepository::new(app_state.db.clone()));

    // 3. 调用服务层处理业务逻辑
    // 从应用状态中获取 JWT 密钥
    let jwt_secret = &app_state.jwt_secret;
    let auth_response = login_user(user_repo, payload, jwt_secret).await?;

    println!("AUTH_CONTROLLER: 用户登录成功，用户名: {}", auth_response.user.username);

    // 4. 返回认证响应
    Ok(ResponseJson(auth_response))
}
