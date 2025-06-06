use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    #[sea_orm(unique = true)]
    pub username: String,
    pub password_hash: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// --- Payloads for Authentication ---

#[derive(Debug, Deserialize)]
pub struct RegisterUserPayload {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginUserPayload {
    pub username: String,
    pub password: String,
}

// --- Responses for Authentication ---

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i32,
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
}

// --- JWT Claims ---
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (user_id)
    pub company: String, // Optional: company name or identifier
    pub username: String, // Optional: username
    pub exp: usize,  // Expiration time (timestamp)
    pub iat: usize,  // Issued at (timestamp)
}
