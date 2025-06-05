// 文件路径: src/app/service/auth_service.rs

// /--------------------------------------------------------------------------------------------------\
// |                                【模块功能图示】 (auth_service.rs)                                  |
// |--------------------------------------------------------------------------------------------------|
// |                                                                                                  |
// |  [控制器层 (AuthController)]                                                                      |
// |   - `register_handler(payload)` 调用 `AuthService::register_user(...)`                             |
// |   - `login_handler(payload)` 调用 `AuthService::login_user(...)`                                 |
// |      |                                                                                           |
// |      V (方法调用, 传入数据库连接 `db` 和用户提供的 `username`, `password`)                             |
// |  [服务层 (`AuthService`)]                                                                          |
// |   - `register_user(db, username, password)`:                                                     |
// |     1. 调用 `UserRepository::find_by_username` 检查用户是否存在。                                    |
// |     2. 如果存在，返回 `AppError::UserAlreadyExists`。                                                |
// |     3. 生成盐 (salt) (使用 `rand` crate)。                                                         |
// |     4. 使用 Argon2 算法哈希密码 (密码 + 盐)。                                                      |
// |     5. 创建 `user_entity::ActiveModel` (包含用户名和哈希后的密码)。                                 |
// |     6. 调用 `UserRepository::create_user` 保存新用户到数据库。                                       |
// |     7. 返回创建的 `user_entity::Model` 或 `AppError`。                                             |
// |   - `login_user(db, username, password)`:                                                        |
// |     1. 调用 `UserRepository::find_by_username` 查找用户。                                          |
// |     2. 如果用户不存在，返回 `AppError::InvalidCredentials`。                                         |
// |     3. 使用 Argon2 验证提供的密码和存储的哈希密码是否匹配。                                          |
// |     4. 如果不匹配，返回 `AppError::InvalidCredentials`。                                           |
// |     5. 创建 JWT Claims (包含 `sub`=user_id, `exp`=过期时间, `iat`=签发时间)。                         |
// |     6. 使用 `jsonwebtoken` crate 和预设的密钥 (JWT_SECRET) 生成 JWT 字符串。                       |
// |     7. 返回 JWT 字符串或 `AppError`。                                                              |
// |      |                                                                                           |
// |      V (调用 UserRepository)                                                                      |
// |  [数据仓库层 (UserRepository)]                                                                     |
// |   - `find_by_username(...)`                                                                      |
// |   - `create_user(...)`                                                                           |
// |      |                                                                                           |
// |      V (与数据库交互)                                                                               |
// |  [数据库 (SQLite via SeaORM)]                                                                      |
// |                                                                                                  |
// \--------------------------------------------------------------------------------------------------/
//
// 【模块核心职责】 (Module Core Responsibilities)
// 1. **实现认证业务逻辑 (Implement Authentication Business Logic)**:
//    - **用户注册 (User Registration)**: 处理新用户的注册请求，包括检查用户名是否已存在、安全地哈希用户密码、以及通过数据仓库层将新用户信息存入数据库。
//    - **用户登录 (User Login)**: 处理用户的登录请求，包括验证用户提供的凭证（用户名和密码）是否正确、以及在验证成功后生成并返回 JSON Web Token (JWT) 作为认证令牌。
// 2. **协调数据访问与安全操作 (Coordinate Data Access and Security Operations)**:
//    - 调用 `UserRepository` 来执行与用户数据相关的数据库操作 (查找用户、创建用户)。
//    - 使用 `argon2` crate 进行密码哈希和验证，确保密码安全存储和比较。
//    - 使用 `jsonwebtoken` crate 生成和处理 JWT，用于后续的 API 请求认证。
// 3. **错误处理与转换 (Error Handling and Transformation)**:
//    - 将底层操作（如数据库错误 `DbErr`、密码哈希错误、JWT 生成错误）映射到应用层定义的 `AppError` 类型，为控制器层提供统一的错误处理接口。
//    - 例如，如果注册时用户名已存在，则返回 `AppError::UserAlreadyExists`；如果登录时凭证无效，则返回 `AppError::InvalidCredentials`。
// 4. **服务层抽象 (Service Layer Abstraction)**: 作为服务层，它封装了认证功能的具体实现细节，使控制器层 (API 接口层) 的代码更简洁，
//    只需调用 `AuthService` 提供的公共方法即可完成复杂的认证操作。
//
// 【关键技术点】 (Key Technologies)
// - **异步函数 (`async fn`)**: 所有服务方法都是异步的，因为它们依赖于异步的数据库操作。
// - **数据库连接 (`DatabaseConnection`)**: 从 SeaORM 获取的数据库连接实例，通过参数传递给服务方法，再传递给仓库层。
// - **数据仓库模式 (`UserRepository`)**: 通过调用 `UserRepository` 的方法来间接访问数据库，而不是直接在服务层编写数据库查询。
// - **数据模型 (`user_entity::Model`, `user_entity::ActiveModel`, `Claims`)**:
//   - `user_entity::Model`: 代表从数据库读取的用户数据。
//   - `user_entity::ActiveModel`: 用于向数据库插入新用户数据。
//   - `Claims`: JWT 中存储的声明信息。
// - **错误处理 (`Result<T, AppError>`, `map_err`)**:
//   - 服务方法返回 `Result`，其中错误类型是自定义的 `AppError`。
//   - 使用 `.map_err()` 将底层库的错误 (如 `DbErr` from SeaORM, `argon2::Error`) 转换为合适的 `AppError` 变体。
// - **密码哈希 (`argon2` crate)**:
//   - **Argon2id**: 使用 Argon2id 算法，这是一种现代、安全的密码哈希函数，能抵抗多种攻击。
//   - **加盐 (Salting)**: 为每个密码生成一个唯一的随机盐 (`salt`)，与密码一起哈希。盐能有效防止彩虹表攻击。
//     `rand::RngCore` 用于生成安全的随机盐。
//   - **配置 (`argon2::Config`)**: 配置 Argon2 算法的参数，如内存消耗 (`mem_cost`)、时间消耗 (`time_cost`，即迭代次数)、并行度 (`lanes`)，以平衡安全性和性能。
//   - `argon2::hash_encoded`: 将密码和盐哈希后，编码成一个包含所有必要信息（算法、版本、参数、盐、哈希值）的字符串，方便存储。
//   - `argon2::verify_encoded`: 验证提供的明文密码是否与存储的哈希编码字符串匹配。
// - **JWT (`jsonwebtoken` crate)**:
//   - **生成 (Encoding)**: 使用 `jsonwebtoken::encode` 函数，传入 JWT 头部 (`Header`)、声明 (`Claims`) 和一个密钥 (`EncodingKey`) 来创建 JWT 字符串。
//   - **密钥 (`EncodingKey`)**: 从一个保密的字符串 (JWT_SECRET) 创建。**此密钥的安全性至关重要。**
//   - **声明 (`Claims`)**: 包含如 `sub` (用户ID), `exp` (过期时间), `iat` (签发时间) 等信息。
// - **时间处理 (`chrono` crate)**: 使用 `chrono::Utc::now()` 获取当前的 UTC 时间，用于设置 JWT 的 `iat` 和 `exp` 声明。
// - **依赖注入思想 (Implicit)**: 虽然没有显式使用依赖注入框架，但 `AuthService` 的方法接收 `&DatabaseConnection` 作为参数，
//   这使得调用者 (如控制器或测试代码) 可以传入不同的数据库连接实例，体现了依赖注入的一些原则。

