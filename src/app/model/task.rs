// src/app/model/task.rs

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// --- Task Entity Definition ---

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "tasks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub title: String,
    #[sea_orm(nullable)] // Explicitly tell SeaORM this can be NULL in the DB
    pub description: Option<String>,
    pub completed: bool,
    pub created_at: i64, // Unix timestamp (seconds)
    pub updated_at: i64, // Unix timestamp (seconds)
    // pub user_id: Option<i32>, // Example if linking to users, keep None for now
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // Example: If tasks were linked to users
    // #[sea_orm(
    //     belongs_to = "super::user::Entity", // Assuming user entity is in super::user
    //     from = "Column::UserId",
    //     to = "super::user::Column::Id"
    // )]
    // User,
}

// If linking to users, you might need this:
// impl Related<super::user::Entity> for Entity {
//     fn to() -> RelationDef {
//         Relation::User.def()
//     }
// }

impl ActiveModelBehavior for ActiveModel {
    /// Optionally override default behavior for active model
    /// For example, automatically update `updated_at` timestamps
    fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("SystemTime before UNIX EPOCH!")
            .as_secs() as i64;
        Self {
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
            ..ActiveModelTrait::default()
        }
    }

    fn before_save(mut self, insert: bool) -> Result<Self, DbErr> {
        if !insert { // Only update `updated_at` if it's not a new record
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("SystemTime before UNIX EPOCH!")
                .as_secs() as i64;
            self.updated_at = ActiveValue::Set(now);
        }
        Ok(self)
    }
}


// --- Request Payload Structs ---
// These are kept for controller layer to receive JSON data.
// Service layer will then map these to ActiveModel for SeaORM.

#[derive(Debug, Deserialize)]
pub struct CreateTaskPayload {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub completed: bool,
}

#[derive(Debug, Deserialize, Default)] // Added Default for easier updates
pub struct UpdateTaskPayload {
    pub title: Option<String>,
    #[serde(default, with = "double_option")]
    pub description: Option<String>,
    pub completed: Option<bool>,
}

// Module for handling Option<Option<T>> deserialization for PATCH-like updates
pub(crate) mod double_option {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        T: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        Option::<Option<T>>::deserialize(deserializer).map(|opt_opt| opt_opt.flatten())
    }

    pub fn serialize<S, T>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        match value {
            None => serializer.serialize_none(),
            Some(val) => serializer.serialize_some(val),
        }
    }
}
