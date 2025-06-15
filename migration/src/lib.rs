pub use sea_orm_migration::prelude::*;

mod m20250610_035426_create_task_table;
mod m20250615_075512_create_users_table;
mod m20250615_081240_add_user_id_to_tasks;
pub mod task_entity;
pub mod user_entity;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250610_035426_create_task_table::Migration),
            Box::new(m20250615_075512_create_users_table::Migration),
            Box::new(m20250615_081240_add_user_id_to_tasks::Migration),
        ]
    }
}
