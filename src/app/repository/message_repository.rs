//! `message_repository.rs`
//!
//! 消息仓库模块，负责所有与聊天消息相关的数据库操作。
//! 实现了消息的存储、检索、分页查询等核心功能，为企业级聊天应用提供高性能的消息持久化服务。
//!
//! ## 设计原则
//! - **高并发支持**: 使用异步操作和优化的查询，支持百万级并发消息处理
//! - **分页优化**: 实现高效的分页查询，避免大数据集的性能问题
//! - **索引友好**: 查询设计考虑数据库索引，确保查询性能
//! - **类型安全**: 使用强类型参数，避免运行时错误
//!
//! ## 主要功能
//! - `create`: 创建新消息
//! - `find_by_chat_room`: 按聊天室查询消息（支持分页）
//! - `find_by_id`: 根据ID查询单条消息
//! - `update_status`: 更新消息状态
//! - `count_by_chat_room`: 统计聊天室消息数量

use async_trait::async_trait;
use migration::message_entity::{ ActiveModel, Column, Entity, Model, MessageStatus, MessageType };
use sea_orm::{
    prelude::Uuid,
    ActiveModelTrait,
    ColumnTrait,
    DatabaseConnection,
    DbErr,
    EntityTrait,
    Order,
    PaginatorTrait,
    QueryFilter,
    QueryOrder,
    Set,
};

/// 分页查询参数结构体
#[derive(Debug, Clone)]
pub struct MessagePaginationParams {
    /// 页码（从1开始）
    pub page: u64,
    /// 每页大小（建议范围：10-100）
    pub page_size: u64,
    /// 是否按时间倒序（最新消息在前）
    pub desc_order: bool,
}

impl Default for MessagePaginationParams {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 50,
            desc_order: true,
        }
    }
}

/// 分页查询结果结构体
#[derive(Debug, Clone)]
pub struct MessagePaginationResult {
    /// 消息列表
    pub messages: Vec<Model>,
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

/// 消息查询过滤器
#[derive(Debug, Clone, Default)]
pub struct MessageFilter {
    /// 消息类型过滤
    pub message_type: Option<MessageType>,
    /// 消息状态过滤
    pub status: Option<MessageStatus>,
    /// 发送者ID过滤
    pub sender_id: Option<Uuid>,
    /// 聊天室ID过滤（可选，用于跨聊天室搜索时限制范围）
    pub chat_room_id: Option<Uuid>,
    /// 开始时间过滤
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    /// 结束时间过滤
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    /// 关键词搜索（在消息内容中搜索）
    pub keyword: Option<String>,
}

/// 消息仓库结构体
#[derive(Debug, Clone)]
pub struct MessageRepository {
    db: DatabaseConnection,
}

impl MessageRepository {
    /// 创建新的MessageRepository实例
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// 获取数据库连接（用于测试）
    #[cfg(test)]
    pub fn db(&self) -> &DatabaseConnection {
        &self.db
    }
}

/// 消息仓库的抽象Trait
///
/// 定义了消息仓库必须实现的所有功能协定。
/// 使用async_trait宏以支持dyn compatibility，适用于依赖注入场景。
#[async_trait]
pub trait MessageRepositoryContract: Send + Sync {
    /// 创建新消息
    ///
    /// # 参数
    /// - `data`: 包含消息数据的ActiveModel
    ///
    /// # 返回
    /// 成功时返回创建的消息模型，失败时返回DbErr
    async fn create(&self, data: ActiveModel) -> Result<Model, DbErr>;

    /// 根据ID查询单条消息
    ///
    /// # 参数
    /// - `id`: 消息的唯一标识符
    ///
    /// # 返回
    /// 成功时返回Option<Model>，找到则为Some(message)，否则为None
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Model>, DbErr>;

    /// 按聊天室查询消息（支持分页和过滤）
    ///
    /// # 参数
    /// - `chat_room_id`: 聊天室ID
    /// - `params`: 分页参数
    /// - `filter`: 可选的过滤条件
    ///
    /// # 返回
    /// 成功时返回分页查询结果，失败时返回DbErr
    async fn find_by_chat_room(
        &self,
        chat_room_id: Uuid,
        params: MessagePaginationParams,
        filter: Option<MessageFilter>
    ) -> Result<MessagePaginationResult, DbErr>;

