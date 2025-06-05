// 文件路径: src/app/model/auth_dtos.rs

// /--------------------------------------------------------------------------------------------------\
// |                               【模块功能图示】 (auth_dtos.rs)                                     |
// |--------------------------------------------------------------------------------------------------|
// |                                                                                                  |
// | [客户端 HTTP 请求 (JSON)]                                                                         |
// |   (例如: POST /api/register, body: {"username": "u", "password": "p"})                           |
// |      |                                                                                           |
// |      V (Axum 使用 serde Deserialize 进行反序列化)                                                   |
// | [`RegisterPayload` 或 `LoginPayload` 结构体实例] (Rust 对象，包含从 JSON 解析的数据)                   |
// |      |                                                                                           |
// |      V (传递给服务层进行业务逻辑处理)                                                                 |
// | [应用核心逻辑 (AuthService)]                                                                       |
// |      |                                                                                           |
// |      V (例如: 登录成功，生成 JWT Claims)                                                            |
// | [`Claims` 结构体实例] (Rust 对象，包含 JWT 声明)                                                   |
// |      |                                                                                           |
// |      V (AuthService 使用 jsonwebtoken Serialize 进行编码)                                          |
// | [JWT 字符串]                                                                                       |
// |      | (包装在 `LoginResponse` 中)                                                                |
// |      V                                                                                           |
// | [`LoginResponse` 或 `UserResponse` 结构体实例] (Rust 对象，准备返回给客户端的数据)                    |
// |      |                                                                                           |
// |      V (Axum 使用 serde Serialize 进行序列化)                                                      |
// | [服务端 HTTP 响应 (JSON)]                                                                         |
// |   (例如: 200 OK, body: {"token": "jwt_string"})                                                  |
// |   (例如: 201 Created, body: {"id": 1, "username": "u", ...})                                   |
// |                                                                                                  |
// \--------------------------------------------------------------------------------------------------/
//
// 【模块核心职责】 (Module Core Responsibilities)
// 1. **定义数据传输对象 (Data Transfer Objects - DTOs)**: 此模块定义了用于在客户端和服务器之间传输数据的 Rust 结构体。
//    - **请求载荷 (Request Payloads)**: 如 `RegisterPayload` 和 `LoginPayload`，用于从客户端接收的 JSON 请求体反序列化成 Rust 结构体。
//    - **响应体 (Response Bodies)**: 如 `LoginResponse` 和 `UserResponse`，用于将服务器端的 Rust 数据结构序列化为 JSON 响应体发送给客户端。
// 2. **定义 JWT 声明结构 (`Claims`)**: `Claims` 结构体定义了 JSON Web Token (JWT) 中包含的数据声明，例如用户标识 (subject) 和过期时间 (expiration)。
// 3. **确保类型安全的数据交换**: 通过为这些 DTOs 和 `Claims` 结构体派生 `serde::Serialize` 和 `serde::Deserialize` traits，
//    可以确保在 JSON 数据和 Rust 类型之间进行安全、可靠的转换。这有助于在编译时捕获数据格式不匹配等问题。
// 4. **API 契约**: 这些 DTOs 在某种程度上也定义了 API 的“契约”，明确了客户端应发送什么样的数据，以及服务器会返回什么样的数据。
//
// 【关键技术点】 (Key Technologies)
// - **Rust 结构体 (`struct`)**: 用于定义自定义数据类型，将相关字段组合在一起。每个 DTO 和 `Claims` 都是一个结构体。
// - **Serde (序列化/反序列化框架)**:
//   - `Serialize` trait: 允许将 Rust 数据结构转换为其他格式 (如 JSON)。通过 `#[derive(Serialize)]` 自动实现。
//   - `Deserialize` trait: 允许从其他格式 (如 JSON) 的数据创建 Rust 数据结构。通过 `#[derive(Deserialize)]` 自动实现。
//   - 是 Rust 生态中最流行的数据序列化/反序列化库。
// - **派生宏 (`#[derive(...)]`)**: Rust 的一种元编程特性，用于在编译时自动为类型生成代码以实现某些 traits。
//   - `Debug`: 实现 `std::fmt::Debug`，允许使用 `{:?}` 打印结构体进行调试。
//   - `Clone`: 实现 `Clone`，允许创建结构体实例的副本。
// - **数据类型**: 如 `String` (文本字符串), `i32` (32位整数), `usize` (指针大小的无符号整数, 常用于时间戳), `chrono::DateTime<Utc>` (带时区的日期时间)。
// - **`From` trait**: Rust 的标准转换 trait，用于实现类型之间的转换逻辑 (例如从数据库实体 `user_entity::Model` 转换为 API 响应 DTO `UserResponse`)。
// - **模块化**: 将 DTOs 和 `Claims` 放在专门的模块中，有助于保持代码组织清晰。

