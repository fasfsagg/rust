// app/model/mod.rs
//
// 【模型模块声明文件】
// 这个文件声明并导出模型子模块，使它们在应用程序其他部分可见。
// 在Rust中，mod.rs文件用于声明和组织子模块，类似于目录的"入口点"。

// /----------------------------------------------------------------\
// |                      【模块功能图示】                        |
// |----------------------------------------------------------------|
// |      crate::app::model (本模块)                               |
// |          |                                                    |
// |          +-- pub mod task; (声明子模块)                       |
// |          |      |                                             |
// |          |      +--> src/app/model/task.rs (子模块文件)         |
// |          |             - Task                                  |
// |          |             - CreateTaskPayload                     |
// |          |             - UpdateTaskPayload                     |
// |          |                                                    |
// |          +-- pub mod user_entity; (声明子模块)                 |
// |          |      |                                             |
// |          |      +--> src/app/model/user_entity.rs (子模块文件) |
// |          |             - UserEntity                             |
// |          |             - CreateUserEntityPayload               |
// |          |             - UpdateUserEntityPayload               |
// |          |                                                    |
// |          +-- pub mod auth; (声明子模块)                        |
// |          |      |                                             |
// |          |      +--> src/app/model/auth.rs (子模块文件)         |
// |          |             - AuthEntity                             |
// |          |             - CreateAuthEntityPayload               |
// |          |             - UpdateAuthEntityPayload               |
// |          |                                                    |
// |          +-- pub use task::*; (重新导出子模块公共项)            |
// |                 |                                             |
// |                 +--> 使外部可以直接 use crate::app::model::Task |
// \----------------------------------------------------------------/
//
// 文件路径: src/app/model/mod.rs
//
// 【模块核心职责】
// 这个 `mod.rs` 文件扮演着 `src/app/model/` 目录的【入口和组织者】的角色。
// 它的主要职责是：
// 1. **声明子模块**: 告诉 Rust 编译器存在哪些子模块（即同目录下的 `.rs` 文件或子目录）。
// 2. **控制可见性与导出**: 决定哪些子模块或子模块中的项（结构体、枚举、函数等）可以被 `model` 模块之外的代码访问。
//
// 【Rust 模块系统基础】
// - **模块 (Module)**: Rust 用模块来组织代码、控制作用域和路径。
// - **`mod.rs`**: 当一个目录下同时包含 `mod.rs` 和其他 `.rs` 文件（或子目录）时，`mod.rs` 文件定义了该目录对应的模块，而其他 `.rs` 文件或子目录则被视为该模块的【子模块】。
// - **`pub mod <子模块名>;`**: 在 `mod.rs` 中，这行代码【声明】了一个名为 `<子模块名>` 的子模块。Rust 会查找同目录下的 `<子模块名>.rs` 文件或 `<子模块名>/mod.rs` 文件作为该子模块的内容。
// - **`pub use <路径>;`**: 这是【重新导出 (re-exporting)】。它使得其他模块可以通过【当前模块的路径】来访问 `<路径>` 指向的内容，从而简化访问路径，隐藏内部组织结构。
//
// 【本文件具体作用】
// 1. 通过 `pub mod task;` 声明了 `task` 这个子模块（对应 `task.rs` 文件）。 `pub` 关键字使得 `task` 模块本身可以被 `model` 模块之外的代码访问（如果需要的话，例如 `use crate::app::model::task;`）。
// 2. 通过 `pub mod user_entity;` 声明了 `user_entity` 这个子模块（对应 `user_entity.rs` 文件）。 `pub` 关键字使得 `user_entity` 模块本身可以被 `model` 模块之外的代码访问（如果需要的话，例如 `use crate::app::model::user_entity;`）。
// 3. 通过 `pub mod auth;` 声明 `auth` 子模块 (对应 `auth.rs` 文件).
// 4. 通过 `pub use task::*;` 将 `task` 模块中所有【公共的 (public)】项（如 `Task`, `CreateTaskPayload`, `UpdateTaskPayload` 结构体）直接【提升】到 `model` 模块的作用域中。
//    - 这意味着其他代码可以直接写 `use crate::app::model::Task;` 来导入 `Task` 结构体，而不需要写更长的 `use crate::app::model::task::Task;`。
//    - `*` 是通配符，表示导出 `task` 模块内所有公共项。

// --- 声明子模块 ---
// `pub mod task;`
// 【作用】: 声明存在一个名为 `task` 的公共子模块。
// 【查找规则】: Rust 编译器会查找 `src/app/model/task.rs` 文件。
// 【可见性】: `pub` 使得 `task` 模块本身可以被外部访问 (虽然我们通常通过重新导出的项来访问其内容)。[[关键语法要素: pub, mod]]
pub mod task;
pub mod user_entity;
pub mod auth;

// --- 重新导出公共项 ---
// `pub use task::*;`
// 【作用】: 将 `task` 模块中所有 `pub` 的项（结构体、枚举、函数等）引入到当前的 `model` 模块作用域，并使它们也成为 `pub`。
// 【效果】: 简化外部模块的导入路径。
// 【举例】: 如果 `task.rs` 中定义了 `pub struct Task { ... }`，那么其他文件现在可以通过 `use crate::app::model::Task;` 来使用它。
// 【`*` 通配符】: 表示导出 `task` 模块内的所有公共项。有时为了更清晰，也会选择性地重新导出，例如 `pub use task::{Task, CreateTaskPayload};`。[[关键语法要素: pub, use, * (glob)]]
pub use task::*;
pub use auth::*;