    /// 更新消息状态
    ///
    /// # 参数
    /// - `id`: 消息ID
    /// - `status`: 新的消息状态
    ///
    /// # 返回
    /// 成功时返回更新后的消息模型，失败时返回DbErr
    async fn update_status(&self, id: Uuid, status: MessageStatus) -> Result<Model, DbErr>;

    /// 统计聊天室消息数量
    ///
    /// # 参数
    /// - `chat_room_id`: 聊天室ID
    /// - `filter`: 可选的过滤条件
    ///
    /// # 返回
    /// 成功时返回消息数量，失败时返回DbErr
    async fn count_by_chat_room(
        &self,
        chat_room_id: Uuid,
        filter: Option<MessageFilter>
    ) -> Result<u64, DbErr>;

    /// 删除消息（软删除，更新状态为已删除）
    ///
    /// # 参数
    /// - `id`: 消息ID
    ///
    /// # 返回
    /// 成功时返回更新后的消息模型，失败时返回DbErr
    async fn soft_delete(&self, id: Uuid) -> Result<Model, DbErr>;

    /// 搜索消息（支持关键词搜索和高级过滤）
    ///
    /// # 参数
    /// - `keyword`: 搜索关键词（在消息内容中搜索）
    /// - `params`: 分页参数
    /// - `filter`: 可选的高级过滤条件
    ///
    /// # 返回
    /// 成功时返回分页搜索结果，失败时返回DbErr
    ///
    /// # 说明
    /// 此方法与find_by_chat_room的区别在于：
    /// - 不限制特定聊天室，可跨聊天室搜索
    /// - 专门针对关键词搜索优化
    /// - 支持更复杂的搜索逻辑
    async fn search_messages(
        &self,
        keyword: String,
        params: MessagePaginationParams,
        filter: Option<MessageFilter>
    ) -> Result<MessagePaginationResult, DbErr>;
}

#[async_trait]
impl MessageRepositoryContract for MessageRepository {
    async fn create(&self, mut data: ActiveModel) -> Result<Model, DbErr> {
        // 确保设置了UUID，如果没有则自动生成
        if data.id.is_not_set() {
            data.id = Set(Uuid::new_v4());
        }

        // 设置创建和更新时间
        let now = chrono::Utc::now();
        if data.created_at.is_not_set() {
            data.created_at = Set(now);
        }
        if data.updated_at.is_not_set() {
            data.updated_at = Set(now);
        }

        data.insert(&self.db).await
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Model>, DbErr> {
        Entity::find_by_id(id).one(&self.db).await
    }

    async fn find_by_chat_room(
        &self,
        chat_room_id: Uuid,
        params: MessagePaginationParams,
        filter: Option<MessageFilter>
    ) -> Result<MessagePaginationResult, DbErr> {
        // 构建基础查询
        let mut query = Entity::find().filter(Column::ChatRoomId.eq(chat_room_id));

        // 应用过滤条件
        if let Some(filter) = &filter {
            if let Some(message_type) = &filter.message_type {
                query = query.filter(Column::MessageType.eq(message_type.clone()));
            }
            if let Some(status) = &filter.status {
                query = query.filter(Column::Status.eq(status.clone()));
            }
            if let Some(sender_id) = filter.sender_id {
                query = query.filter(Column::SenderId.eq(sender_id));
            }
            if let Some(start_time) = filter.start_time {
                query = query.filter(Column::CreatedAt.gte(start_time));
            }
            if let Some(end_time) = filter.end_time {
                query = query.filter(Column::CreatedAt.lte(end_time));
            }
            // 关键词搜索：在消息内容中进行模糊匹配
            if let Some(keyword) = &filter.keyword {
                if !keyword.trim().is_empty() {
                    let search_pattern = format!("%{}%", keyword.trim());
                    query = query.filter(Column::Content.like(&search_pattern));
                }
            }
        }

        // 应用排序
        query = if params.desc_order {
            query.order_by(Column::CreatedAt, Order::Desc)
        } else {
            query.order_by(Column::CreatedAt, Order::Asc)
        };

        // 执行分页查询
        let paginator = query.paginate(&self.db, params.page_size);
        let total_pages = paginator.num_pages().await?;
        let total_count = paginator.num_items().await?;

        let messages = paginator.fetch_page(params.page - 1).await?; // SeaORM分页从0开始

        Ok(MessagePaginationResult {
            messages,
            total_count,
            total_pages,
            current_page: params.page,
            page_size: params.page_size,
            has_next: params.page < total_pages,
            has_prev: params.page > 1,
        })
    }

    async fn update_status(&self, id: Uuid, status: MessageStatus) -> Result<Model, DbErr> {
        let message = Entity::find_by_id(id)
            .one(&self.db).await?
            .ok_or_else(|| DbErr::RecordNotFound("Message not found".to_string()))?;

        let mut active_model: ActiveModel = message.into();
        active_model.status = Set(status);
        active_model.updated_at = Set(chrono::Utc::now());

        active_model.update(&self.db).await
    }

    async fn count_by_chat_room(
        &self,
        chat_room_id: Uuid,
        filter: Option<MessageFilter>
    ) -> Result<u64, DbErr> {
        let mut query = Entity::find().filter(Column::ChatRoomId.eq(chat_room_id));

        // 应用过滤条件
        if let Some(filter) = filter {
            if let Some(message_type) = filter.message_type {
                query = query.filter(Column::MessageType.eq(message_type));
            }
            if let Some(status) = filter.status {
                query = query.filter(Column::Status.eq(status));
            }
            if let Some(sender_id) = filter.sender_id {
                query = query.filter(Column::SenderId.eq(sender_id));
            }
            if let Some(start_time) = filter.start_time {
                query = query.filter(Column::CreatedAt.gte(start_time));
            }
            if let Some(end_time) = filter.end_time {
                query = query.filter(Column::CreatedAt.lte(end_time));
            }
            // 关键词搜索：在消息内容中进行模糊匹配
            if let Some(keyword) = filter.keyword {
                if !keyword.trim().is_empty() {
                    let search_pattern = format!("%{}%", keyword.trim());
                    query = query.filter(Column::Content.like(&search_pattern));
                }
            }
        }

        query.count(&self.db).await
    }

    async fn soft_delete(&self, id: Uuid) -> Result<Model, DbErr> {
        self.update_status(id, MessageStatus::Deleted).await
    }

    async fn search_messages(
        &self,
        keyword: String,
        params: MessagePaginationParams,
        filter: Option<MessageFilter>
    ) -> Result<MessagePaginationResult, DbErr> {
        // 验证关键词不为空
        if keyword.trim().is_empty() {
            return Err(DbErr::Custom("搜索关键词不能为空".to_string()));
        }

        // 构建基础查询，包含关键词搜索
        let search_pattern = format!("%{}%", keyword.trim());
        let mut query = Entity::find().filter(Column::Content.like(&search_pattern));

        // 应用额外的过滤条件
        if let Some(filter) = &filter {
            if let Some(message_type) = &filter.message_type {
                query = query.filter(Column::MessageType.eq(message_type.clone()));
            }
            if let Some(status) = &filter.status {
                query = query.filter(Column::Status.eq(status.clone()));
            }
            if let Some(sender_id) = filter.sender_id {
                query = query.filter(Column::SenderId.eq(sender_id));
            }
            if let Some(start_time) = filter.start_time {
                query = query.filter(Column::CreatedAt.gte(start_time));
            }
            if let Some(end_time) = filter.end_time {
                query = query.filter(Column::CreatedAt.lte(end_time));
            }
            // 如果过滤器中指定了聊天室ID，则限制搜索范围到特定聊天室
            if let Some(chat_room_id) = filter.chat_room_id {
                query = query.filter(Column::ChatRoomId.eq(chat_room_id));
            }
        }

        // 应用排序
        query = if params.desc_order {
            query.order_by(Column::CreatedAt, Order::Desc)
        } else {
            query.order_by(Column::CreatedAt, Order::Asc)
        };

        // 执行分页查询
        let paginator = query.paginate(&self.db, params.page_size);
        let total_pages = paginator.num_pages().await?;
        let total_count = paginator.num_items().await?;

        let messages = paginator.fetch_page(params.page - 1).await?; // SeaORM分页从0开始

        Ok(MessagePaginationResult {
            messages,
            total_count,
            total_pages,
            current_page: params.page,
            page_size: params.page_size,
            has_next: params.page < total_pages,
            has_prev: params.page > 1,
        })
    }
}