// --- 导入依赖 ---
// `use serde::{Deserialize, Serialize};`
//   - 从 `serde` crate 中导入 `Deserialize` 和 `Serialize` 这两个核心 traits。
//   - `Serialize`: 用于将 Rust 数据结构（如我们的 DTO 结构体）转换为某种序列化格式（本项目中主要是 JSON）。
//   - `Deserialize`: 用于将序列化格式的数据（如来自 HTTP 请求体的 JSON）转换为 Rust 数据结构。
//   通过在结构体上使用 `#[derive(Serialize)]` 和 `#[derive(Deserialize)]`，可以让 `serde` 自动为我们实现这些转换逻辑。
use serde::{Deserialize, Serialize};
// `use chrono::{DateTime, Utc};`
//   - 从 `chrono` crate (一个流行的 Rust 日期和时间处理库) 中导入 `DateTime` 和 `Utc` 类型。
//   - `DateTime<Utc>`: 表示一个与 UTC (协调世界时) 相关联的特定日期和时间点。
//     常用于存储时间戳，如记录的创建时间 (`created_at`) 和更新时间 (`updated_at`)。
use chrono::{DateTime, Utc};
// `use crate::app::model::user_entity;`
//   - `crate::`: 表示从当前项目的根模块开始的路径。
//   - 导入在 `src/app/model/user_entity.rs` 中定义的 `user_entity` 模块。
//   - 这是为了在下面的 `impl From<user_entity::Model> for UserResponse` 实现中使用 `user_entity::Model` 类型。
use crate::app::model::user_entity; // 用于 UserResponse 的 From<user_entity::Model> 实现

// --- 请求载荷 (Request Payloads) ---
// 请求载荷结构体用于定义客户端在发起特定 API 请求时，HTTP 请求体中应包含的数据格式。
// 它们通常会派生 `Deserialize` trait，以便 Axum (通过 serde) 能自动将传入的 JSON 请求体转换为这些结构体的实例。

// `#[derive(Deserialize, Debug)]`:
// - `Deserialize`: 告诉 `serde` 为 `RegisterPayload` 自动生成从序列化格式 (如 JSON) 反序列化的代码。
//   这意味着当 Axum 控制器接收到一个包含 JSON 的 HTTP POST 请求时，
//   它可以尝试将这个 JSON 解析成一个 `RegisterPayload` 实例。
//   例如: `Json(payload): Json<RegisterPayload>` 在 Axum handler 中。
// - `Debug`: 允许使用 `{:?}` 打印 `RegisterPayload` 实例，方便调试。
#[derive(Deserialize, Debug)]
// `pub struct RegisterPayload { ... }`: 定义一个公共结构体 `RegisterPayload`。
//   用于封装用户注册时客户端需要发送的数据。
pub struct RegisterPayload {
    // `pub username: String,`: 公共字段 `username`，类型为 `String`。
    //   表示用户注册时提供的用户名。客户端发送的 JSON 中应包含一个名为 "username" 的字符串字段。
    pub username: String,
    // `pub password: String,`: 公共字段 `password`，类型为 `String`。
    //   表示用户注册时提供的密码。客户端发送的 JSON 中应包含一个名为 "password" 的字符串字段。
    pub password: String,
}

// `#[derive(Deserialize, Debug)]`: 类似 `RegisterPayload`，用于反序列化和调试。
#[derive(Deserialize, Debug)]
// `pub struct LoginPayload { ... }`: 定义一个公共结构体 `LoginPayload`。
//   用于封装用户登录时客户端需要发送的数据。
pub struct LoginPayload {
    // `pub username: String,`: 用户登录时提供的用户名。
    pub username: String,
    // `pub password: String,`: 用户登录时提供的密码。
    pub password: String,
}

// --- 响应体 (Response Bodies) ---
// 响应体结构体用于定义服务器向客户端返回数据时，HTTP 响应体的数据格式。
// 它们通常会派生 `Serialize` trait，以便 Axum (通过 serde) 能自动将这些结构体的实例序列化为 JSON 响应体。

