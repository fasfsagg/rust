// 文件路径: src/app/state.rs

// /--------------------------------------------------------------------------------------------------\
// |                                 【模块功能图示】 (app/state.rs)                                   |
// |--------------------------------------------------------------------------------------------------|
// |                                                                                                  |
// |  [startup.rs - `init_app` 函数]                                                                    |
// |   1. `db_conn = db::establish_connection().await` (创建 `DatabaseConnection`)                      |
// |   2. `arc_db_conn = Arc::new(db_conn)` (将 `DatabaseConnection` 包装在 `Arc` 中)                   |
// |   3. `app_state = AppState { db_conn: arc_db_conn }` (创建 `AppState` 实例)                       |
// |      |                                                                                           |
// |      V (传递给 Axum Router)                                                                        |
// |  [Axum Router (`routes.rs`)]                                                                       |
// |   - `.with_state(app_state)` (整个应用共享此 `AppState` 实例的克隆)                                |
// |      |                                                                                           |
// |      V (当请求到达某个 Handler 时)                                                                   |
// |  [Axum 请求处理函数 (Controller Handler)]                                                          |
// |   - `State(state): State<AppState>` (通过 Axum 的 `State` 提取器安全地访问 `AppState` 的克隆)        |
// |   - `let db_ref = &state.db_conn;` (获取对 `Arc<DatabaseConnection>` 的引用)                         |
// |   - (现在 Handler 可以使用 `db_ref` 来执行数据库操作，例如传递给服务层或仓库层)                        |
// |                                                                                                  |
// \--------------------------------------------------------------------------------------------------/
//
// 【模块核心职责】 (Module Core Responsibilities)
// 1. **定义共享状态结构 (`AppState`)**: `AppState` 结构体是本模块的核心，它定义了需要在整个 Axum Web 应用中
//    被多个请求处理函数 (handlers) 安全共享的数据。
// 2. **封装共享资源**: 在本项目中，`AppState` 主要用于封装数据库连接池 (`sea_orm::DatabaseConnection`)。
//    通过将其包装在 `std::sync::Arc` 中，可以确保数据库连接池在异步和多线程的 Axum 环境中被安全、高效地共享。
// 3. **支持 Axum 状态管理**: `AppState` 结构体通过派生 `Clone` trait，满足了 Axum `State` 提取器的要求，
//    使得 Axum 能够将应用状态的克隆副本注入到各个请求处理函数中。
//
// 【关键技术点】 (Key Technologies)
// - **Rust 结构体 (`struct AppState`)**: 用于定义自定义的 `AppState` 数据类型。
// - **`std::sync::Arc` (原子引用计数智能指针)**: 这是实现跨线程安全共享数据的核心机制。
//   它允许多个所有者共同“拥有”堆上的同一份数据，并通过原子操作来管理引用计数，确保数据在没有任何引用时才被清理。
// - **`sea_orm::DatabaseConnection`**: SeaORM 提供的数据库连接（池）类型，用于与数据库进行交互。
// - **Axum `State` 提取器 (`axum::extract::State`)**: Axum 提供的一种机制，允许请求处理函数安全地访问在应用级别共享的状态 (`AppState`)。
// - **`Clone` Trait (`#[derive(Clone)]`)**: Axum 要求共享状态必须实现 `Clone` trait。对于包含 `Arc` 的结构体，
//   克隆操作通常是廉价的（只增加引用计数）。

// --- 导入依赖 ---
// `use sea_orm::DatabaseConnection;`
//   - 从 `sea_orm` crate 中导入 `DatabaseConnection` 类型。
//   - `DatabaseConnection` 代表一个与数据库建立的连接。在 SeaORM 中，这通常是一个数据库连接池的句柄，
//     它管理着多个实际的数据库连接，以提高并发请求处理的效率和性能。
//   - 我们将在 `AppState` 中存储这个连接（池），以便所有需要访问数据库的请求处理函数都能共享它。
use sea_orm::DatabaseConnection;
// `use std::sync::Arc;`
//   - 从 Rust 标准库 (`std`) 的同步模块 (`sync`) 中导入 `Arc` 类型。
//   - `Arc` 是 "Atomically Referenced Counter" (原子引用计数) 的缩写。它是一种智能指针 (smart pointer)，
//     用于在多线程环境下实现共享数据所有权。我们将在下面的 `AppState` 结构体中详细解释它的作用。
use std::sync::Arc;

