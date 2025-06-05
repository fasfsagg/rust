// 文件路径: src/app/model/user_entity.rs

// /--------------------------------------------------------------------------------------------------\
// |                                【模块功能图示】 (user_entity.rs)                                  |
// |--------------------------------------------------------------------------------------------------|
// |                                                                                                  |
// |  [数据库 (SQLite)]                                                                                |
// |    - 表: `users`                                                                                  |
// |      - 列: id (INTEGER, PRIMARY KEY, AUTOINCREMENT)                                               |
// |      - 列: username (TEXT, UNIQUE, NOT NULL)                                                      |
// |      - 列: hashed_password (TEXT, NOT NULL)                                                       |
// |      - 列: created_at (DATETIME, NOT NULL, DEFAULT CURRENT_TIMESTAMP)                             |
// |      - 列: updated_at (DATETIME, NOT NULL, DEFAULT CURRENT_TIMESTAMP)                             |
// |         ^                                                                                        |
// |         | (通过 SeaORM 映射)                                                                       |
// |         V                                                                                        |
// |  [Rust 代码中的实体 (`user_entity.rs`)]                                                            |
// |    - `pub struct Model` (代表从数据库读取的一条用户记录)                                            |
// |      - `id: i32`                                                                                  |
// |      - `username: String`                                                                         |
// |      - `hashed_password: String`                                                                  |
// |      - `created_at: DateTimeUtc`                                                                  |
// |      - `updated_at: DateTimeUtc`                                                                  |
// |    - `pub struct ActiveModel` (用于创建/更新用户记录)                                               |
// |         ^                                                                                        |
// |         | (在应用逻辑中使用，例如 Service 层、Repository 层)                                          |
// |         V                                                                                        |
// |  [应用程序其他部分] (例如: `auth_service.rs`, `user_repository.rs`)                                |
// |    - 使用 `Model` 来读取和表示用户数据。                                                            |
// |    - 使用 `ActiveModel` 来创建新用户或更新现有用户。                                                  |
// |                                                                                                  |
// \--------------------------------------------------------------------------------------------------/
//
// 【模块核心职责】 (Module Core Responsibilities)
// 1. **定义数据结构 (Define Data Structure)**: 使用 Rust 结构体 `Model` 来精确描述数据库中 `users` 表的每一行数据是如何在程序中表示的。
//    这包括每个字段的名称、Rust 数据类型以及它们如何映射到数据库表的列。
// 2. **ORM 映射 (ORM Mapping)**: 利用 SeaORM 的宏 (macros) 和特性 (traits) 来建立 Rust 结构体 (`Model`, `ActiveModel`) 与数据库表 (`users`) 之间的映射关系。
//    这使得开发者可以用 Rust 代码来操作数据库记录，而不是直接编写 SQL 语句。
// 3. **数据一致性蓝图 (Blueprint for Data Consistency)**: 作为用户数据的“蓝图”，确保了程序中处理用户数据时的一致性和类型安全。
//    例如，`username` 字段被定义为 `String`，那么所有与用户名相关的操作都会期望或产生 `String` 类型的数据。
//
// 【关键技术点】 (Key Technologies)
// - **SeaORM (ORM 框架)**:
//   - `DeriveEntityModel` (派生宏): SeaORM 提供的核心宏，用于自动为结构体生成作为数据库实体所需的代码。
//   - `#[sea_orm(...)]` (属性宏): 用于在结构体字段上或结构体本身添加元数据，告诉 SeaORM如何将 Rust 代码映射到数据库表和列。
//     例如 `table_name`, `primary_key`, `auto_increment`, `unique`, `column_type`, `default_expr`。
//   - `EntityTrait`, `ModelTrait`, `ActiveModelTrait`: SeaORM 定义的 traits，提供了与数据库交互的各种方法 (如 `find()`, `insert()`, `update()`)。
//   - `ActiveModelBehavior`: 一个 trait，允许自定义 `ActiveModel` (用于插入/更新操作的模型) 的行为。
//   - `DateTimeUtc`: SeaORM 提供的用于处理 UTC 时间戳的类型，通常映射到数据库的 DATETIME 或 TIMESTAMP 类型。
// - **Rust 结构体 (`struct Model`)**: Rust 用来创建自定义数据类型的基本方式，将相关数据字段组织在一起。
// - **派生宏 (`#[derive(...)]`)**: Rust 的一种强大元编程特性，允许在编译时自动生成代码来实现某些 traits。
//   - `Serialize`, `Deserialize` (来自 `serde` crate): 用于将 Rust 结构体与 JSON (或其他格式) 相互转换，常用于 API 的请求和响应。
//   - `Clone`: 允许创建结构体实例的副本。
//   - `Debug`: 允许使用 `{:?}` 格式化打印结构体实例，便于调试。
//   - `PartialEq`, `Eq`: 允许比较结构体实例是否相等。
//   - `Default`: 允许创建一个具有默认值的结构体实例。
// - **Rust 数据类型**: 如 `i32` (32位有符号整数), `String` (可增长的 UTF-8 字符串)。

