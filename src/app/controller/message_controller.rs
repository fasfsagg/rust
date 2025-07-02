//! `message_controller.rs`
//!
//! 消息控制器模块，提供消息搜索和过滤的HTTP API端点。
//! 实现了企业级聊天应用的消息查询功能，支持关键词搜索、高级过滤和分页。
//!
//! ## 设计原则
//! - **RESTful API**: 遵循REST设计原则，提供清晰的API接口
//! - **参数验证**: 严格验证输入参数，确保数据安全性
//! - **错误处理**: 完善的错误处理机制，提供友好的错误信息
//! - **性能优化**: 支持分页查询，避免大数据集的性能问题
//! - **JWT认证**: 所有端点都需要JWT认证，确保安全性
//!
//! ## API端点
//! - `GET /api/messages/search`: 搜索消息（支持关键词和过滤）
//! - `GET /api/messages/chat-room/{id}`: 获取聊天室消息（支持过滤和分页）

use axum::{ extract::{ Path, Query, State, Extension }, response::Json };
use chrono::{ DateTime, Utc };
use sea_orm::prelude::Uuid;
use serde::{ Deserialize, Serialize };

use crate::{
    app::{
        middleware::auth_middleware::AuthenticatedUser,
        repository::message_repository::{
            MessageFilter,
            MessagePaginationParams,
            MessagePaginationResult,
            MessageRepository,
            MessageRepositoryContract,
        },
    },
    error::{ AppError, Result },
    startup::AppState,
};
use migration::message_entity::{ MessageStatus, MessageType };

/// 消息搜索请求参数
#[derive(Debug, Deserialize)]
pub struct MessageSearchQuery {
    /// 搜索关键词（必需）
    pub keyword: String,
    /// 页码（从1开始，默认为1）
    pub page: Option<u64>,
    /// 每页大小（默认为50，最大100）
    pub page_size: Option<u64>,
    /// 是否按时间倒序（默认为true）
    pub desc_order: Option<bool>,
    /// 消息类型过滤
    pub message_type: Option<String>,
    /// 消息状态过滤
    pub status: Option<String>,
    /// 发送者ID过滤
    pub sender_id: Option<String>,
    /// 聊天室ID过滤（可选，限制搜索范围）
    pub chat_room_id: Option<String>,
    /// 开始时间过滤（ISO 8601格式）
    pub start_time: Option<String>,
    /// 结束时间过滤（ISO 8601格式）
    pub end_time: Option<String>,
}

/// 聊天室消息查询参数
#[derive(Debug, Deserialize)]
pub struct ChatRoomMessageQuery {
    /// 页码（从1开始，默认为1）
    pub page: Option<u64>,
    /// 每页大小（默认为50，最大100）
    pub page_size: Option<u64>,
    /// 是否按时间倒序（默认为true）
    pub desc_order: Option<bool>,
    /// 消息类型过滤
    pub message_type: Option<String>,
    /// 消息状态过滤
    pub status: Option<String>,
    /// 发送者ID过滤
    pub sender_id: Option<String>,
    /// 关键词搜索（可选）
    pub keyword: Option<String>,
    /// 开始时间过滤（ISO 8601格式）
    pub start_time: Option<String>,
    /// 结束时间过滤（ISO 8601格式）
    pub end_time: Option<String>,
}

