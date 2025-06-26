pub use sea_orm_migration::prelude::*;

mod m20250610_035426_create_task_table;
mod m20250615_075512_create_users_table;
mod m20250615_081240_add_user_id_to_tasks;
// 聊天室相关迁移
mod m20250624_120000_create_chat_rooms_table;
mod m20250624_120001_create_messages_table;
mod m20250624_120002_create_user_sessions_table;
pub mod task_entity;
pub mod user_entity;
// 聊天室相关实体模块
pub mod chat_room_entity;
pub mod message_entity;
pub mod user_session_entity;

// 测试模块
#[cfg(test)]
mod tests;

pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250610_035426_create_task_table::Migration),
            Box::new(m20250615_075512_create_users_table::Migration),
            Box::new(m20250615_081240_add_user_id_to_tasks::Migration),
            // 聊天室相关迁移
            Box::new(m20250624_120000_create_chat_rooms_table::Migration),
            Box::new(m20250624_120001_create_messages_table::Migration),
            Box::new(m20250624_120002_create_user_sessions_table::Migration)
        ]
    }
}
