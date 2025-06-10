//! `task_entity.rs`
//!
//! 这个文件专门定义了与 `tasks` 表相关联的 SeaORM 实体。
//! 它被设计为 `migration` crate 的一部分，以便迁移脚本可以直接访问它，
//! 同时也可以被主应用 `axum-tutorial` 导入，实现代码的复用和单向依赖。

use sea_orm::entity::prelude::*;
use serde::{ Deserialize, Serialize };

/// `Model` 结构体代表了 `tasks` 表中的一行数据。
///
/// `#[derive(DeriveEntityModel)]` 是 SeaORM 的核心魔法。
/// 它会读取这个结构体，并自动生成：
/// - `Entity` 结构体
/// - `Column` 枚举
/// - `PrimaryKey` 枚举
/// - `Relation` 枚举
/// - 以及所有相关的 trait 实现 (`EntityTrait`, `Iden`, `IdenStatic` 等)
///
/// 我们不再需要手动编写这些样板代码。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "tasks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid, // 主键使用 Uuid 类型

    pub title: String,

    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,

    pub completed: bool,

    // 对于自动时间戳，我们可以在 ActiveModelBehavior 中处理，
    // 但为了简化，这里暂时移除 before_save 的逻辑，
    // 让数据库自己通过 `DEFAULT CURRENT_TIMESTAMP` 来处理。
    // 这要求迁移脚本中为这两列设置了默认值。
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

/// `Relation` 枚举，定义表之间的关系。
/// SeaORM 的宏要求这个枚举存在，即使它是空的。
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

/// `ActiveModelBehavior` trait 允许我们挂接实体的生命周期事件。
/// 通过提供一个空的实现，我们满足了 `DeriveEntityModel` 的 trait 约束，
/// 同时将所有创建/更新时间戳的逻辑完全委托给数据库的默认值设置。
impl ActiveModelBehavior for ActiveModel {}
