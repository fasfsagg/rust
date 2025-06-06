// src/db.rs

use sea_orm::{Database, DatabaseConnection, DbErr, ConnectOptions, ConnectionTrait, Statement, Schema, DbBackend, ActiveModelTrait};
use std::env;
use std::time::Duration;

// Import entities
use crate::app::model::{user, task};


/// Establishes a connection to the database.
///
/// Reads the `DATABASE_URL` environment variable to determine the connection string.
/// It configures connection options such as timeout and SQL logging.
///
/// # Returns
/// * `Result<DatabaseConnection, DbErr>`: A connection pool on success, or a database error on failure.
pub async fn establish_connection() -> Result<DatabaseConnection, DbErr> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let mut opt = ConnectOptions::new(database_url);
    opt.max_connections(100)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(true)
        .sqlx_logging_level(log::LevelFilter::Info); // Setting Info level for SQLx logs

    Database::connect(opt).await
}

/// Runs database migrations using SeaORM's SchemaManager.
///
/// This function will create tables for defined entities if they do not already exist.
/// It currently handles the `user` and `task` entities.
///
/// In a more complex application, `sea-orm-cli` and dedicated migration files
/// (`MigratorTrait`) would be preferred for more robust schema evolution.
///
/// # Arguments
/// * `db`: A reference to the `DatabaseConnection`.
///
/// # Returns
/// * `Result<(), DbErr>`: Ok if schema creation was successful for all entities, or a database error.
pub async fn run_migrations(db: &DatabaseConnection) -> Result<(), DbErr> {
    println!("DB: Running migrations...");

    let schema_manager = Schema::new(db.get_database_backend());

    // Create 'users' table
    match db.execute(db.get_database_backend().build(&schema_manager.create_table_from_entity(user::Entity).if_not_exists())).await {
        Ok(_) => println!("DB: 'users' table created or already exists."),
        Err(e) => {
            eprintln!("DB: Error creating 'users' table: {}", e);
            return Err(e);
        }
    }

    // Create 'tasks' table
    match db.execute(db.get_database_backend().build(&schema_manager.create_table_from_entity(task::Entity).if_not_exists())).await {
        Ok(_) => println!("DB: 'tasks' table created or already exists."),
        Err(e) => {
            eprintln!("DB: Error creating 'tasks' table: {}", e);
            return Err(e);
        }
    }

    println!("DB: Migrations completed.");
    Ok(())
}
