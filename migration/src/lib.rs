pub use sea_orm_migration::prelude::*;

mod m20250610_035426_create_task_table;
pub mod task_entity;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20250610_035426_create_task_table::Migration)]
    }
}