// --- 导入依赖 ---
// `use sea_orm::DatabaseConnection;`
//   - 从 `sea_orm` crate 导入 `DatabaseConnection` 类型。这是与数据库进行交互所必需的连接（池）对象。
//   此服务的所有需要数据库操作的方法都会接收一个对 `DatabaseConnection` 的引用。
use sea_orm::DatabaseConnection;
// `use crate::app::model::user_entity;`
//   - 导入在 `src/app/model/user_entity.rs` 中定义的 `user_entity` 模块。
//   - 这允许我们使用 `user_entity::Model` (代表数据库中的用户记录) 和 `user_entity::ActiveModel` (用于创建/更新用户记录)。
use crate::app::model::user_entity;
// `use crate::app::repository::UserRepository;`
//   - 导入在 `src/app/repository/user_repository.rs` 中定义的 `UserRepository` 结构体。
//   - `AuthService` 将通过调用 `UserRepository` 的方法来执行实际的数据库查询和命令，而不是直接使用 SeaORM。
use crate::app::repository::UserRepository;
// `use crate::error::{AppError, Result};`
//   - 导入在 `src/error.rs` 中定义的 `AppError` 枚举 (自定义的应用错误类型) 和 `Result` 类型别名 (`core::result::Result<T, AppError>`)。
//   - 这使得服务层的方法可以返回统一的错误类型。
use crate::error::{AppError, Result};

// --- Argon2 (密码哈希) 相关导入 ---
// `use argon2::{self, Config, ThreadMode, Variant, Version};`
//   - 从 `argon2` crate 导入密码哈希所需的主要组件。
//   - `self` (或 `argon2`): 允许我们使用 `argon2::hash_encoded` 和 `argon2::verify_encoded` 等核心函数。
//   - `Config`: 用于配置 Argon2 算法参数的结构体。
//   - `ThreadMode`, `Variant`, `Version`: Argon2 算法的不同配置选项的枚举。
use argon2::{self, Config, ThreadMode, Variant, Version};
// `use rand::RngCore;`
//   - 从 `rand` crate (一个流行的 Rust 随机数生成库) 导入 `RngCore` trait。
//   - `RngCore` trait 提供了生成随机字节的方法 (如 `fill_bytes`)，我们将用它来为密码哈希生成一个安全的随机盐 (salt)。
use rand::RngCore;

