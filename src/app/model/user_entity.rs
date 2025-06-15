//! `SeaORM` Entity, DTO, and related implementations for the `User` entity.

use sea_orm::entity::prelude::*;
use serde::{ Deserialize, Serialize };

/// Defines the `user` entity, which will be mapped to the `users` table in the database.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    /// The unique identifier for the user, using a UUID.
    #[sea_orm(primary_key, auto_increment = false, column_type = "Uuid")]
    pub id: Uuid,

    /// The user's chosen username. Must be unique.
    #[sea_orm(unique)]
    pub username: String,

    /// The user's hashed password. This should never be returned in an API response.
    #[serde(skip_serializing, skip_deserializing)]
    pub password_hash: String,
}

/// The `UserResponse` DTO (Data Transfer Object).
/// This is the version of a user that is safe to send back in an API response.
/// It notably excludes the `password_hash`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
}

/// Implements the conversion from a `user::Model` (the database entity)
/// to a `UserResponse` (the API DTO).
impl From<Model> for UserResponse {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            username: model.username,
        }
    }
}

/// Implements the conversion from a `migration::user_entity::Model` (the database entity)
/// to a `UserResponse` (the API DTO).
impl From<migration::user_entity::Model> for UserResponse {
    fn from(model: migration::user_entity::Model) -> Self {
        Self {
            id: model.id,
            username: model.username,
        }
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
