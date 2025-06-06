# 模型层 (Model) 核心概念 (`src/app/model/`)

## 1. 职责与定位

模型层 (`src/app/model/`) 在应用程序架构中扮演着【数据蓝图】的角色。它的核心职责是：

- **定义数据结构与数据库模式**:
    - 通过 **SeaORM 实体 (Entities)** (例如 `user::Entity`, `task::Entity`) 定义应用程序核心业务实体及其在数据库中的表结构、列、主键和关系。这是数据库模式的直接代码表示。
    - 相关的 **模型 (Models)** (例如 `user::Model`, `task::Model`) 是从数据库检索数据时，代表具体数据行的 Rust 结构体。
    - **活动模型 (ActiveModels)** (例如 `user::ActiveModel`, `task::ActiveModel`) 用于构建插入和更新数据库的操作。
- **定义数据传输对象 (DTOs) / 请求载荷 (Payloads)**:
    - 为 API 请求和响应定义专门的结构体 (例如 `CreateTaskPayload`, `LoginUserPayload`)。这些结构体通常直接映射到 API 的 JSON 数据格式。
- **定义 JWT 声明 (Claims)**:
    - `Claims` 结构体定义了在 JSON Web Token 中编码的用户信息和元数据。

**关键点**: 模型层主要关注“数据应该是什么样子”，包括其在数据库中的持久化形态 (通过 SeaORM 实体) 和在应用各层之间以及通过 API 传输时的形态 (通过普通结构体如 Payloads 和 Claims)。它不应包含复杂的业务逻辑或直接的数据库访问方法 (这些由服务层和 SeaORM 本身处理)。

## 2. SeaORM 实体 (Entities) 与相关结构

SeaORM 是本项目的对象关系映射器 (ORM)，用于与 SQLite 数据库交互。

*   **实体 (`user::Entity`, `task::Entity`)**:
    *   在各自的模块文件 (`model/user.rs`, `model/task.rs`) 中定义。
    *   使用 `#[derive(DeriveEntityModel)]` 宏从对应的 `Model` 结构体生成。
    *   定义了数据库表的名称、列 (字段)、主键以及表之间的关系。
    *   是数据库表结构在 Rust 代码中的表示。

*   **模型 (`user::Model`, `task::Model`)**:
    *   与实体紧密关联的 Rust 结构体。
    *   代表从数据库中查询出来的一行完整数据。
    *   通常派生 `serde::{Serialize, Deserialize}` 以便用于 API 响应或需要序列化的场景。
    *   也派生 `Clone` 和 `Debug`。

*   **活动模型 (`user::ActiveModel`, `task::ActiveModel`)**:
    *   用于创建 (Insert) 和更新 (Update) 数据库记录。
    *   允许你只设置需要修改或插入的字段值 (使用 `Set(value)` 或 `ActiveValue::Set(value)`)，其他字段可以保持未设置状态 (`NotSet`)。
    *   实现了 `ActiveModelTrait`，提供了 `.insert(db)`, `.update(db)` 等方法。
    *   可以通过实现 `ActiveModelBehavior` trait 来定制其行为，例如自动填充 `created_at` 和 `updated_at` 时间戳。本项目中 `task::ActiveModel` 就利用了此特性。

## 3. JWT 声明 (`Claims`)

*   **定义位置**: `src/app/model/user.rs` 中的 `Claims` 结构体。
*   **用途**: 代表 JWT (JSON Web Token) 中的载荷部分，包含了关于用户的声明信息。
*   **字段**:
    *   `sub` (Subject): 通常存储用户的唯一标识符 (如用户 ID)。
    *   `username`: 存储用户名。
    *   `exp` (Expiration Time): JWT 的过期时间戳。
    *   `iat` (Issued At): JWT 的签发时间戳。
    *   其他自定义声明 (如项目中的 `company` 示例字段)。
*   **序列化**: 派生 `serde::{Serialize, Deserialize}`，用于将 `Claims` 编码到 JWT 字符串中，以及从 JWT 字符串中解码回 `Claims` 结构体。

## 4. 请求载荷 (Payloads / DTOs)

这些结构体专门用于 API 请求，定义了客户端应发送的数据格式。

*   **用途**: 作为数据传输对象 (DTO)，用于接收和验证来自客户端的 HTTP 请求体中的 JSON 数据。
*   **定义**:
    *   用户认证相关: `RegisterUserPayload`, `LoginUserPayload` (在 `model/user.rs`)。
    *   任务管理相关: `CreateTaskPayload`, `UpdateTaskPayload` (在 `model/task.rs`)。