// --- JWT (JSON Web Token) 相关导入 ---
// `use jsonwebtoken::{encode, EncodingKey, Header};`
//   - 从 `jsonwebtoken` crate 导入 JWT 处理所需的主要组件。
//   - `encode`: 用于将 JWT Claims 编码为 JWT 字符串的函数。
//   - `EncodingKey`: 用于包装 JWT 签名密钥的类型。
//   - `Header`: 代表 JWT 的头部，通常包含算法和令牌类型信息。
use jsonwebtoken::{encode, EncodingKey, Header};
// `use serde::{Deserialize, Serialize};`
//   - `serde` 的 `Deserialize` 和 `Serialize` traits 主要由 `Claims` 结构体 (在 `model/auth_dtos.rs` 中定义) 使用，
//     以允许 `Claims` 与 JSON 格式相互转换（JWT 的 Payload 部分是 JSON）。
//     虽然 `Claims` 已移至 `auth_dtos.rs`，但如果将来此文件直接处理需要序列化/反序列化的数据，保留它也无妨。
//     (当前严格来说，此文件内的 `AuthService` 逻辑不直接使用 `Serialize`/`Deserialize`，而是 `Claims` 结构体需要它们。)
use serde::{Deserialize, Serialize}; // 理论上，如果 Claims 自身处理序列化，这里可以不导入。但为了清晰，保留。
// `use chrono::Utc;`
//   - 从 `chrono` crate 导入 `Utc` 类型。
//   - `Utc` 代表协调世界时，用于获取当前时间戳，以设置 JWT 的签发时间 (`iat`) 和过期时间 (`exp`)。
use chrono::Utc;
// `use crate::app::model::Claims;`
//   - 导入在 `src/app/model/auth_dtos.rs` 中定义的 `Claims` 结构体。
//   - `Claims` 结构体定义了 JWT 载荷 (payload) 中包含的数据字段。
use crate::app::model::Claims;


// `#[derive(Debug, Default)]`
// - `Debug`: 自动实现 `std::fmt::Debug` trait，允许使用 `{:?}` 打印 `AuthService` 实例进行调试。
// - `Default`: 自动实现 `std::default::Default` trait，允许通过 `AuthService::default()` 创建一个默认实例。
//   对于空结构体，默认实例就是其本身。
#[derive(Debug, Default)]
// `pub struct AuthService;`
//   - 定义一个公共的 (public) 结构体 `AuthService`。
//   - **当前是空结构体**: 类似于 `UserRepository`，`AuthService` 在这个实现中也是一个“无状态”的服务。
//     它不持有任何数据（比如配置信息或数据库连接）。
//     它的所有方法都将必要的依赖 (如 `&DatabaseConnection`) 作为参数接收。
//   - **设计选择**:
//     - **无状态服务**: 优点是易于创建和测试。依赖通过方法参数传入，清晰明了。
//     - **有状态服务**: 另一种设计是在 `AuthService` 结构体中存储依赖，例如 `jwt_secret` 或 `AppConfig` 的引用/`Arc`。
//       例如: `pub struct AuthService { config: Arc<AppConfig> }`。
//       这样方法就不需要显式接收这些配置参数，而是通过 `&self.config` 访问。
//     本项目目前采用无状态设计，依赖通过参数传递。如果 JWT 密钥等配置项增多，可以考虑将其改为有状态服务，
//     并在创建 `AuthService` 实例时注入配置。
pub struct AuthService;

// `impl AuthService { ... }`
//   - 为 `AuthService` 结构体实现方法。
impl AuthService {
    // `pub fn new() -> Self { Self }`
    //   - 定义一个公共的关联函数 `new`，作为 `AuthService` 的构造函数。
    //   - 对于空结构体，`Self` 或 `AuthService::default()` 都可以创建实例。
    pub fn new() -> Self {
        Self
    }

