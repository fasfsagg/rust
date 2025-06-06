// src/app/state.rs

use sea_orm::DatabaseConnection;
use crate::config::AppConfig;

/// Shared application state.
///
/// This struct holds all shared resources that need to be accessible by Axum handlers.
/// It includes the database connection pool and the application configuration.
///
/// It derives `Clone` because Axum requires state to be cloneable.
/// - `DatabaseConnection` is an `Arc` internally, so cloning it is cheap (increases ref count).
/// - `AppConfig` also needs to be `Clone`.
#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: AppConfig,
}