// --- 导入依赖 ---
// `use sea_orm::entity::prelude::*;`
//   - `use`: Rust 关键字，用于将其他模块或 crate 中的项引入当前作用域。
//   - `sea_orm::entity::prelude::*`: 这是 SeaORM 的一个“预导入”模块 (prelude)。
//     - `sea_orm`: 我们使用的 ORM crate 名称。
//     - `entity`: `sea_orm` crate 内部的一个模块，专门处理与实体相关的定义。
//     - `prelude`: 许多 Rust crate 会提供一个 `prelude` 模块，它重新导出了一些最常用或最重要的项 (如 traits, structs, enums, macros)，
//       这样用户只需要导入这个 `prelude` 就可以方便地使用这些核心功能，而无需单独导入每一个。
//     - `*`: 通配符 (glob operator)，表示导入 `prelude` 模块中所有公共的项。
//     这行代码使得我们可以直接使用像 `EntityTrait`, `ModelTrait`, `ColumnTrait`, `DeriveEntityModel`, `DateTimeUtc` 等 SeaORM 核心组件。
use sea_orm::entity::prelude::*;
// `use serde::{Deserialize, Serialize};`
//   - `serde`: 一个非常流行的 Rust crate，用于高效地序列化 (Serialization) 和反序列化 (Deserialization) Rust 数据结构。
//     - **序列化**: 将 Rust 数据结构 (如 `struct` 或 `enum`) 转换为某种数据交换格式 (如 JSON, Bincode, YAML 等)。
//     - **反序列化**: 将数据交换格式的内容转换回 Rust 数据结构。
//   - `{Deserialize, Serialize}`: 从 `serde` crate 中选择性地导入 `Deserialize` 和 `Serialize` 这两个核心 traits。
//     - `Serialize`: 如果一个类型实现了 `Serialize` trait，那么它的实例就可以被序列化。
//     - `Deserialize`: 如果一个类型实现了 `Deserialize` trait，那么它就可以从序列化的数据中被创建出来。
//     我们通过 `#[derive(Serialize, Deserialize)]` 宏来为 `Model` 结构体自动实现这两个 traits，
//     这样 `Model` 的实例就可以方便地与 JSON 格式相互转换 (例如，在 API 响应中返回用户信息时)。
use serde::{Deserialize, Serialize};

