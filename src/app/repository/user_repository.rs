//! `user_repository.rs`
//!
//! 这个模块负责所有与用户（User）相关的数据库操作。
//! 它使用了 SeaORM 作为 ORM 工具，将底层的数据库查询逻辑封装起来，
//! 为服务层（Service Layer）提供了一套清晰、独立的异步接口。
//!
//! ## 设计原则 (Design Principles)
//! - **单一职责**: `UserRepository` 的唯一职责是处理用户数据的持久化。
//! - **解耦**: 将数据访问逻辑与业务逻辑（在 `auth_service` 中）分离。
//! - **异步**: 所有数据库操作都是异步的，以充分利用 Tokio 的非阻塞 I/O。
//! - **错误处理**: 函数返回 `Result<T, DbErr>`，将数据库错误传递给上层进行处理。
//!
//! ## 函数说明
//! - `find_by_username`: 根据用户名查询用户（用于登录验证）。
//! - `create`: 创建一个新用户（用于注册）。

use async_trait::async_trait;
use migration::user_entity::{ActiveModel, Entity, Model};
use sea_orm::{
    prelude::Uuid,
    ActiveModelTrait,
    ColumnTrait,
    DatabaseConnection,
    DbErr,
    EntityTrait,
    QueryFilter,
};

/// 用户仓库结构体。
///
/// 它持有一个数据库连接池的克隆 (`DatabaseConnection`)，所有数据库操作都通过它进行。
#[derive(Debug, Clone)]
pub struct UserRepository {
    db: DatabaseConnection,
}

impl UserRepository {
    /// 创建一个新的 UserRepository 实例。
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

/// 用户仓库的抽象 Trait。
///
/// 定义了用户仓库必须实现的所有功能协定。
/// 使用 `#[async_trait]` 宏来支持在 trait 中定义异步函数。
/// `Send + Sync` 约束是让它能在多线程环境下安全地共享。
#[async_trait]
pub trait UserRepositoryContract: Send + Sync {
    /// 根据用户名查询用户。
    /// 这个方法主要用于登录验证，需要获取用户的密码哈希进行比较。
    async fn find_by_username(&self, username: &str) -> Result<Option<Model>, DbErr>;

    /// 创建一个新用户。
    /// 这个方法用于用户注册，将新用户信息保存到数据库。
    async fn create(&self, data: ActiveModel) -> Result<Model, DbErr>;
}

#[async_trait]
impl UserRepositoryContract for UserRepository {
    /// 根据用户名查询用户。
    ///
    /// # 参数
    /// - `username`: 要查找的用户名。
    ///
    /// # 返回
    /// 成功时返回 `Option<Model>`，如果找到则为 `Some(user)`，否则为 `None`。
    /// 失败时返回 `DbErr`。
    async fn find_by_username(&self, username: &str) -> Result<Option<Model>, DbErr> {
        Entity::find()
            .filter(migration::user_entity::Column::Username.eq(username))
            .one(&self.db)
            .await
    }

    /// 创建一个新用户。
    ///
    /// # 参数
    /// - `data`: 包含新用户数据的 `ActiveModel`。这是由服务层构建的。
    ///
    /// # 返回
    /// 成功时返回创建的用户模型 `Model`，失败时返回 `DbErr`。
    async fn create(&self, mut data: ActiveModel) -> Result<Model, DbErr> {
        // 确保设置了UUID，如果没有则自动生成
        if data.id.is_not_set() {
            data.id = sea_orm::Set(Uuid::new_v4());
        }
        data.insert(&self.db).await
    }
}