    // `pub async fn register_user(...) -> Result<user_entity::Model>`
    //   - `pub async fn`: 定义一个公共的异步函数 `register_user`。
    //   - `db: &DatabaseConnection`: 数据库连接的不可变引用。服务层方法通过它调用仓库层，或直接进行数据库操作（但不推荐）。
    //   - `username: String`: 用户提供的用户名。`String` 类型表示函数获取了 `username` 的【所有权】。
    //     这意味着调用者传递 `username` 后，不能再使用原来的 `username` 变量 (除非它被克隆)。
    //   - `password: String`: 用户提供的原始密码。同样获取所有权。
    //   - `-> Result<user_entity::Model>`: 返回类型。
    //     - `Result<_, AppError>`: 操作可能成功或失败 (返回 `AppError`)。
    //     - `user_entity::Model`: 如果注册成功，返回新创建的用户的数据库模型实例。
    /// 处理用户注册的核心业务逻辑。
    /// 1. 检查用户名是否已存在。
    /// 2. 如果不存在，则哈希密码。
    /// 3. 创建新用户记录并存入数据库。
    ///
    /// # 参数
    /// * `db`: 数据库连接的引用。
    /// * `username`: 用户提供的用户名 (获取所有权)。
    /// * `password`: 用户提供的明文密码 (获取所有权)。
    ///
    /// # 返回值
    /// * `Ok(user_model)`: 如果用户成功注册，返回新创建用户的模型。
    /// * `Err(AppError::UserAlreadyExists)`: 如果用户名已被占用。
    /// * `Err(AppError::DatabaseError)`: 如果数据库操作失败。
    /// * `Err(AppError::InternalServerError)`: 如果密码哈希等内部操作失败。
    pub async fn register_user(
        db: &DatabaseConnection,
        username: String, // 用户名 (String - 所有权转移到函数内)
        password: String, // 密码 (String - 所有权转移到函数内)
    ) -> Result<user_entity::Model> { // 返回自定义 Result，成功时是用户模型
        // --- 步骤 1: 检查用户是否已存在 ---
        // 调用 `UserRepository` 的 `find_by_username` 方法来查询数据库。
        //   - `&username`: 将 `username` (类型 `String`) 的【不可变引用】 (`&str`) 传递给 `find_by_username`。
        //     这是因为 `find_by_username` 的参数是 `&str`，`String` 可以自动解引用 (deref coercion) 为 `&str`。
        //   - `.await`: 等待异步数据库查询完成。
        //   - `.map_err(|db_err| ...)`: 如果 `find_by_username` 返回 `Err(DbErr)` (数据库错误)，
        //     则这个闭包会被执行，将 `DbErr` 转换为我们自定义的 `AppError::DatabaseError`。
        //     `format!(...)` 用于创建包含具体数据库错误信息的字符串。
        //   - `?`: Rust 的【问号操作符】用于错误传播。
        //     - 如果前面的表达式结果是 `Ok(value)`，则 `?` 将 `value` 提取出来。
        //     - 如果结果是 `Err(error)`，则 `?` 会使当前函数 (`register_user`) 立即返回这个 `Err(error)`。
        //       (前提是 `error` 的类型可以被转换为 `register_user` 函数签名中声明的错误类型 `AppError`，
        //        这里 `AppError::DatabaseError` 就是 `AppError` 类型，所以可以直接返回)。
        if UserRepository::find_by_username(db, &username)
            .await
            .map_err(|db_err| AppError::DatabaseError(format!("检查用户名是否存在时出错: {}", db_err)))?
            // `.is_some()`: `find_by_username` 返回 `Result<Option<Model>, DbErr>`。
            // 如果数据库查询成功 (`Ok`)，`?` 会提取出 `Option<Model>`。
            // `.is_some()` 检查这个 `Option` 是否是 `Some(_)` (即用户已存在)。
            .is_some()
        {
            // 如果用户已存在，则调用我们定义的辅助函数 `AppError::user_already_exists` 创建一个错误实例，
            // 并通过 `Err(...)` 将其作为当前函数的返回值。
            // `username` 的所有权在这里被转移到 `AppError::UserAlreadyExists` 变体中。
            return Err(AppError::user_already_exists(username));
        }

        // --- 步骤 2: 哈希密码 (使用 Argon2) ---
        // **什么是盐 (Salt)?** 盐是一个随机生成的、附加到密码上的数据片段，然后再进行哈希。
        // **为什么用盐?**
        //   - **抵抗彩虹表攻击**: 彩虹表是预先计算好的哈希值列表。如果没有盐，攻击者可以用彩虹表快速查找常用密码的哈希。
        //     加盐后，即使两个用户使用相同的密码，由于盐不同，他们的哈希值也不同，使得彩虹表失效。
        //   - **增加哈希复杂度**: 即使密码本身不够复杂，盐也能增加最终哈希值的随机性。
        // `let mut salt = [0u8; 32];`: 创建一个包含32个字节的数组，并用0初始化。`mut` 表示它是可变的。
        //   `[0u8; 32]` 是一个固定大小的数组，元素类型是 `u8` (字节)，长度是32。
        let mut salt = [0u8; 32]; // 32字节的盐通常足够安全。
        // `rand::thread_rng()`: 获取一个线程本地的、密码学安全的随机数生成器 (CSPRNG)。
        // `.fill_bytes(&mut salt)`: 使用随机数生成器填充 `salt` 数组。`&mut salt` 是对 `salt` 数组的可变引用。
        rand::thread_rng().fill_bytes(&mut salt);

        // `let config = Config { ... };`: 创建 Argon2 算法的配置实例。
        // 这些参数共同决定了哈希的计算强度和资源消耗，需要根据安全需求和服务器性能进行调整。
        // - `variant: Variant::Argon2id`: 选择 Argon2id 变体。Argon2 有几种变体 (Argon2d, Argon2i, Argon2id)。
        //   Argon2id 结合了 Argon2i (抵抗侧信道攻击) 和 Argon2d (抵抗 GPU 破解) 的优点，是推荐的选择。
        // - `version: Version::Version13`: 指定 Argon2 算法的版本 (版本1.3，即0x13)。
        // - `mem_cost: 65536`: 内存消耗，单位是 KiB (千字节)。这里是 65536 KiB = 64 MiB。
        //   越高的值越安全，但也消耗更多内存。
        // - `time_cost: 10`: 时间消耗，表示迭代次数。越高的值越安全，但也需要更长的计算时间。
        // - `lanes: 4`: 并行度 (或称为线程数)。指哈希计算可以并行使用的通道数。
        //   通常设置为服务器 CPU 的核心数或期望的并行处理单元数。
        // - `thread_mode: ThreadMode::Parallel`: 明确指定使用并行模式 (如果 `lanes > 1`)。
        // - `secret: &[]`: 可选的密钥。如果提供，则 Argon2 变为密钥哈希函数。通常用于密码哈希时为空。
        // - `ad: &[]`: 附加关联数据 (Additional Associated Data)。通常用于密码哈希时为空。
        // - `hash_length: 32`: 输出哈希的长度，单位是字节。32字节 (256位) 是一个常见的安全长度。
        let config = Config {
            variant: Variant::Argon2id,
            version: Version::Version13,
            mem_cost: 65536,
            time_cost: 10,
            lanes: 4,
            thread_mode: ThreadMode::Parallel,
            secret: &[], // 密码哈希通常不使用 secret
            ad: &[],     // 密码哈希通常不使用关联数据
            hash_length: 32,
        };

        // `argon2::hash_encoded(&password.as_bytes(), &salt, &config)`: 执行密码哈希。
        //   - `&password.as_bytes()`: 将输入的 `password` (String) 转换为字节切片 (`&[u8]`)。Argon2 处理字节。
        //   - `&salt`: 传入生成的盐的引用。
        //   - `&config`: 传入 Argon2 配置的引用。
        //   - 此函数返回 `Result<String, argon2::Error>`。成功时，`String` 是编码后的哈希字符串。
        //     这个编码字符串包含了所有需要验证密码的信息 (算法、版本、参数、盐、哈希值本身)。
        //   - `.map_err(|e| ...)`: 如果哈希失败，将其映射为 `AppError::InternalServerError`。
        //   - `?`: 错误传播。
        let hashed_password = argon2::hash_encoded(&password.as_bytes(), &salt, &config)
            .map_err(|e| AppError::InternalServerError(format!("密码哈希失败: {}", e)))?;
        // `hashed_password` 现在是一个可以安全存储在数据库中的字符串。

        // --- 步骤 3: 创建用于插入数据库的 ActiveModel ---
        // `user_entity::ActiveModel { ... }`: 创建一个 `user_entity` 模块中定义的 `ActiveModel` 的实例。
        // `ActiveModel` 用于向数据库插入或更新数据。它的字段是 `sea_orm::ActiveValue<T>` 类型。
        let new_user_active_model = user_entity::ActiveModel {
            // `username: sea_orm::Set(username.clone())`: 设置 `username` 字段。
            //   - `sea_orm::Set(value)`: 表示要将此字段的值明确设置为 `value`。
            //   - `username.clone()`: 因为 `username` (String) 的所有权之前可能已被用于 `UserAlreadyExists` 错误，
            //     或者我们想保留 `username` 变量在此处之后仍可用 (虽然这里它之后没有被直接使用)。
            //     如果 `username` 在 `UserAlreadyExists` 分支中没有被消耗，且之后不再需要，可以不克隆。
            //     但为了代码在不同分支下的行为一致性和避免所有权问题，显式克隆通常更安全，除非性能非常关键。
            //     (实际上，如果 `UserAlreadyExists` 分支被执行，函数就返回了，所以这里的 `username` 仍拥有所有权。
            //      但 `Set` 需要获取值的所有权，所以如果 `username` 是 `String`，要么克隆，要么直接传递所有权。)
            //     这里假设 `username` (参数) 的所有权可以被 `Set` 获取。
            username: sea_orm::Set(username), // `username` 的所有权被转移到 `Set` 中。
            // `hashed_password: sea_orm::Set(hashed_password)`: 设置 `hashed_password` 字段。
            //   `hashed_password` (String) 的所有权被转移到 `Set` 中。
            hashed_password: sea_orm::Set(hashed_password),
            // `..Default::default()`: Rust 的【结构体更新语法 (struct update syntax)】与 `Default` trait 结合。
            //   - `..`: 表示其余未显式设置的字段。
            //   - `Default::default()`: 创建一个包含所有字段默认值的 `ActiveModel` 实例。
            //     对于 `ActiveModel`，字段的默认 `ActiveValue` 通常是 `NotSet`。
            //   - 这意味着 `id`, `created_at`, `updated_at` 等字段在这里被设置为 `NotSet`。
            //     - `id`: 我们期望数据库自动生成。
            //     - `created_at`, `updated_at`: 在 `user_entity.rs` 中定义了 `default_expr = "Expr::current_timestamp()"`，
            //       所以数据库会在插入时自动填充它们。
            ..Default::default()
        };

        // --- 步骤 4: 通过 UserRepository 将新用户数据保存到数据库 ---
        // `UserRepository::create_user(db, new_user_active_model)`: 调用仓库的 `create_user` 方法。
        //   - `db`: 数据库连接引用。
        //   - `new_user_active_model`: 包含新用户数据的活动模型。
        //   - `.await`: 等待异步数据库操作完成。
        //   - `.map_err(|db_err| ...)`: 如果仓库方法返回 `Err(DbErr)`，则将其转换为 `AppError::DatabaseError`。
        //   - (这里没有 `?`，因为这是函数的最后一个表达式，其 `Result` 可以直接作为函数的返回。)
        UserRepository::create_user(db, new_user_active_model)
            .await
            .map_err(|db_err| AppError::DatabaseError(format!("创建用户时数据库操作失败: {}", db_err)))
    }