// `#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize, Default)]`
// 这是一个【派生宏 (derive macro)】的列表，它们会为下面的 `Model` 结构体自动生成代码来实现指定的 traits。
// - `Clone`: 实现 `std::clone::Clone` trait。
//   - 允许我们通过调用 `.clone()` 方法来创建 `Model` 实例的一个深拷贝副本。
//   - 例如: `let user_copy = original_user.clone();`
//   - 这在需要传递数据副本而不是所有权或引用时非常有用。
// - `Debug`: 实现 `std::fmt::Debug` trait。
//   - 允许我们使用 `{:?}` (普通调试) 或 `{:#?}` (美化调试) 格式化占位符来打印 `Model` 实例的内容。
//   - 例如: `println!("用户数据: {:?}", user_instance);`
//   - 对于调试和日志记录非常关键。
// - `PartialEq`, `Eq`: 实现 `std::cmp::PartialEq` 和 `std::cmp::Eq` traits。
//   - `PartialEq`: 允许使用 `==` 和 `!=` 操作符来比较两个 `Model` 实例是否“部分相等”。对于结构体，默认会比较所有字段。
//   - `Eq`: 是 `PartialEq` 的一个子集，表明相等关系是自反的、对称的和传递的 (即真正的等价关系)。
//   - 简单来说，它们允许你比较两个用户模型是否包含完全相同的数据。
// - `DeriveEntityModel`: 这是 SeaORM 提供的核心宏，它会将这个普通的 Rust `struct` 转换为一个 SeaORM 实体模型。
//   它会生成大量与数据库表交互所需的底层代码，例如实现 `EntityTrait`。
// - `Serialize`, `Deserialize` (来自 `serde`):
//   - `Serialize`: 允许 `Model` 实例被序列化为 JSON (或其他格式)。当我们需要在 API 响应中返回用户数据时，这很有用。
//   - `Deserialize`: 允许从 JSON (或其他格式) 创建 `Model` 实例。虽然对于从数据库读取的实体模型，反序列化不常直接使用 (通常是 `ActiveModel` 用于输入)，
//     但有时为了保持一致性或在特定场景下 (如从缓存读取) 可能会用到。
// - `Default`: 实现 `std::default::Default` trait。
//   - 允许通过调用 `Model::default()` 来创建一个具有默认值的 `Model` 实例。
//   - 对于结构体，通常每个字段都会被设置为其各自类型的默认值 (例如 `0` for `i32`, 空 `String` for `String`, `false` for `bool`)。
//   - 这在某些情况下可以简化实例的创建，特别是当某些字段可以稍后填充时。
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize, Default)]
// `#[sea_orm(table_name = "users")]`
// 这是 SeaORM 的一个【属性宏 (attribute macro)】，应用于 `Model` 结构体。
// - `sea_orm(...)`: 表示这是一个 SeaORM 相关的配置。
// - `table_name = "users"`: 指定这个 `Model` 结构体对应到数据库中的名为 "users" 的表。
//   SeaORM 会根据这个名称来生成 SQL 查询，例如 `SELECT * FROM users ...`。
pub struct Model {
    // `#[sea_orm(primary_key, auto_increment = true)]`
    // 这是一个应用于 `id` 字段的 SeaORM 属性宏，用于定义其在数据库表中的特性。
    // - `primary_key`: 标记 `id` 字段是表的主键。
    //   - **什么是主键?** 主键 (Primary Key) 是数据库表中的一列（或一组列），它的值能够唯一地标识表中的每一行数据。
    //     主键的值必须是唯一的，并且通常不能为空。它对于快速查找、数据完整性和表间关系至关重要。
    //     把主键想象成每个人的身份证号码，每个人都有一个，而且是唯一的。
    // - `auto_increment = true`: 指示数据库在插入新记录时自动为这个 `id` 字段生成一个唯一的、递增的数值。
    //   这意味着我们通常不需要在创建新用户时手动指定 `id`，数据库会处理它。
    // `pub id: i32,`: 定义一个公共字段 `id`，其 Rust 类型是 `i32` (32位有符号整数)。
    //   - `pub`: 表示这个字段可以从结构体外部直接访问。
    //   - `id`: 字段名，用户的唯一数字标识符。
    //   - `i32`: 数据类型。在 SQLite 中，这通常会映射到 `INTEGER` 类型。
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,