*   **`#[derive(Deserialize)]`**: 使 `serde` 能够将传入的 JSON 解析为这些结构体的实例。
*   **字段设计**:
    *   创建操作的 Payload (如 `CreateTaskPayload`, `RegisterUserPayload`) 通常包含必需字段。
    *   更新操作的 Payload (如 `UpdateTaskPayload`) 的字段通常是 `Option<T>` 类型，允许客户端只发送需要修改的字段（部分更新，类似 PATCH 请求）。
    *   `#[serde(default)]`: 用于为在 JSON 中可能缺失的字段提供默认值 (例如，`Option` 字段默认为 `None`，`bool` 默认为 `false`)。

## 5. `double_option` 模块 (用于 `UpdateTaskPayload`)

*   **解决特定问题**: 在处理 `UpdateTaskPayload` 中的可选字段（如 `description: Option<String>`）时，需要区分三种更新意图：
    1.  **不修改字段**: JSON 请求中根本不包含该字段键。
    2.  **设置新值**: JSON 中包含该字段键，并有一个非 `null` 的值。
    3.  **清除字段值 (设为 NULL)**: JSON 中包含该字段键，但其值为 `null`。
*   **标准 `Option<T>` 的局限性**: 如果字段类型仅为 `Option<String>` 并使用 `#[serde(default)]`，则上述情况 1 和 3 都会导致反序列化后的结构体字段为 `None`，无法区分。
*   **`double_option` 模块的自定义反序列化**:
    *   通过 `#[serde(default, with = "double_option")]` 应用于 `UpdateTaskPayload` 中的 `description` 字段。
    *   其 `deserialize` 函数内部逻辑能够处理这种区分：它期望字段在 JSON 中表现为 `Option<Option<T>>` 的语义（尽管实际载荷字段仍是 `Option<T>`）。
        *   JSON 中**无此键** -> `UpdateTaskPayload` 字段为 `None` (通过 `default`)。服务层通常解释为“不更新此字段”。
        *   JSON 中**键值为 `null`** -> `UpdateTaskPayload` 字段为 `None` (通过 `double_option` 的逻辑)。服务层通常解释为“将此字段值设为 NULL”。
        *   JSON 中**键值为 `"some_value"`** -> `UpdateTaskPayload` 字段为 `Some("some_value")`。服务层解释为“更新为此值”。
    *   **注意**: 服务层在处理时，需要根据 `UpdateTaskPayload` 字段是否为 `Some` 或 `None`，并结合业务需求（例如一个字段是否真的允许被设为 NULL），来决定如何更新 `ActiveModel`。对于 `ActiveModel`，`field: Set(None)` 会将数据库列设为 NULL，而不调用 `Set` 则保持原样。

## 6. `serde` 的广泛应用

*   **序列化 (Serialization)**: 将 Rust 结构体 (如 `user::Model`, `task::Model`, `Claims`, `UserResponse`, `LoginResponse`) 转换为 JSON，主要用于 API 响应。
    *   `#[serde(skip_serializing_if = "Option::is_none")]`: 可用于在序列化时跳过值为 `None` 的 `Option` 字段，使 JSON 输出更简洁。
*   **反序列化 (Deserialization)**: 将来自 API 请求的 JSON 数据转换为 Rust 结构体 (如各种 Payloads, `Claims`)。

## 7. 与其他层的关系

*   **服务层 (Service)**:
    *   使用 `ActiveModel` 来构建和执行数据库的插入和更新操作。
    *   使用 `Entity::find*()` 方法进行数据查询，通常结果会映射到 `Model` 结构体。
    *   接收来自控制器层的 Payloads，并将其数据用于构建 `ActiveModel`。
*   **控制器层 (Controller)**:
    *   接收 HTTP 请求，并将 JSON 请求体反序列化为相应的 Payload 结构体。
    *   调用服务层函数，传递从 Payload 中获取的数据。
    *   从服务层接收 `Model` 结构体的实例（或包含它们的 `Vec`），并将这些 `Model` 序列化为 JSON 作为 HTTP 响应。
*   **数据库交互 (`db.rs`)**:
    *   模型层定义的实体 (`user::Entity`, `task::Entity`) 是 `db.rs` 中 `run_migrations` 函数创建表的基础。SeaORM 根据实体定义生成相应的 SQL 来建表。