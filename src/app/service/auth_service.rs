// src/app/service/auth_service.rs
use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2
};
use sea_orm::*;
use jsonwebtoken::{encode, Header, EncodingKey, Algorithm};
use chrono::{Utc, Duration};

use crate::{
    app::model::user::{self, RegisterUserPayload, LoginUserPayload, Claims}, // Assuming user::Model for User entity
    config::AppConfig,
    error::AppError,
};

/// `AuthService` provides authentication-related business logic.
///
/// This includes user registration and login functionality, handling password hashing,
/// JWT generation, and interaction with the user data model via SeaORM.
pub struct AuthService;

impl AuthService {
    /// Registers a new user.
    ///
    /// Hashes the provided password and stores the new user in the database.
    /// Returns the created user model if successful, or an `AppError` otherwise.
    /// Errors can include username already existing or database issues.
    pub async fn register_user(
        db: &DatabaseConnection,
        payload: RegisterUserPayload,
    ) -> Result<user::Model, AppError> {
        // Check if username already exists
        if user::Entity::find()
            .filter(user::Column::Username.eq(&payload.username))
            .one(db)
            .await?
            .is_some()
        {
            return Err(AppError::UsernameAlreadyExists(payload.username));
        }

        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(payload.password.as_bytes(), &salt)
            .map_err(|e| AppError::PasswordHashingError(e.to_string()))?
            .to_string();

        let new_user = user::ActiveModel {
            username: Set(payload.username),
            password_hash: Set(password_hash),
            ..Default::default() // id will be auto-generated
        };

        let user_model = new_user.insert(db).await.map_err(AppError::DatabaseError)?;
        Ok(user_model)
    }

    /// Logs in an existing user.
    ///
    /// Verifies the username and password against the stored hash.
    /// If successful, generates and returns a JWT.
    /// Returns an `AppError` for cases like user not found, invalid password, or JWT creation failure.
    pub async fn login_user(
        db: &DatabaseConnection,
        app_config: &AppConfig,
        payload: LoginUserPayload,
    ) -> Result<String, AppError> {
        let user = user::Entity::find()
            .filter(user::Column::Username.eq(&payload.username))
            .one(db)
            .await?
            .ok_or_else(|| AppError::UserNotFound(payload.username.clone()))?;

        let parsed_hash = PasswordHash::new(&user.password_hash)
            .map_err(|e| AppError::PasswordHashingError(e.to_string()))?;

        Argon2::default()
            .verify_password(payload.password.as_bytes(), &parsed_hash)
            .map_err(|_| AppError::InvalidPassword)?;

        let now = Utc::now();
        let iat = now.timestamp() as usize;
        let exp = (now + Duration::seconds(app_config.jwt_expiration_seconds)).timestamp() as usize;

        let claims = Claims {
            sub: user.id.to_string(), // Assuming user.id is i32
            company: "YourCompanyName".to_string(), // Example, make this configurable if needed
            username: user.username.clone(),
            iat,
            exp,
        };

        encode(
            &Header::new(Algorithm::HS512),
            &claims,
            &EncodingKey::from_secret(app_config.jwt_secret.as_ref()),
        )
        .map_err(|e| AppError::JwtCreationError(e.to_string()))
    }
}
