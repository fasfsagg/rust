//! `task.rs`
//!
//! 在重构后，这个文件的职责变得更加清晰：
//! 1.  **定义 API 数据契约**: `CreateTaskPayload` 和 `UpdateTaskPayload` 这些结构体，
//!     专门用于序列化和反序列化 Controller 层的 HTTP 请求体 (JSON)。
//!     它们是应用对外的接口，与数据库的内部实现解耦。
//! 2.  **定义 API 数据传输对象 (DTO)**: `Task` 结构体作为视图模型，
//!     用于将数据库实体安全地序列化为 JSON 返回给客户端。
//! 3.  **隔离数据库实体**: 不再直接暴露 `migration` crate 中的实体，
//!     而是通过 `From` trait 进行转换，实现内外模型的隔离。

use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };
use uuid::Uuid;

// 将数据库实体模型重命名导入，以避免名称冲突。
// 这是内部表示，不应该直接暴露给 API。
use migration::task_entity as db_model;

// --- API 响应 DTO (Data Transfer Object) ---

/// 任务的数据传输对象 (DTO)。
///
/// 这是返回给 API 调用方的视图模型，包含了需要序列化为 JSON 的所有字段。
/// `#[derive(Debug, Serialize)]` 是解决 500 错误的关键：
///   - `Serialize`: 告诉 `serde` 如何将这个结构体转换为 JSON。
///   - `Debug`: 方便在日志中打印调试。
#[derive(Debug, Serialize)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub completed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 实现 `From` trait，用于将数据库实体 `db_model::Model` 转换为 API DTO `Task`。
///
/// 这是一种优雅的类型转换方式，符合 Rust 的惯例。
/// 拥有这个实现后，我们可以在代码中简单地使用 `.into()` 来完成转换。
impl From<db_model::Model> for Task {
    fn from(model: db_model::Model) -> Self {
        Self {
            id: model.id,
            title: model.title,
            description: model.description,
            completed: model.completed,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

// --- API 请求载荷结构体 (Request Payloads) ---
// 这些结构体定义了 API 的【外部契约】。

/// 创建任务的请求载荷。
#[derive(Deserialize)]
pub struct CreateTaskPayload {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub completed: bool,
}

/// 更新任务的请求载aho荷。
#[derive(Deserialize)]
pub struct UpdateTaskPayload {
    pub title: Option<String>,
    #[serde(default, with = "double_option")]
    pub description: Option<Option<String>>,
    pub completed: Option<bool>,
}

/// 自定义 serde 辅助模块，用于处理双层 Option，以区分 "未提供" 和 "null"。
mod double_option {
    use serde::{ Deserialize, Deserializer };

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
        where T: Deserialize<'de>, D: Deserializer<'de>
    {
        Option::<Option<T>>::deserialize(deserializer).map(|opt| opt.flatten())
    }
}
