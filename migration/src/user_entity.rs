//! `user_entity.rs`
//!
//! 这个文件专门定义了与 `users` 表相关联的 SeaORM 实体。
//! 它被设计为 `migration` crate 的一部分，以便迁移脚本可以直接访问它，
//! 同时也可以被主应用 `axum-tutorial` 导入，实现代码的复用和单向依赖。

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub username: String,
    pub password_hash: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::task_entity::Entity")]
    Task,
}

impl Related<super::task_entity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Task.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