    // `#[sea_orm(unique, column_type = "Text")]`
    // 应用于 `username` 字段的 SeaORM 属性宏。
    // - `unique`: 约束条件，表示 `username` 字段的值在整个 `users` 表中必须是唯一的。
    //   数据库会阻止插入或更新导致用户名重复的记录。这对于确保每个用户有唯一的登录名很重要。
    // - `column_type = "Text"`: 显式指定此字段在数据库中对应的列类型为 "Text"。
    //   虽然 SeaORM 通常可以根据 Rust 类型 (`String`) 推断出合适的数据库类型 (如 `TEXT` 或 `VARCHAR`)，
    //   但显式指定可以提供更精确的控制，或在需要特定数据库类型特性时使用。
    //   SQLite 中的 `TEXT` 类型用于存储字符串。
    // `pub username: String,`: 定义公共字段 `username`，类型为 `String`。
    //   - `String`: Rust 中用于存储可变长度、UTF-8 编码的文本。
    //     在内存中，`String` 通常在栈上存储一个指向堆上实际字符数据的指针、当前字符串长度和已分配容量。
    //     (栈: [ptr, len, capacity] -> 堆: ["u", "s", "e", "r", "n", "a", "m", "e", ...])
    //   - 这是用户的登录名。
    #[sea_orm(unique, column_type = "Text")]
    pub username: String,

    // `#[sea_orm(column_type = "Text")]`
    // 应用于 `hashed_password` 字段。
    // - `column_type = "Text"`: 同样，指定数据库列类型为 "Text"。哈希后的密码通常是一长串字符，适合用文本类型存储。
    // `pub hashed_password: String,`: 定义公共字段 `hashed_password`，类型为 `String`。
    //   - 存储的是用户原始密码经过 Argon2 哈希算法处理后得到的哈希值，而不是明文密码。
    //   - **重要**: 绝不能在数据库中存储明文密码！哈希是单向的，很难从哈希值反推出原始密码。
    #[sea_orm(column_type = "Text")]
    pub hashed_password: String,

    // `#[sea_orm(default_expr = "Expr::current_timestamp()")]`
    // 应用于 `created_at` 和 `updated_at` 字段。
    // - `default_expr = "Expr::current_timestamp()"`: 指定一个默认值表达式。
    //   - `Expr::current_timestamp()`: 这是 SeaORM 提供的一种方式来表示“数据库当前的日期和时间戳”。
    //     当插入新记录且没有为这个字段提供值时，数据库会自动将该列的值设置为记录创建时的时间戳。
    //     这对于追踪记录的创建和修改时间非常有用。
    //     在 SQLite 中，这通常会使用 `CURRENT_TIMESTAMP` SQL 函数。
    // `pub created_at: DateTimeUtc,`: 定义公共字段 `created_at`，类型为 `DateTimeUtc`。
    //   - `DateTimeUtc`: SeaORM 提供的用于处理 UTC (协调世界时) 日期和时间的类型。
    //     它通常基于 `chrono::DateTime<Utc>`，并提供了与数据库时间类型的良好集成。
    //     存储记录的创建时间。
    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    pub created_at: DateTimeUtc,

    // `pub updated_at: DateTimeUtc,`: 定义公共字段 `updated_at`，类型为 `DateTimeUtc`。
    //   存储记录的最后修改时间。通常在创建记录时，其值与 `created_at` 相同。
    //   在更新记录时，应用逻辑（或数据库触发器，但本项目中是应用逻辑）需要负责更新此字段。
    //   (注意: `default_expr` 只在插入时生效，更新时需要手动或通过 `ActiveModelBehavior` 处理)
    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    pub updated_at: DateTimeUtc,
}
// 内存结构示意图 (Conceptual Memory Layout for user_entity::Model instance)
// 假设一个 Model 实例在内存中：
// +--------------------------+
// | user_model (on stack, if local variable, or heap if part of Vec/Box etc.) |
// +--------------------------+
// | id: i32 (e.g., 4 bytes)  | (直接存储整数值)
// |--------------------------|
// | username: String         | (本身在结构体中占约 24 bytes on 64-bit: ptr, len, capacity)
// |   (ptr) -------------------> [字符数据 "example_user" on heap] (堆上分配，长度可变)
// |--------------------------|
// | hashed_password: String  | (同上, 约 24 bytes on 64-bit)
// |   (ptr) -------------------> [哈希后的密码字符数据 on heap] (堆上分配，长度可变)
// |--------------------------|
// | created_at: DateTimeUtc  | (具体大小取决于其内部实现, 通常是 i64 (秒) + u32 (纳秒) 或类似结构, e.g., 12 bytes)
// |--------------------------|
// | updated_at: DateTimeUtc  | (同上, e.g., 12 bytes)
// +--------------------------+
// 总大小会是各字段大小之和，其中 String 类型自身大小固定，但它们指向的数据在堆上。


