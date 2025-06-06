// src/app/service/task_service.rs

use sea_orm::{DatabaseConnection, EntityTrait, ModelTrait, Set, ActiveModelTrait, IntoActiveModel}; // Removed QueryFilter, ColumnTrait
use chrono::Utc;

use crate::{
    app::model::{
        task, // SeaORM task entity (model, active_model, entity, column)
        CreateTaskPayload, // Still used by controller, service will adapt
        UpdateTaskPayload, // Still used by controller, service will adapt
    },
    error::{AppError, Result as AppResult}, // Use AppResult alias
};

/// Service: Creates a new task.
pub async fn create_task(
    db: &DatabaseConnection,
    payload: CreateTaskPayload,
) -> AppResult<task::Model> {
    println!("SERVICE: Processing create task request...");

    // The ActiveModelBehavior's new() method automatically sets created_at and updated_at.
    // Other fields are set from the payload.
    let mut active_task = task::ActiveModel::new();
    active_task.title = Set(payload.title);
    active_task.description = Set(payload.description);
    active_task.completed = Set(payload.completed);
    // created_at and updated_at are set by ActiveModelBehavior

    let saved_task = active_task.insert(db).await.map_err(AppError::DatabaseError)?;

    println!("SERVICE: Task created successfully (ID: {})", saved_task.id);
    Ok(saved_task)
}

/// Service: Retrieves all tasks.
pub async fn get_all_tasks(db: &DatabaseConnection) -> AppResult<Vec<task::Model>> {
    println!("SERVICE: Processing get all tasks request...");
    task::Entity::find()
        .all(db)
        .await
        .map_err(AppError::DatabaseError)
}

/// Service: Retrieves a single task by its ID.
pub async fn get_task_by_id(db: &DatabaseConnection, id: i32) -> AppResult<task::Model> {
    println!("SERVICE: Processing get task by ID request for '{}'", id);
    let task = task::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AppError::DatabaseError)?;

    match task {
        Some(t) => Ok(t),
        None => Err(AppError::TaskNotFound(id)),
    }
}

/// Service: Updates an existing task.
pub async fn update_task(
    db: &DatabaseConnection,
    id: i32,
    payload: UpdateTaskPayload,
) -> AppResult<task::Model> {
    println!("SERVICE: Processing update task request for ID: {}", id);

    // Fetch the existing task
    let task_to_update = task::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or_else(|| AppError::TaskNotFound(id))?;

    // Convert the model to an active model for updates
    let mut active_task = task_to_update.into_active_model();

    // Apply updates from payload.
    // If a field is Some in payload, it means client wants to change it.
    // If a field is None in payload (for Option fields like description or completed),
    // it means client wants to set it to NULL (for description) or did not provide (for completed).
    // The `Set` operation handles this: `Set(Some(value))` updates, `Set(None)` sets to NULL.
    // For non-Option fields like title, `payload.title` being `None` means "don't update".

    if let Some(title) = payload.title {
        active_task.title = Set(title);
    }

    // `payload.description` is Option<String>. `Set` also takes Option<String>.
    // If `payload.description` is `None` (from JSON `null` or omitted field), this sets DB to NULL.
    // If `payload.description` is `Some(value)`, this sets DB to that value.
    // This is the desired behavior for an optional field that can be cleared.
    active_task.description = Set(payload.description);

    if let Some(completed) = payload.completed {
        active_task.completed = Set(completed);
    }

    // `updated_at` is handled by ActiveModelBehavior's `before_save`.
    // The `update` method will only send an UPDATE SQL query if an actual field value has changed.
    // If only `updated_at` is changed by `ActiveModelBehavior` but no other fields are `Set` differently,
    // it will still perform an update.
    let updated_task = active_task.update(db).await.map_err(AppError::DatabaseError)?;
    println!("SERVICE: Task updated successfully for ID: {}", id);
    Ok(updated_task)
}


/// Service: Deletes a task by its ID.
pub async fn delete_task(db: &DatabaseConnection, id: i32) -> AppResult<task::Model> {
    println!("SERVICE: Processing delete task request for ID: {}", id);

    // First, find the task to be deleted so we can return it.
    let task_to_delete = task::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or_else(|| AppError::TaskNotFound(id))?;

    // Perform the deletion
    let delete_result = task::Entity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(AppError::DatabaseError)?;

    if delete_result.rows_affected == 0 {
        // This case should ideally be caught by the find_by_id above,
        // but it's good for robustness.
        return Err(AppError::TaskNotFound(id));
    }

    println!("SERVICE: Task deleted successfully for ID: {}", id);
    Ok(task_to_delete) // Return the model of the task that was deleted
}