// `#[derive(Clone)]`
// - 这是一个【派生宏 (derive macro)】，它告诉 Rust 编译器自动为 `AppState` 结构体实现 `std::clone::Clone` trait。
// - **为什么需要 `Clone`?**
//   - Axum 的状态管理机制 (`.with_state()` 和 `State<T>` 提取器) 要求共享的状态类型 `T` 必须实现 `Clone` trait。
//   - 当一个 HTTP 请求进入，并且路由到的处理函数需要访问共享状态时，Axum 会为该处理函数提供一个状态的【克隆副本】。
//   - 这样做是为了确保每个请求处理（可能在不同的线程上并发执行）都能独立、安全地访问状态数据，而不会发生数据竞争或生命周期问题。
// - **`AppState` 的克隆行为**:
//   - 当 `AppState` 被克隆时，其内部字段也会被克隆。
//   - 对于 `db_conn: Arc<DatabaseConnection>` 字段，克隆 `Arc` 是一个非常廉价的操作。它并不会复制底层的 `DatabaseConnection` 数据
//     （这可能是一个复杂的连接池对象，复制成本很高或根本不可复制）。相反，克隆 `Arc` 只是创建一个新的指向相同堆上数据的 `Arc` 指针，
//     并以原子方式（线程安全地）增加该数据的引用计数。
//   - 因此，即使 `AppState` 被频繁克隆，共享的 `DatabaseConnection` 实例仍然是同一个，只是有更多的 `Arc` 指针指向它。
#[derive(Clone)]
// `pub struct AppState { ... }`: 定义一个公共的 (public) 结构体 `AppState`。
//   - `pub`: 表示这个结构体可以被项目中的其他模块（如 `startup.rs`, `main.rs`, 以及 `app/controller/` 下的模块）访问。
//   - `struct`: 关键字，用于定义结构体。`AppState` 将作为我们应用范围内的共享状态容器。
/// 应用程序共享状态结构体 (Application State Struct)
///
/// 【用途】: 封装需要在整个应用程序（特别是不同的请求处理函数之间）共享的数据。
///          在本项目中，它主要用于共享数据库连接池。
/// 【生命周期】: 通常在应用程序启动时 (如 `startup.rs` 中的 `init_app` 函数) 创建一次 `AppState` 的实例。
///             然后，这个实例（或其 `Arc` 包裹的内部数据的引用）会被传递给 Axum 的 `Router`，
///             并通过 Axum 的 `State` 提取器提供给各个请求处理函数。
/// 【共享机制】: Axum 要求共享状态必须实现 `Clone` trait。当请求到达时，Axum 会为处理该请求的 Handler 克隆一份 `AppState`。
///             因此，`AppState` 内部的字段，特别是那些需要在线程间共享且可能不是 `Clone` 或克隆成本高昂的资源 (如数据库连接池)，
///             通常需要使用 `Arc` (原子引用计数智能指针) 来包裹，以实现安全高效的共享。
pub struct AppState {
    // `pub db_conn: Arc<DatabaseConnection>;`: 定义一个公共字段 `db_conn`。
    //   - `pub`: 表示这个字段可以从结构体外部直接访问 (例如 `app_state_instance.db_conn`)。
    //   - `db_conn`: 字段名，清晰地表明它存储的是数据库连接。
    //   - `Arc<DatabaseConnection>`: 字段的类型。这是理解共享状态的关键部分。
    //     - `DatabaseConnection`: 这是 `sea_orm` 提供的数据库连接类型（通常是一个连接池）。
    //       直接在多个线程/任务中共享可变的 `DatabaseConnection` 实例是不安全的，而且 `DatabaseConnection` 可能没有实现 `Clone` trait，
    //       或者即使实现了，克隆它（即创建一个全新的连接池）的成本也非常高。
    //     - **`Arc<T>` (Atomic Reference Counting - 原子引用计数)**:
    //       - `Arc` 是一种【智能指针】，它允许多个“所有者”安全地共享对同一份堆分配数据 (`T`) 的只读或可变（通过内部可变性，如 `Mutex`）访问。
    //         在这里，`T` 是 `DatabaseConnection`。
    //       - **共享所有权**: 当你克隆一个 `Arc<T>` 时 (`let new_arc = old_arc.clone();`)，你并不是在复制 `T` 本身，
    //         而是创建了另一个指向堆上相同 `T` 数据的 `Arc` 指针，并以【原子方式】（线程安全地）增加内部的引用计数。
    //         这意味着多个 `Arc` 指针可以同时“指向”并“拥有”同一个 `DatabaseConnection` 实例。
    //       - **生命周期管理**: 只有当最后一个指向数据的 `Arc` 指针被销毁时（引用计数降为零），堆上的 `DatabaseConnection` 数据才会被清理。
    //       - **线程安全**: "Atomic" 意味着引用计数的增减操作是线程安全的，可以在多线程环境中正确工作，这对于 Web 服务器（如 Axum，它通常使用多线程 Tokio 运行时来处理并发请求）至关重要。
    //       - **为什么不用 `Rc<T>`?** `std::rc::Rc` 也是一种引用计数智能指针，但它是非原子的，因此不适用于多线程共享。`Arc` 是其多线程安全的对应版本。
    //
    //     - **实际效果**: 通过将 `DatabaseConnection` 包装在 `Arc` 中，我们可以安全地将 `AppState`（及其包含的 `Arc<DatabaseConnection>`）
    //       克隆并传递给多个并发执行的 Axum 请求处理函数。所有这些处理函数最终都将共享对同一个底层数据库连接池的访问，
    //       而 `Arc` 负责管理这个共享访问的生命周期和线程安全。
    //
    //     **内存结构示意图 (Conceptual Memory Layout for AppState with Arc<DatabaseConnection>)**
    //     ```
    //     // AppState 实例 (例如在 `startup.rs` 中创建，然后 Axum 为每个需要它的 Handler 克隆它)
    //     // (假设 AppState 实例本身存储在栈上或作为 Axum 内部状态的一部分)
    //     //
    //     // app_state_instance_for_handler_1: AppState {
    //     //   db_conn: ArcSmartPointer1 { // 这是一个 Arc 指针 (通常包含指向数据和引用计数的指针)
    //     //                // 指向堆上的共享数据区域 -->-+
    //     //            }                                |
    //     // }                                          |
    //     //                                            |   +-----------------------------------------+
    //     // app_state_instance_for_handler_2: AppState { // | 共享数据区域 (在堆上 Heap Allocated)    |
    //     //   db_conn: ArcSmartPointer2 { //              | +---------------------------------------+ |
    //     //                // 指向堆上的共享数据区域 -->-+ | | 引用计数 (AtomicUsize) e.g., 3      | |
    //     //            }                                | | +---------------------------------------+ |
    //     // }                                          | | | DatabaseConnection 实例 (连接池数据) | |
    //     //                                            | | +---------------------------------------+ |
    //     // app_state_instance_for_handler_3: AppState { // |                                         |
    //     //   db_conn: ArcSmartPointer3 { //              | +-----------------------------------------+
    //     //                // 指向堆上的共享数据区域 -->-+
    //     //            }
    //     // }
    //     //
    //     // 当一个 Handler 完成，其持有的 AppState (及内部的 Arc) 被销毁时，对应 Arc 指针失效，
    //     // 引用计数会原子地减一。当引用计数变为 0 时，堆上的 DatabaseConnection 实例才会被清理。
    //     ```
    /// 数据库连接实例 (Database Connection Instance)，使用 `Arc` 进行共享。
    ///
    /// 【类型】: `Arc<DatabaseConnection>` (SeaORM 的数据库连接池被原子引用计数指针包裹)。
    /// 【共享】: `Arc` 使得 `DatabaseConnection` (通常是一个连接池) 可以在多个异步任务和线程
    ///         （例如并发处理的 HTTP 请求）之间被安全地共享。
    ///         克隆 `AppState`（或直接克隆此 `Arc` 字段）的成本很低，因为它只复制指针并增加引用计数，
    ///         而不是复制整个数据库连接池。
    pub db_conn: Arc<DatabaseConnection>,

    // 未来可以向 AppState 添加更多需要在整个应用中共享的资源，例如：
    // - `config: Arc<AppConfig>`: 如果配置也需要在 handler 中访问。
    // - `http_client: reqwest::Client`: 如果需要共享一个 HTTP 客户端实例。
    // - `template_engine: ...`: 如果使用模板引擎。
}

// 在 Axum 的请求处理函数 (handler) 中，可以通过 `axum::extract::State<AppState>` 提取器来访问这个共享状态：
//
// async fn my_handler(
//   State(app_state): State<AppState>, // Axum 会提供 AppState 的克隆副本
//   // ... 其他提取器 ...
// ) -> impl IntoResponse {
//   // 现在可以使用 app_state.db_conn (它是一个 Arc<DatabaseConnection>) 来访问数据库连接
//   // 例如，传递给服务层或仓库层的方法：
//   // let user_service = UserService::new(&app_state.db_conn); // 如果服务层需要直接引用
//   // my_service::do_something_with_db(&app_state.db_conn).await;
//
//   // ...
// }
//
// Axum 确保了这种状态访问是类型安全和线程安全的。
