// src/app/controller/auth_controller.rs
use axum::{extract::State, Json};
// Remove direct DatabaseConnection and AppConfig imports if they are no longer used separately
// use sea_orm::DatabaseConnection;
// use crate::config::AppConfig;

use crate::{
    app::service::auth_service,
    app::AppState, // Import the new AppState
    error::AppError,
    app::model::user::{RegisterUserPayload, LoginUserPayload, UserResponse, LoginResponse},
};

/// Handles user registration requests.
///
/// Expects a JSON payload with username and password (`RegisterUserPayload`).
/// Calls the `auth_service` to register the user.
/// Returns a JSON response with the new user's ID and username (`UserResponse`)
/// and a 200 OK status on success, or an `AppError` on failure.
pub async fn register_handler(
    State(app_state): State<AppState>, // Use AppState
    Json(payload): Json<RegisterUserPayload>,
) -> Result<Json<UserResponse>, AppError> {
    let user = auth_service::register_user(&app_state.db, payload).await?;
    Ok(Json(UserResponse {
        id: user.id,
        username: user.username,
    }))
}

/// Handles user login requests.
///
/// Expects a JSON payload with username and password (`LoginUserPayload`).
/// Calls the `auth_service` to verify credentials and generate a JWT.
/// Returns a JSON response with the JWT (`LoginResponse`) and a 200 OK status
/// on success, or an `AppError` on failure (e.g., invalid credentials).
pub async fn login_handler(
    State(app_state): State<AppState>, // Use AppState
    Json(payload): Json<LoginUserPayload>,
) -> Result<Json<LoginResponse>, AppError> {
    let token = auth_service::login_user(&app_state.db, &app_state.config, payload).await?;
    Ok(Json(LoginResponse { token }))
}