/// 消息响应结构体
#[derive(Debug, Serialize)]
pub struct MessageResponse {
    /// 消息ID
    pub id: String,
    /// 消息内容
    pub content: String,
    /// 消息类型
    pub message_type: String,
    /// 消息状态
    pub status: String,
    /// 发送者ID
    pub sender_id: String,
    /// 聊天室ID
    pub chat_room_id: String,
    /// 回复的消息ID
    pub reply_to_id: Option<String>,
    /// 消息元数据
    pub metadata: Option<String>,
    /// 消息优先级
    pub priority: i32,
    /// 是否置顶
    pub is_pinned: bool,
    /// 过期时间
    pub expires_at: Option<DateTime<Utc>>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 分页响应结构体
#[derive(Debug, Serialize)]
pub struct PaginatedMessageResponse {
    /// 消息列表
    pub messages: Vec<MessageResponse>,
    /// 分页信息
    pub pagination: PaginationInfo,
}

/// 分页信息结构体
#[derive(Debug, Serialize)]
pub struct PaginationInfo {
    /// 总消息数量
    pub total_count: u64,
    /// 总页数
    pub total_pages: u64,
    /// 当前页码
    pub current_page: u64,
    /// 每页大小
    pub page_size: u64,
    /// 是否有下一页
    pub has_next: bool,
    /// 是否有上一页
    pub has_prev: bool,
}

/// 搜索消息
///
/// # 端点
/// `GET /api/messages/search`
///
/// # 参数
/// - `keyword`: 搜索关键词（必需）
/// - `page`: 页码（可选，默认1）
/// - `page_size`: 每页大小（可选，默认50，最大100）
/// - 其他过滤参数（可选）
///
/// # 返回
/// 成功时返回分页的消息搜索结果
pub async fn search_messages(
    State(app_state): State<AppState>,
    Extension(_user): Extension<AuthenticatedUser>, // JWT认证
    Query(query): Query<MessageSearchQuery>
) -> Result<Json<PaginatedMessageResponse>> {
    // 验证关键词
    if query.keyword.trim().is_empty() {
        return Err(AppError::BadRequest("搜索关键词不能为空".to_string()));
    }

    // 构建分页参数
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(50).clamp(1, 100);
    let desc_order = query.desc_order.unwrap_or(true);

    let pagination_params = MessagePaginationParams {
        page,
        page_size,
        desc_order,
    };

    // 构建过滤器
    let filter = build_message_filter(&query)?;

    // 创建消息仓库实例
    let message_repository = MessageRepository::from_arc(app_state.db.clone());

    // 执行搜索
    let result = message_repository
        .search_messages(query.keyword, pagination_params, filter).await
        .map_err(AppError::DbErr)?;

    // 转换响应格式
    let response = convert_to_paginated_response(result);

    Ok(Json(response))
}

/// 获取聊天室消息
///
/// # 端点
/// `GET /api/messages/chat-room/{id}`
///
/// # 参数
/// - `id`: 聊天室ID（路径参数）
/// - 查询参数：分页和过滤选项
///
/// # 返回
/// 成功时返回分页的聊天室消息列表
pub async fn get_chat_room_messages(
    State(app_state): State<AppState>,
    Extension(_user): Extension<AuthenticatedUser>, // JWT认证
    Path(chat_room_id): Path<String>,
    Query(query): Query<ChatRoomMessageQuery>
) -> Result<Json<PaginatedMessageResponse>> {
    // 解析聊天室ID
    let chat_room_uuid = Uuid::parse_str(&chat_room_id).map_err(|_|
        AppError::BadRequest("无效的聊天室ID格式".to_string())
    )?;

    // 构建分页参数
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(50).clamp(1, 100);
    let desc_order = query.desc_order.unwrap_or(true);

    let pagination_params = MessagePaginationParams {
        page,
        page_size,
        desc_order,
    };

    // 构建过滤器
    let filter = build_chat_room_message_filter(&query)?;

    // 创建消息仓库实例
    let message_repository = MessageRepository::from_arc(app_state.db.clone());

    // 执行查询
    let result = message_repository
        .find_by_chat_room(chat_room_uuid, pagination_params, filter).await
        .map_err(AppError::DbErr)?;

    // 转换响应格式
    let response = convert_to_paginated_response(result);

    Ok(Json(response))
}

/// 构建消息搜索过滤器
fn build_message_filter(query: &MessageSearchQuery) -> Result<Option<MessageFilter>> {
    let mut filter = MessageFilter::default();
    let mut has_filter = false;

    // 消息类型过滤
    if let Some(ref message_type_str) = query.message_type {
        let message_type = parse_message_type(message_type_str)?;
        filter.message_type = Some(message_type);
        has_filter = true;
    }

    // 消息状态过滤
    if let Some(ref status_str) = query.status {
        let status = parse_message_status(status_str)?;
        filter.status = Some(status);
        has_filter = true;
    }

    // 发送者ID过滤
    if let Some(ref sender_id_str) = query.sender_id {
        let sender_id = Uuid::parse_str(sender_id_str).map_err(|_|
            AppError::BadRequest("无效的发送者ID格式".to_string())
        )?;
        filter.sender_id = Some(sender_id);
        has_filter = true;
    }

    // 聊天室ID过滤
    if let Some(ref chat_room_id_str) = query.chat_room_id {
        let chat_room_id = Uuid::parse_str(chat_room_id_str).map_err(|_|
            AppError::BadRequest("无效的聊天室ID格式".to_string())
        )?;
        filter.chat_room_id = Some(chat_room_id);
        has_filter = true;
    }

    // 时间范围过滤
    if let Some(ref start_time_str) = query.start_time {
        let start_time = DateTime::parse_from_rfc3339(start_time_str)
            .map_err(|_|
                AppError::BadRequest("无效的开始时间格式，请使用ISO 8601格式".to_string())
            )?
            .with_timezone(&Utc);
        filter.start_time = Some(start_time);
        has_filter = true;
    }

    if let Some(ref end_time_str) = query.end_time {
        let end_time = DateTime::parse_from_rfc3339(end_time_str)
            .map_err(|_|
                AppError::BadRequest("无效的结束时间格式，请使用ISO 8601格式".to_string())
            )?
            .with_timezone(&Utc);
        filter.end_time = Some(end_time);
        has_filter = true;
    }

    Ok(if has_filter { Some(filter) } else { None })
}

/// 构建聊天室消息过滤器
fn build_chat_room_message_filter(query: &ChatRoomMessageQuery) -> Result<Option<MessageFilter>> {
    let mut filter = MessageFilter::default();
    let mut has_filter = false;

    // 关键词搜索
    if let Some(ref keyword) = query.keyword {
        if !keyword.trim().is_empty() {
            filter.keyword = Some(keyword.clone());
            has_filter = true;
        }
    }

    // 消息类型过滤
    if let Some(ref message_type_str) = query.message_type {
        let message_type = parse_message_type(message_type_str)?;
        filter.message_type = Some(message_type);
        has_filter = true;
    }

    // 消息状态过滤
    if let Some(ref status_str) = query.status {
        let status = parse_message_status(status_str)?;
        filter.status = Some(status);
        has_filter = true;
    }

    // 发送者ID过滤
    if let Some(ref sender_id_str) = query.sender_id {
        let sender_id = Uuid::parse_str(sender_id_str).map_err(|_|
            AppError::BadRequest("无效的发送者ID格式".to_string())
        )?;
        filter.sender_id = Some(sender_id);
        has_filter = true;
    }

    // 时间范围过滤
    if let Some(ref start_time_str) = query.start_time {
        let start_time = DateTime::parse_from_rfc3339(start_time_str)
            .map_err(|_|
                AppError::BadRequest("无效的开始时间格式，请使用ISO 8601格式".to_string())
            )?
            .with_timezone(&Utc);
        filter.start_time = Some(start_time);
        has_filter = true;
    }

    if let Some(ref end_time_str) = query.end_time {
        let end_time = DateTime::parse_from_rfc3339(end_time_str)
            .map_err(|_|
                AppError::BadRequest("无效的结束时间格式，请使用ISO 8601格式".to_string())
            )?
            .with_timezone(&Utc);
        filter.end_time = Some(end_time);
        has_filter = true;
    }

    Ok(if has_filter { Some(filter) } else { None })
}

/// 解析消息类型字符串
fn parse_message_type(type_str: &str) -> Result<MessageType> {
    match type_str.to_lowercase().as_str() {
        "text" => Ok(MessageType::Text),
        "image" => Ok(MessageType::Image),
        "file" => Ok(MessageType::File),
        "system" => Ok(MessageType::System),
        "voice" => Ok(MessageType::Voice),
        "video" => Ok(MessageType::Video),
        _ =>
            Err(
                AppError::BadRequest(
                    format!("无效的消息类型: {}。支持的类型: text, image, file, system, voice, video", type_str)
                )
            ),
    }
}

/// 解析消息状态字符串
fn parse_message_status(status_str: &str) -> Result<MessageStatus> {
    match status_str.to_lowercase().as_str() {
        "sent" => Ok(MessageStatus::Sent),
        "delivered" => Ok(MessageStatus::Delivered),
        "read" => Ok(MessageStatus::Read),
        "deleted" => Ok(MessageStatus::Deleted),
        "edited" => Ok(MessageStatus::Edited),
        _ =>
            Err(
                AppError::BadRequest(
                    format!("无效的消息状态: {}。支持的状态: sent, delivered, read, deleted, edited", status_str)
                )
            ),
    }
}

/// 转换分页结果为响应格式
fn convert_to_paginated_response(result: MessagePaginationResult) -> PaginatedMessageResponse {
    let messages = result.messages
        .into_iter()
        .map(|msg| MessageResponse {
            id: msg.id.to_string(),
            content: msg.content,
            message_type: message_type_to_string(&msg.message_type),
            status: message_status_to_string(&msg.status),
            sender_id: msg.sender_id.to_string(),
            chat_room_id: msg.chat_room_id.to_string(),
            reply_to_id: msg.reply_to_id.map(|id| id.to_string()),
            metadata: msg.metadata,
            priority: msg.priority,
            is_pinned: msg.is_pinned,
            expires_at: msg.expires_at,
            created_at: msg.created_at,
            updated_at: msg.updated_at,
        })
        .collect();

    let pagination = PaginationInfo {
        total_count: result.total_count,
        total_pages: result.total_pages,
        current_page: result.current_page,
        page_size: result.page_size,
        has_next: result.has_next,
        has_prev: result.has_prev,
    };

    PaginatedMessageResponse {
        messages,
        pagination,
    }
}

/// 消息类型转换为字符串
fn message_type_to_string(message_type: &MessageType) -> String {
    match message_type {
        MessageType::Text => "text".to_string(),
        MessageType::Image => "image".to_string(),
        MessageType::File => "file".to_string(),
        MessageType::System => "system".to_string(),
        MessageType::Voice => "voice".to_string(),
        MessageType::Video => "video".to_string(),
    }
}

/// 消息状态转换为字符串
fn message_status_to_string(status: &MessageStatus) -> String {
    match status {
        MessageStatus::Sent => "sent".to_string(),
        MessageStatus::Delivered => "delivered".to_string(),
        MessageStatus::Read => "read".to_string(),
        MessageStatus::Deleted => "deleted".to_string(),
        MessageStatus::Edited => "edited".to_string(),
    }
}