// `#[derive(Serialize, Debug)]`:
// - `Serialize`: 告诉 `serde` 为 `LoginResponse` 自动生成序列化为某种格式 (如 JSON) 的代码。
//   这意味着当 Axum 控制器返回一个 `Json<LoginResponse>` 时，
//   `LoginResponse` 实例会被转换成 JSON 字符串作为 HTTP 响应体。
// - `Debug`: 用于调试打印。
#[derive(Serialize, Debug)]
// `pub struct LoginResponse { ... }`: 定义一个公共结构体 `LoginResponse`。
//   用于封装用户成功登录后服务器返回的数据。
pub struct LoginResponse {
    // `pub token: String,`: 公共字段 `token`，类型为 `String`。
    //   表示用户成功登录后，服务器颁发的认证令牌 (JWT 字符串)。
    //   客户端在后续请求受保护资源时需要携带此令牌。
    pub token: String,
}

// `#[derive(Serialize, Debug, Clone)]`:
// - `Serialize`, `Debug`: 同上。
// - `Clone`: 允许创建 `UserResponse` 实例的副本。这在某些场景下可能有用，例如在返回之前需要对副本进行一些处理。
#[derive(Serialize, Debug, Clone)]
// `pub struct UserResponse { ... }`: 定义一个公共结构体 `UserResponse`。
//   用于在 API 响应中安全地表示用户信息，通常在用户注册成功或获取用户信息时返回。
//   **重要**: 这个结构体不应包含敏感信息，如哈希密码。
pub struct UserResponse {
    // `pub id: i32,`: 用户的唯一 ID (整数类型)。
    pub id: i32,
    // `pub username: String,`: 用户的用户名。
    pub username: String,
    // `pub created_at: DateTime<Utc>,`: 用户账户的创建时间。
    //   类型为 `chrono::DateTime<Utc>`，表示一个带时区 (UTC) 的精确时间点。
    //   当通过 `serde` 序列化为 JSON 时，`chrono` 的 `serde` 特性 (如果启用)
    //   通常会将其转换为标准的日期时间字符串格式 (如 RFC3339: "2023-10-27T10:30:00Z")。
    pub created_at: DateTime<Utc>,
    // `pub updated_at: DateTime<Utc>,`: 用户账户的最后更新时间。
    pub updated_at: DateTime<Utc>,
}

