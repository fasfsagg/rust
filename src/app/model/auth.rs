//! src/app/model/auth.rs
//! This module contains the Data Transfer Objects (DTOs) for authentication-related requests.
use serde::Deserialize;
use validator::Validate;

/// Represents the payload for a user registration request.
/// It includes validation rules for each field.
#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    /// The username for the new account.
    /// It must be at least 3 characters long.
    #[validate(length(min = 3, message = "Username must be at least 3 characters long."))]
    pub username: String,

    /// The password for the new account.
    /// It must be at least 8 characters long.
    #[validate(length(min = 8, message = "Password must be at least 8 characters long."))]
    pub password: String,

    /// The confirmation of the password.
    /// It must match the `password` field.
    #[validate(must_match(other = "password", message = "Passwords must match."))]
    #[serde(rename = "confirmPassword")]
    pub confirm_password: String,
}

/// Represents the payload for a user login request.
/// It includes basic validation to ensure fields are not empty.
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    /// The username for login.
    /// Cannot be empty.
    #[validate(length(min = 1, message = "Username cannot be empty."))]
    pub username: String,

    /// The password for login.
    /// Cannot be empty.
    #[validate(length(min = 1, message = "Password cannot be empty."))]
    pub password: String,
}