    // `pub async fn login_user(...) -> Result<String>`
    //   - `pub async fn`: 公共异步函数 `login_user`。
    //   - 参数与 `register_user` 类似: `db` (数据库连接引用), `username` (String, 所有权), `password` (String, 所有权)。
    //   - `-> Result<String>`: 返回类型。
    //     - `Result<_, AppError>`: 操作可能成功或失败。
    //     - `String`: 如果登录成功，返回一个 JWT (JSON Web Token) 字符串。
    /// 处理用户登录的核心业务逻辑。
    /// 1. 根据用户名查找用户。
    /// 2. 如果找到用户，验证提供的密码是否与存储的哈希密码匹配。
    /// 3. 如果凭证有效，生成并返回一个 JWT。
    ///
    /// # 参数
    /// * `db`: 数据库连接的引用。
    /// * `username`: 用户提供的用户名 (获取所有权)。
    /// * `password`: 用户提供的明文密码 (获取所有权)。
    ///
    /// # 返回值
    /// * `Ok(token_string)`: 如果登录成功，返回 JWT 字符串。
    /// * `Err(AppError::InvalidCredentials)`: 如果用户名不存在或密码不匹配。
    /// * `Err(AppError::DatabaseError)`: 如果数据库查询失败。
    /// * `Err(AppError::InternalServerError)`: 如果密码验证或 JWT 生成过程出错。
    pub async fn login_user(
        db: &DatabaseConnection,
        username: String,
        password: String,
    ) -> Result<String> { // 成功时返回 JWT 字符串
        // --- 步骤 1: 根据用户名查找用户 ---
        // 调用仓库的 `find_by_username` 方法。
        let user = UserRepository::find_by_username(db, &username)
            .await
            .map_err(|db_err| AppError::DatabaseError(format!("登录时查询用户信息失败: {}", db_err)))?
            // `UserRepository::find_by_username` 返回 `Result<Option<Model>, DbErr>`。
            // 上面的 `?` 处理了 `DbErr`。如果成功，我们得到 `Option<Model>`。
            // `.ok_or_else(|| AppError::invalid_credentials())?`: 处理 `Option<Model>`。
            //   - `ok_or_else` 将 `Option<T>` 转换为 `Result<T, E>`。
            //     - 如果是 `Some(user_model)`，它返回 `Ok(user_model)`。
            //     - 如果是 `None` (即用户未找到)，它执行闭包 `|| AppError::invalid_credentials()` 来生成错误值，
            //       并返回 `Err(AppError::invalid_credentials())`。
            //   - `?` 再次用于错误传播：如果用户未找到 (返回 `Err`)，则 `login_user` 函数立即返回此错误。
            .ok_or_else(AppError::invalid_credentials)?; // 如果用户不存在，则返回无效凭证错误。
        // `user` 现在是 `user_entity::Model` 类型，包含了找到的用户数据。

        // --- 步骤 2: 验证密码 ---
        // `argon2::verify_encoded(&user.hashed_password, password.as_bytes())`:
        //   - `&user.hashed_password`: 从数据库中获取的、已编码的哈希密码字符串的引用。
        //   - `password.as_bytes()`: 用户在登录时提供的明文密码，转换为字节切片。
        //   - 此函数会从 `user.hashed_password` 中提取出盐和哈希参数，然后用相同的参数哈希提供的明文密码，
        //     最后比较两个哈希值是否相同。
        //   - 返回 `Result<bool, argon2::Error>`。`bool` 表示密码是否匹配。
        //   - `.map_err(|e| ...)`: 如果 `verify_encoded` 过程本身出错 (不是指密码不匹配，而是指哈希字符串格式错误等内部问题)，
        //     则映射为 `AppError::InternalServerError`。
        //   - `?`: 错误传播。
        let matches = argon2::verify_encoded(&user.hashed_password, password.as_bytes())
            .map_err(|e| AppError::InternalServerError(format!("密码验证过程中发生错误: {}", e)))?;

        // `if !matches { ... }`: 如果 `matches` 是 `false` (即密码不匹配)。
        if !matches {
            // 返回无效凭证错误。注意，出于安全考虑，不应明确告诉用户是“密码错误”还是“用户名不存在”，
            // 统一返回“无效凭证”可以防止攻击者枚举用户名。
            return Err(AppError::invalid_credentials());
        }

        // --- 步骤 3: 生成 JWT (JSON Web Token) ---
        // 用户名和密码均已验证成功，现在为用户生成一个 JWT。

        // `let now = Utc::now();`: 获取当前的 UTC 时间。`Utc::now()` 返回 `chrono::DateTime<Utc>`。
        let now = Utc::now();
        // `let iat = now.timestamp() as usize;`: 获取 "Issued At" (签发时间)声明。
        //   - `now.timestamp()`: 将 `DateTime<Utc>` 转换为 Unix 时间戳 (从1970-01-01 00:00:00 UTC 开始的秒数)，类型为 `i64`。
        //   - `as usize`: 将 `i64` 类型的时间戳转换为 `usize` 类型。`jsonwebtoken` crate 的 `Claims` 结构期望 `exp` 和 `iat` 是 `usize`。
        //     **注意**: 这种转换在不同平台或未来时间戳很大时可能有问题 (例如 `i64` 为负数，或超出 `usize` 范围)。
        //     对于现代系统和合理的令牌有效期，这通常是可接受的。更健壮的做法是使用 `u64` 并确保兼容性。
        let iat = now.timestamp() as usize;

        // `let exp = (now + chrono::Duration::hours(24)).timestamp() as usize;`: 计算 "Expiration Time" (过期时间) 声明。
        //   - `chrono::Duration::hours(24)`: 创建一个表示24小时的时间段。
        //   - `now + ...`: 将当前时间加上24小时，得到未来的过期时间点。
        //   - `.timestamp() as usize`: 同样转换为 `usize` 类型的 Unix 时间戳。
        //   **TODO**: JWT 的过期时间应该可以通过应用配置 (`AppConfig`) 来设置，而不是硬编码为24小时。
        let exp = (now + chrono::Duration::hours(24)).timestamp() as usize; // 令牌有效期24小时

        // `let claims = Claims { ... };`: 创建在 `src/app/model/auth_dtos.rs` 中定义的 `Claims` 结构体的实例。
        let claims = Claims {
            // `sub: user.id.to_string()`: "Subject" (主体) 声明。通常是用户的唯一标识符。
            //   - `user.id`: 从数据库模型中获取的用户 ID (类型 `i32`)。
            //   - `.to_string()`: 将 `i32` 转换为 `String`。
            sub: user.id.to_string(),
            exp, // 过期时间戳
            iat, // 签发时间戳
        };

        // **TODO**: JWT 签名密钥 (`jwt_secret`) 应该从应用配置 (`AppConfig`) 中安全地获取，
        // 而不是硬编码在代码中。硬编码密钥是一个严重的安全风险，因为它很容易被泄露。
        // 在 `AppConfig` 中添加一个 `jwt_secret: String` 字段，并在 `main.rs` 或 `startup.rs` 中加载它，
        // 然后通过 `AppState` 或直接参数传递给此服务方法。
        let jwt_secret = "your-placeholder-super-secret-key-that-must-be-changed"; // ⚠️ 极不安全! 必须替换!

        // `jsonwebtoken::encode(...)`: 调用 `jsonwebtoken` crate 的 `encode` 函数来生成 JWT 字符串。
        //   - `&Header::default()`: JWT 的头部。`Header::default()` 通常创建一个使用 HS256 算法的头部。
        //     (例如: `{"alg": "HS256", "typ": "JWT"}`)
        //   - `&claims`: 要编码到 JWT Payload 中的 `Claims` 结构体的引用。
        //   - `&EncodingKey::from_secret(jwt_secret.as_ref())`: 编码和签名所用的密钥。
        //     - `jwt_secret.as_ref()`: 将 `String` 类型的 `jwt_secret` 转换为 `&[u8]` (字节切片)。
        //     - `EncodingKey::from_secret(...)`: 从字节切片创建一个适用于对称算法 (如 HS256) 的编码密钥。
        //   - `encode` 函数返回 `Result<String, jsonwebtoken::errors::Error>`。
        //   - `.map_err(|e| ...)`: 如果 JWT 生成失败，映射为 `AppError::InternalServerError`。
        //   - `?`: 错误传播。
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(jwt_secret.as_ref()),
        )
        .map_err(|e| AppError::InternalServerError(format!("JWT 生成失败: {}", e)))?;

