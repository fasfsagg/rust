// src/app/middleware/auth_middleware.rs

use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    http::{request::Parts, HeaderMap}, // Removed StatusCode
    // Removed IntoResponse, Json,
};
use jsonwebtoken::{decode, Validation, Algorithm, DecodingKey};
// Removed serde_json::json

use crate::{
    app::model::user::Claims, // Your Claims struct
    app::AppState,            // Use the re-exported AppState from app/mod.rs
    // config::AppConfig,     // AppConfig is accessed via AppState
    error::AppError,          // Your AppError enum
};

const BEARER_PREFIX: &str = "Bearer ";

#[async_trait]
impl FromRequestParts<AppState> for Claims {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        // Extract the Authorization header
        let auth_header = parts.headers.get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok());

        let token = match auth_header {
            Some(header_value) if header_value.starts_with(BEARER_PREFIX) => {
                header_value.trim_start_matches(BEARER_PREFIX).to_owned()
            }
            _ => {
                return Err(AppError::Unauthorized("Missing or malformed Bearer token".to_string()));
            }
        };

        // Decode the token
        // The JWT secret should be accessed via AppState -> AppConfig
        let decoding_key = DecodingKey::from_secret(state.config.jwt_secret.as_ref());
        let validation = Validation::new(Algorithm::HS512); // Use the same algorithm used for encoding

        match decode::<Claims>(&token, &decoding_key, &validation) {
            Ok(token_data) => Ok(token_data.claims),
            Err(err) => {
                // Log the detailed error for debugging on the server
                eprintln!("JWT decoding error: {:?}", err);
                // Return a generic unauthorized error to the client
                Err(AppError::Unauthorized(format!("Invalid token: {}", err.kind().to_string())))
            }
        }
    }
}

// Helper function to construct an AppError for unauthorized access.
// This might be useful if you need to return a specific JSON structure for auth errors
// directly from the middleware, but with FromRequestParts, returning AppError is cleaner.
#[allow(dead_code)]
fn unauthorized_error(message: String) -> AppError {
    AppError::Unauthorized(message)
}

// The Layer-based middleware approach is commented out because FromRequestParts is used.
// Removing it as per cleanup subtask.

[end of src/app/middleware/auth_middleware.rs]
