//! `task_repository.rs`
//!
//! 这个模块负责所有与任务（Task）相关的数据库操作。
//! 它使用了 SeaORM 作为 ORM 工具，将底层的数据库查询逻辑封装起来，
//! 为服务层（Service Layer）提供了一套清晰、独立的异步接口。
//!
//! ## 设计原则 (Design Principles)
//! - **单一职责**: `TaskRepository` 的唯一职责是处理任务数据的持久化。
//! - **解耦**: 将数据访问逻辑与业务逻辑（在 `task_service` 中）分离。
//! - **异步**: 所有数据库操作都是异步的，以充分利用 Tokio 的非阻塞 I/O。
//! - **错误处理**: 函数返回 `Result<T, DbErr>`，将数据库错误传递给上层进行处理。
//!
//! ## 函数说明
//! - `find_all`: 查询所有任务。
//! - `find_by_id`: 根据 UUID 查询单个任务。
//! - `create`: 创建一个新任务。
//! - `update`: 更新一个现有任务。
//! - `delete`: 删除一个任务。

use migration::task_entity::{ ActiveModel, Entity, Model };
use sea_orm::{
    prelude::Uuid,
    ActiveModelTrait,
    DatabaseConnection,
    DbErr,
    DeleteResult,
    EntityTrait,
};

/// 任务仓库结构体。
///
/// 这是一个零大小的结构体（Zero-Sized Type），主要用作关联函数的命名空间。
/// 我们不需要存储任何状态，因为 `DatabaseConnection` 是通过函数参数传入的。
#[derive(Debug, Default)]
pub struct TaskRepository;

impl TaskRepository {
    /// 查询所有任务。
    ///
    /// # 参数
    /// - `db`: 数据库连接的引用。
    ///
    /// # 返回
    /// 成功时返回包含所有任务模型的 `Vec<Model>`，失败时返回 `DbErr`。
    pub async fn find_all(db: &DatabaseConnection) -> Result<Vec<Model>, DbErr> {
        Entity::find().all(db).await
    }

    /// 根据 ID 查询单个任务。
    ///
    /// # 参数
    /// - `db`: 数据库连接的引用。
    /// - `id`: 要查找的任务的 UUID。
    ///
    /// # 返回
    /// 成功时返回 `Option<Model>`，如果找到则为 `Some(task)`，否则为 `None`。
    /// 失败时返回 `DbErr`。
    pub async fn find_by_id(db: &DatabaseConnection, id: Uuid) -> Result<Option<Model>, DbErr> {
        Entity::find_by_id(id).one(db).await
    }

    /// 创建一个新任务。
    ///
    /// # 参数
    /// - `db`: 数据库连接的引用。
    /// - `data`: 包含新任务数据的 `ActiveModel`。这是由服务层构建的。
    ///
    /// # 返回
    /// 成功时返回创建的任务模型 `Model`，失败时返回 `DbErr`。
    pub async fn create(db: &DatabaseConnection, mut data: ActiveModel) -> Result<Model, DbErr> {
        // 确保设置了UUID，如果没有则自动生成
        if data.id.is_not_set() {
            data.id = sea_orm::Set(Uuid::new_v4());
        }
        data.insert(db).await
    }

    /// 更新一个现有任务。
    ///
    /// 注意：此函数期望 `data` 是一个包含了主键的 `ActiveModel`。
    /// 服务层负责获取现有模型，并将其转换为一个用于更新的 `ActiveModel`。
    ///
    /// # 参数
    /// - `db`: 数据库连接的引用。
    /// - `data`: 包含更新后任务数据的 `ActiveModel`。
    ///
    /// # 返回
    /// 成功时返回更新后的任务模型 `Model`，失败时返回 `DbErr`。
    pub async fn update(db: &DatabaseConnection, data: ActiveModel) -> Result<Model, DbErr> {
        data.update(db).await
    }

    /// 根据 ID 删除一个任务。
    ///
    /// 这是一个高效的操作，它直接在数据库中执行删除命令，而无需先获取任务模型。
    ///
    /// # 参数
    /// - `db`: 数据库连接的引用。
    /// - `id`: 要删除的任务的 UUID。
    ///
    /// # 返回
    /// 成功时返回 `DeleteResult`，其中包含了受影响的行数。服务层可以检查 `rows_affected`
    /// 是否为 1 来确认删除是否成功。失败时返回 `DbErr`。
    pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<DeleteResult, DbErr> {
        Entity::delete_by_id(id).exec(db).await
    }
}
