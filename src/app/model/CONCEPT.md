# 模型层 (Model) 核心概念

## 1. 职责与定位

模型层 (`src/app/model/`) 在应用程序架构中扮演着【数据蓝图】的角色。它的核心职责是：

- **定义数据结构**: 清晰地描述应用程序需要处理的核心业务实体（如"任务" `Task`）以及它们拥有的属性（如 `id`, `title`, `completed`）。
- **定义数据传输对象 (DTO)**: 定义用于特定操作（如创建、更新）的数据载体结构（如 `CreateTaskPayload`, `UpdateTaskPayload`）。这些结构体通常直接映射到 API 请求或响应的格式（如 JSON）。
- **封装数据验证（可选）**: 虽然本项目中验证逻辑主要在服务层，但简单的格式或值域验证有时也可以在模型层通过属性宏（如 `serde` 的校验属性）实现。

**关键点**: 模型层【不应该】包含业务逻辑、数据库交互或 HTTP 请求处理逻辑。它只关注"数据长什么样"。

## 2. `Task` 结构体

- **核心业务实体**: 代表系统中的一个基本任务单元。
- **字段**: 包含了任务的所有持久化属性。
- **`#[derive(Clone, Debug, Serialize, Deserialize)]`**: 
    - `Serialize`/`Deserialize`: 表明 `Task` 可以与 JSON 等格式进行相互转换（主要用于 API 响应，或从存储加载）。
    - `Clone`: 允许创建 `Task` 的副本，便于在不同地方传递数据而无需转移所有权。
    - `Debug`: 方便开发者打印调试信息。

## 3. 请求载荷 (`CreateTaskPayload`, `UpdateTaskPayload`)

- **数据传输对象 (DTO)**: 专门用于 API 请求。
- **`#[derive(Deserialize)]`**: 表明这些结构体可以从传入的 JSON 请求体中创建。
- **字段设计**: 
    - `CreateTaskPayload`: 字段通常是必需的（或有默认值），因为创建时需要提供基本信息。
    - `UpdateTaskPayload`: 字段通常是 `Option<T>` 类型，因为更新时客户端只需提供需要修改的字段。

## 4. `serde` 的广泛应用

- **序列化 (Serialization)**: 将 Rust 结构体 (`Task`) 转换为 JSON，用于 API 响应。
    - `#[serde(skip_serializing_if = "Option::is_none")]`: 定制序列化行为，当 `Option` 字段为 `None` 时不在 JSON 中输出该键，使输出更简洁。
- **反序列化 (Deserialization)**: 将来自 API 请求的 JSON 数据转换为 Rust 结构体 (`CreateTaskPayload`, `UpdateTaskPayload`)。
    - `#[serde(default)]`: 当 JSON 中缺少某个字段或值为 `null` 时，使用该字段类型的默认值（如 `Option` 的 `None`，`bool` 的 `false`）。
    - `#[serde(with = "...")]`: 使用自定义的序列化/反序列化逻辑（如 `double_option` 模块），处理更复杂的场景。

## 5. `double_option` 模块

- **解决特定问题**: 处理 `UpdateTaskPayload` 中可选字段（如 `description`）更新时，区分"未提供"（不修改）和"值为 null"（清除）这两种意图。
- **实现**: 通过自定义 `deserialize` 函数，内部解析 `Option<Option<T>>`，外部返回 `Option<T>`，巧妙地将 JSON 的三种状态（键不存在、值为 null、有值）映射到最终的 `Option<T>` 类型中。

## 6. 与其他层的关系

- **被服务层 (Service) 使用**: 服务层会创建和操作 `Task` 实例。
- **被控制器层 (Controller) 使用**: 控制器层接收 `CreateTaskPayload` 和 `UpdateTaskPayload` 作为请求输入，并将 `Task` 实例序列化后作为响应输出。
- **与数据访问层 (DB) 交互**: 数据访问层负责将 `Task` 实例持久化到存储（如内存、数据库）或从存储中加载它们。 