// `impl From<user_entity::Model> for UserResponse { ... }`
// - `impl From<T> for U`: 这是 Rust 中实现标准转换 trait `std::convert::From` 的方式。
//   `From<T>` trait 用于定义如何从类型 `T` 的一个值创建类型 `U` 的一个值。
//   实现 `From<T> for U` 会自动提供一个 `.into()` 方法 (通过 `Into<U>` trait)，允许 `T` 类型的值调用 `t.into()` 来转换为 `U`。
// - `From<user_entity::Model> for UserResponse`: 表示我们正在定义如何从数据库实体模型 `user_entity::Model` 转换为 API 响应 DTO `UserResponse`。
//   这是一种常见的模式，用于将数据库层的数据结构适配为适合 API 暴露的数据结构，通常会省略或转换某些字段 (如哈希密码)。
impl From<user_entity::Model> for UserResponse {
    // `fn from(model: user_entity::Model) -> Self { ... }`: `From` trait 要求实现的方法。
    // - `model: user_entity::Model`: 参数 `model` 是源类型 (`user_entity::Model`) 的一个实例。
    //   这里通过值传递，意味着函数会获取 `model` 的所有权。
    // - `-> Self`: 返回类型是 `Self`，它指代当前 `impl` 块的目标类型，即 `UserResponse`。
    fn from(model: user_entity::Model) -> Self {
        // `Self { ... }`: 使用结构体字面量创建并返回一个新的 `UserResponse` 实例。
        //   - `id: model.id,`: 将 `user_entity::Model` 的 `id` 字段的值赋给 `UserResponse` 的 `id` 字段。
        //     `i32` 类型是 `Copy` 的，所以这里是值的拷贝。
        //   - `username: model.username,`: 将 `user_entity::Model` 的 `username` 字段的值赋给 `UserResponse` 的 `username` 字段。
        //     `String` 类型不是 `Copy` 的，所以这里是所有权的【移动 (move)】。`model.username` 的所有权被转移到新的 `UserResponse` 中。
        //     (由于 `model` 参数本身也是按值传递的，其所有权在函数结束时无论如何都会被处理，所以这里的移动是安全的。)
        //   - `created_at: model.created_at,`: `DateTime<Utc>` (来自 `sea_orm::entity::prelude::DateTimeUtc`，通常是 `chrono::DateTime<Utc>` 的别名) 是 `Copy` 的 (如果其内部表示是 `Copy` 的，如时间戳)。
        //     如果是结构体，则取决于其是否实现 `Copy`。`chrono::DateTime<Utc>` 本身不是 `Copy`，但 SeaORM 的 `DateTimeUtc` 可能是。
        //     假设它是可简单复制或移动的。
        //   - `updated_at: model.updated_at,`: 同上。
        Self {
            id: model.id,
            username: model.username,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
        // 注意：`hashed_password` 字段没有从 `user_entity::Model` 映射过来，这是故意的，
        // 因为我们不希望在 API 响应中暴露用户的哈希密码。
    }
}

// 下面的注释块是关于直接序列化 DateTime<Utc> 的一些通用提示，与本项目当前实现不直接相关，但对学习有益。
// 如果你需要直接序列化包含 `DateTime<Utc>` 的其他模型 (而不是通过 `UserResponse` DTO)，
// 并且希望控制其序列化格式 (例如，序列化为 Unix 时间戳秒数，而不是 RFC3339 字符串)，
// 你可能需要启用 `chrono` crate 的 "serde" 特性，并可能使用 `#[serde(with = "...")]` 属性。
// 例如:
// #[derive(Serialize, Deserialize)]
// struct SomeOtherModel {
//   #[serde(with = "chrono::serde::ts_seconds")] // 将 DateTime<Utc> 序列化/反序列化为秒级时间戳
//   pub timestamp: DateTime<Utc>,
// }
// 对于 `user_entity::Model` 本身，如果直接在 Axum handler 中返回 `Json<user_entity::Model>`，
// SeaORM 的 `DateTimeUtc` 类型 (通常是 `chrono::DateTime<Utc>` 的包装或别名)
// 配合 `serde` (如果 `Model` 派生了 `Serialize`) 通常会默认序列化为 RFC3339 格式的字符串。
// 对于我们自定义的 DTO `UserResponse`，直接使用 `DateTime<Utc>` 字段，
// 并确保 `chrono` 的 "serde" 特性已在 `Cargo.toml` 中启用，即可使其正确序列化为 JSON 字符串。

// --- JWT Claims (JSON Web Token 声明) ---
// `Claims` 结构体定义了我们将在 JSON Web Token (JWT) 的载荷 (payload) 部分存储的数据。
// JWT 用于无状态认证，这些声明包含了关于认证主体 (用户) 和令牌本身的信息。

// `#[derive(Debug, Serialize, Deserialize, Clone)]`:
// - `Debug`: 用于调试打印。
// - `Serialize`, `Deserialize`: 允许 `Claims` 实例与 JSON 格式相互转换。
//   - `Serialize`: 当生成 JWT 时，`Claims` 实例会被序列化为 JSON 字符串，然后进行 Base64Url 编码作为 JWT 的第二部分 (Payload)。
//   - `Deserialize`: 当验证和解码 JWT 时，从 JWT Payload 部分提取的 JSON 数据会被反序列化回 `Claims` 实例。
// - `Clone`: 允许创建 `Claims` 实例的副本。在 JWT 中间件中，当我们将解码后的 Claims 存入请求扩展时，
//   原始的 `jsonwebtoken::TokenData<Claims>` 中的 `claims` 字段会被克隆一份。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    // `pub sub: String,`: "Subject" (主体) 声明。
    //   - 这是 JWT 标准注册声明之一 (Registered Claim Name)。
    //   - 通常用于唯一标识 JWT 所涉及的主体，例如用户的 ID 或用户名。
    //   - 在本项目中，我们将其用作用户的唯一标识符 (例如，从数据库获取的用户 `id` 转换为字符串)。
    pub sub: String,

    // `pub exp: usize,`: "Expiration Time" (过期时间) 声明。
    //   - 标准注册声明之一。
    //   - 定义了 JWT 在何时之后不再被接受处理。其值必须是一个数字，表示 Unix 时间戳 (NumericDate)。
    //   - **什么是 Unix 时间戳?** 从 UTC 1970年1月1日 00:00:00 开始所经过的秒数 (不考虑闰秒)。
    //   - 使用 `usize` 类型存储时间戳是 `jsonwebtoken` crate 的要求之一。
    //   - 设置过期时间是 JWT 安全性的重要组成部分，可以防止令牌被无限期使用。
    pub exp: usize,

    // `pub iat: usize,`: "Issued At" (签发时间) 声明。
    //   - 标准注册声明之一。
    //   - 定义了 JWT 的签发时间。其值也是一个 Unix 时间戳 (NumericDate)。
    //   - 可以用于记录令牌的“年龄”。
    pub iat: usize,

    // 根据应用需求，还可以添加其他自定义声明，例如：
    // `roles: Vec<String>,` // 用户角色
    // `permissions: Vec<String>,` // 用户权限
    // `iss: String,` // Issuer (签发者)，另一个标准声明
    // `aud: String,` // Audience (受众)，另一个标准声明
}