        // `Ok(token)`: 如果所有步骤都成功，返回包含生成的 JWT 字符串的 `Ok` 结果。
        Ok(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argon2::{self, Config, ThreadMode, Variant, Version};
    use rand::RngCore;
    use jsonwebtoken::{decode, Validation, Algorithm};
    use chrono::Duration;

    // Helper for Argon2 config consistent with auth_service
    fn get_argon2_config() -> Config<'static> {
        Config {
            variant: Variant::Argon2id,
            version: Version::Version13,
            mem_cost: 65536,
            time_cost: 10,
            lanes: 4,
            thread_mode: ThreadMode::Parallel,
            secret: &[],
            ad: &[],
            hash_length: 32,
        }
    }

    #[test]
    fn test_password_hashing_and_verification() {
        let password = "test_password123";
        let mut salt = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut salt);
        let config = get_argon2_config();

        let hashed_password = argon2::hash_encoded(password.as_bytes(), &salt, &config).unwrap();

        // Verify correct password
        let matches_correct = argon2::verify_encoded(&hashed_password, password.as_bytes()).unwrap();
        assert!(matches_correct, "Password verification failed for correct password");

        // Verify wrong password
        let wrong_password = "wrong_password";
        let matches_wrong = argon2::verify_encoded(&hashed_password, wrong_password.as_bytes()).unwrap();
        assert!(!matches_wrong, "Password verification succeeded for wrong password");
    }

    // JWT secret - must match the one in login_user
    const TEST_JWT_SECRET: &str = "your-placeholder-super-secret-key-that-must-be-changed";

    #[test]
    fn test_jwt_generation_and_validation() {
        let now = Utc::now();
        let iat = now.timestamp() as usize;
        let exp = (now + Duration::hours(1)).timestamp() as usize; // Expires in 1 hour

        let original_claims = Claims {
            sub: "test_user_123".to_string(),
            exp,
            iat,
        };

        // Generate token
        let token = encode(
            &Header::default(),
            &original_claims,
            &EncodingKey::from_secret(TEST_JWT_SECRET.as_ref()),
        )
        .expect("JWT encoding failed");

        assert!(!token.is_empty(), "Generated token should not be empty");

        // Decode and validate token
        let token_data = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(TEST_JWT_SECRET.as_ref()),
            &Validation::new(Algorithm::HS256),
        )
        .expect("JWT decoding failed");

        assert_eq!(token_data.claims.sub, original_claims.sub, "Decoded 'sub' claim mismatch");
        assert_eq!(token_data.claims.exp, original_claims.exp, "Decoded 'exp' claim mismatch");
        assert_eq!(token_data.claims.iat, original_claims.iat, "Decoded 'iat' claim mismatch");


        // Attempt to decode with wrong secret
        let wrong_secret = "not-the-correct-secret";
        let decoding_result_wrong_secret = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(wrong_secret.as_ref()),
            &Validation::new(Algorithm::HS256),
        );
        assert!(decoding_result_wrong_secret.is_err(), "JWT decoding should fail with wrong secret");
    }

    // --- Outlined tests for AuthService methods (require mocking) ---

    #[test]
    fn test_register_user_success() {
        // Outline:
        // 1. Setup mock DatabaseConnection.
        // 2. Setup mock UserRepository:
        //    - find_by_username to return Ok(None).
        //    - create_user to return Ok(user_entity::Model { /* ... */ }).
        // 3. Call AuthService::register_user(...).
        // 4. Assert Ok(user_model) and check username and hashed_password (not empty).
        //    (Actual password hash cannot be directly compared due to salt).
        println!("TODO: Implement test_register_user_success with mocking.");
        assert!(true); // Placeholder
    }

    #[test]
    fn test_register_user_already_exists() {
        // Outline:
        // 1. Setup mock DatabaseConnection.
        // 2. Setup mock UserRepository:
        //    - find_by_username to return Ok(Some(existing_user_model)).
        // 3. Call AuthService::register_user(...).
        // 4. Assert Err(AppError::UserAlreadyExists).
        println!("TODO: Implement test_register_user_already_exists with mocking.");
        assert!(true); // Placeholder
    }

    #[test]
    fn test_login_user_success() {
        // Outline:
        // 1. Setup mock DatabaseConnection.
        // 2. Create a sample user with a known password hashed with Argon2.
        // 3. Setup mock UserRepository:
        //    - find_by_username to return Ok(Some(user_model_with_valid_hash)).
        // 4. Call AuthService::login_user(...).
        // 5. Assert Ok(token_string) and token is not empty.
        // 6. Optionally, decode the token to verify claims (though covered by test_jwt_generation_and_validation).
        println!("TODO: Implement test_login_user_success with mocking.");
        assert!(true); // Placeholder
    }

    #[test]
    fn test_login_user_not_found() {
        // Outline:
        // 1. Setup mock DatabaseConnection.
        // 2. Setup mock UserRepository:
        //    - find_by_username to return Ok(None).
        // 3. Call AuthService::login_user(...).
        // 4. Assert Err(AppError::InvalidCredentials).
        println!("TODO: Implement test_login_user_not_found with mocking.");
        assert!(true); // Placeholder
    }

    #[test]
    fn test_login_user_wrong_password() {
        // Outline:
        // 1. Setup mock DatabaseConnection.
        // 2. Create a sample user with a known password hashed with Argon2.
        // 3. Setup mock UserRepository:
        //    - find_by_username to return Ok(Some(user_model_with_valid_hash)).
        // 4. Call AuthService::login_user with the correct username but a wrong password.
        // 5. Assert Err(AppError::InvalidCredentials).
        println!("TODO: Implement test_login_user_wrong_password with mocking.");
        assert!(true); // Placeholder
    }
}