// `#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]`
// 为 `Relation` 枚举派生一些 traits。
// - `Copy`, `Clone`: 允许 `Relation` 实例被简单复制 (如果枚举的所有变体都不包含堆上数据，`Copy` 才可能)。
// - `Debug`: 允许调试打印。
// - `EnumIter`: SeaORM 提供，允许迭代枚举的所有变体。
// - `DeriveRelation`: SeaORM 核心宏，用于定义实体之间的关系。
// `pub enum Relation {}`: 定义一个公共枚举 `Relation`。
//   - **用途**: 在 SeaORM 中，`Relation` 枚举用于定义此实体（`User`）与其他实体之间的关系。
//     例如，如果用户可以有多篇文章 (Posts)，这里会定义一个 `Posts` 变体来表示这种一对多关系。
//   - **当前为空**: `User` 实体目前没有定义与其他实体的显式关系 (例如，没有 `Post` 实体与之关联)。
//     因此，这个 `Relation` 枚举是空的 `{}`。
//     如果未来添加了新的实体并与用户关联，就需要在这里定义这些关系。
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

// `impl ActiveModelBehavior for ActiveModel {}`
// - `ActiveModel`: SeaORM 中用于插入 (Create) 和更新 (Update) 操作的特殊类型的模型。
//   与 `Model` (通常用于读取数据) 不同，`ActiveModel` 的字段可以表示“要设置的值”或“不设置此值”。
// - `ActiveModelBehavior`: 这是 SeaORM 提供的一个 trait。通过实现这个 trait，可以自定义 `ActiveModel` 在执行
//   数据库操作（如 `save`, `insert`, `update`）之前或之后的行为。
//   例如，可以在保存前自动更新 `updated_at` 时间戳，或者执行一些验证逻辑。
// - `impl ActiveModelBehavior for ActiveModel {}`: 为 `user_entity::ActiveModel` 实现 `ActiveModelBehavior` trait。
//   - `{}` (空实现块): 表示我们没有添加任何自定义的行为，而是使用 SeaORM 为 `ActiveModel` 提供的默认行为。
//     例如，默认行为可能包括在插入新记录时处理 `#[sea_orm(default_expr = ...)]` 这样的默认值。
//     如果我们需要在每次更新用户时自动更新 `updated_at` 字段，我们就需要在这里实现 `before_save` 方法。
impl ActiveModelBehavior for ActiveModel {}

// `impl Model { ... }`
// 这个 `impl` 块是为 `Model` 结构体本身实现的关联函数或方法。
// 我们可以在这里添加一些自定义的辅助函数，方便操作 `Model` 实例或与其相关的 `ActiveModel`。
impl Model {
    // 例如，可以添加一个构造函数辅助方法来创建用于插入的 `ActiveModel`:
    // pub fn new_active_model(username: String, hashed_password: String) -> ActiveModel {
    //     ActiveModel {
    //         username: sea_orm::Set(username),
    //         hashed_password: sea_orm::Set(hashed_password),
    //         // id 会自动生成, created_at 和 updated_at 会使用默认表达式
    //         ..Default::default() // 其他字段使用默认值 (通常是 NotSet)
    //     }
    // }
    //
    // 当前实现中没有自定义方法，但保留了这个块以便将来扩展。
}
