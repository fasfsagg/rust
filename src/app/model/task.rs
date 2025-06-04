// /----------------------------------------------------------------\
// |                      【模块功能图示】                        |
// |----------------------------------------------------------------|
// |         +-------------------+      +-------------------+       |
// | 输入    | CreateTaskPayload | ---> |       Task        | 输出  |
// | (JSON)  | (用于创建请求)    |      | (核心任务数据)    | (JSON)|
// |         +-------------------+      +-------------------+       |
// |                                       /|\                      |
// |         +-------------------+        |                       |
// | 输入    | UpdateTaskPayload | -------+                       |
// | (JSON)  | (用于更新请求)    |                                |
// |         +-------------------+                                |
// \----------------------------------------------------------------/
//
// 文件路径: src/app/model/task.rs
//
// 【模块核心职责】
// 这个文件是应用程序的"模型"层的一部分，专门负责定义与"任务"相关的数据结构。
// 可以把它想象成是现实世界"任务"概念在代码中的【蓝图】或【定义】。
// 它只负责【定义数据的样子】。
//
// 【主要内容】
// 1.  `Task`:          核心的任务数据结构。
// 2.  `CreateTaskPayload`: 创建任务的请求数据。
// 3.  `UpdateTaskPayload`: 更新任务的请求数据。
//
// 【关键技术点】: `serde`, `Uuid`, `SystemTime`, `derive`, `Option<T>`
//
// 【面向初学者提示】: 结构体 (Struct), 字段 (Field), 类型 (Type), 函数 (Function), 模块 (Module)
//
// app/model/task.rs
//
// 【任务数据模型】
// 本模块定义了与任务相关的数据结构，包括：
// 1. 核心Task结构体 - 表示一个任务实体
// 2. 创建任务的请求载荷 (CreateTaskPayload)
// 3. 更新任务的请求载荷 (UpdateTaskPayload)
// 这些结构体都使用了serde的序列化/反序列化功能，用于JSON数据交换。

// --- 导入外部依赖 ---
// `use` 关键字用于将其他模块或库中的代码【引入】到当前作用域，以便我们可以使用它们。
// 类似于 Python 的 `import` 或 JavaScript 的 `require` / `import`。

// 导入 `serde` 库中的 `Deserialize` 和 `Serialize` 两个【特性 (Trait)】。
// - `Deserialize`: 让我们的结构体能够从 JSON 或其他格式【反序列化】(解析数据)。[[关键语法要素: use, Trait]]
// - `Serialize`: 让我们的结构体能够【序列化】为 JSON 或其他格式 (生成数据)。[[关键语法要素: use, Trait]]
// 这两个是构建 Web API 的基石，用于处理网络请求和响应中的数据。
use serde::{ Deserialize, Serialize };
// 导入 `uuid` 库中的 `Uuid` 类型。
// `Uuid` 用于生成和表示【全局唯一标识符 (Universally Unique Identifier)】。
// 这对于数据库记录、任务 ID 等需要唯一性的场景非常重要。
use uuid::Uuid;
// 导入标准库 `std::time` 模块下的 `SystemTime` 和 `UNIX_EPOCH`。
// - `SystemTime`: 提供访问【系统时钟】的功能，用于获取当前时间。[[Rust语法基础: 模块路径 std::]]
// - `UNIX_EPOCH`: 代表 Unix 纪元时间 (1970年1月1日午夜 UTC)，是计算时间戳的起点。
use std::time::{ SystemTime, UNIX_EPOCH };

/// 任务实体结构体 (Struct Definition)
///
/// 【核心概念】: 这是我们系统中"任务"这一核心概念的代码表示。
/// 每个 `Task` 结构体的【实例 (Instance)】就代表着一个具体的任务。
/// 比如，一个名为 "学习 Rust" 的任务，另一个名为 "购物" 的任务。
///
/// 【内存结构示意图 (简化)】
/// Task {
///   id:         [Uuid 数据, 通常是16字节的数字] --> 内存地址 A
///   title:      [String 数据, 指向堆上存储的文本] --> 内存地址 B (包含指针、长度、容量) --> "学习 Rust" (在堆上)
///   description:[Option<String> 数据] --> Option::Some(内存地址 C (包含指针、长度、容量) --> "详细描述" (在堆上)) 或 Option::None
///   completed:  [bool 数据, 1字节] --> true / false
///   created_at: [u64 数据, 8字节] --> 代表秒数的整数
///   updated_at: [u64 数据, 8字节] --> 代表秒数的整数
/// }
/// 注意: `String` 和 `Option<String>` 的实际数据存储在【堆 (Heap)】上，结构体本身存储指向这些数据的【指针】以及长度/容量信息。
///
/// 【`derive` 宏解释】: `#[derive(...)]` 是 Rust 的一个强大功能，它告诉编译器自动为我们生成一些代码。[[关键语法要素: derive 宏]]
/// - `Clone`: 让我们可以通过 `.clone()` 方法【复制】一个 `Task` 实例。[[Rust语法特性/概念: Trait]]
///            这在需要传递任务数据副本而不是移动所有权时很有用。
///            【所有权解释】: 默认情况下，将一个 `Task` 赋值给新变量或传递给函数会转移【所有权】，原变量失效。`.clone()` 创建一个全新的独立副本。
/// - `Debug`: 允许我们使用【调试格式化宏】 `println!("{:?}", task);` 来打印 `Task` 实例的内容，方便调试。[[Rust语法特性/概念: Trait]]
/// - `Serialize`: 启用 `serde` 库的功能，可以将 `Task` 实例【转换】成 JSON 字符串或其他格式，用于 API 响应。[[关键语法要素: derive 宏, Trait]]
/// - `Deserialize`: 启用 `serde` 库的功能，可以从 JSON 字符串或其他格式【解析】数据并【创建】 `Task` 实例。[[关键语法要素: derive 宏, Trait]]
///                  虽然 `Task` 主要用于响应，但有时也可能需要反序列化（例如从数据库加载）。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task { // `pub struct` 表示定义一个公共的结构体，可以在其他模块中使用。[[关键语法要素: pub, struct]]
    /// 任务唯一标识符 (Field Definition)
    /// 【类型】: `Uuid` (来自 `uuid` 库)
    /// 【作用】: 唯一识别一个任务，通常在数据库中用作主键。
    /// 【现实映射】: 就像每个人的身份证号码一样，是独一无二的。
    /// 【`pub`】: 表示这个字段是公共的，可以在结构体外部访问。[[关键语法要素: pub]]
    pub id: Uuid, // 字段名: 类型

    /// 任务标题 (Field Definition)
    /// 【类型】: `String` (Rust 内置的可增长的 UTF-8 字符串)
    /// 【作用】: 任务的名称或简要描述。
    /// 【现实映射】: 待办事项列表中的项目名称。
    /// 【内存】: `String` 在栈上存储指针、长度和容量，实际字符数据存储在堆上。
    pub title: String,

    /// 任务描述 (可选) (Field Definition)
    /// 【类型】: `Option<String>`
    /// 【作用】: 提供任务的详细信息，但这个信息是【可选的】(可能没有描述)。
    /// 【现实映射】: 待办事项的备注信息，可有可无。
    /// 【`Option<T>`】: Rust 处理【缺失值】的方式。 `None` 表示没有描述，`Some(String)` 表示有描述。[[Rust语法特性/概念: Option 枚举]]
    /// 【`serde` 属性】: `#[serde(skip_serializing_if = "Option::is_none")]` [[关键语法要素: 属性宏]]
    ///    - 这是一个 `serde` 的【属性宏】，用于定制序列化行为。
    ///    - `skip_serializing_if = "Option::is_none"`: 指示 `serde` 在【序列化】(转成 JSON) 时，如果这个字段的值是 `None`，则【跳过】它，不在最终的 JSON 输出中包含 `description` 键。这可以使 JSON 更紧凑。
    ///      例如，没有描述的任务序列化后是 `{"id": ..., "title": "...", "completed": ..., ...}` 而不是 `{"id": ..., "title": "...", "description": null, "completed": ..., ...}`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// 任务完成状态 (Field Definition)
    /// 【类型】: `bool` (布尔值，只能是 `true` 或 `false`)
    /// 【作用】: 标记任务是否已经完成。
    /// 【现实映射】: 待办事项前面的复选框是否被勾选。
    pub completed: bool,

    /// 任务创建时间 (Unix时间戳，秒) (Field Definition)
    /// 【类型】: `u64` (64 位无符号整数)
    /// 【作用】: 记录任务是何时被创建的。存储的是自 Unix 纪元 (1970-01-01 00:00:00 UTC) 以来的秒数。
    /// 【现实映射】: 任何记录的创建时间戳。
    /// 【选择 `u64`】: 足够大以存储未来的时间戳，且通常用于时间戳表示。
    pub created_at: u64,

    /// 任务最后更新时间 (Unix时间戳，秒) (Field Definition)
    /// 【类型】: `u64`
    /// 【作用】: 记录任务最后一次被修改的时间。
    /// 【现实映射】: 文件的"修改日期"。
    pub updated_at: u64,
}

// --- 关联函数 (Associated Functions) 和方法 (Methods) ---
// `impl Task { ... }` 代码块用于为 `Task` 结构体定义【关联函数】和【方法】。
// - **关联函数 (Associated Function)**: 直接通过结构体名称调用，如 `Task::new()`。通常用作构造函数或工具函数。[[编程基础概念: 函数]]
// - **方法 (Method)**: 通过结构体的实例调用，如 `my_task.update()`。第一个参数通常是 `self`, `&self`, 或 `&mut self`，表示对实例的操作。[[编程基础概念: 方法]]
impl Task {
    /// 创建一个新任务的【关联函数】(构造函数模式)
    ///
    /// 【功能】: 这是创建 `Task` 结构体实例的【标准方式】。它接收必要的任务信息，并自动处理 ID 生成和时间戳设置。
    /// 【编程概念】: 这类似于面向对象编程中的【构造函数 (Constructor)】，但 Rust 没有显式的构造函数关键字，通常使用 `new` 作为关联函数名。
    ///
    /// # 【参数 (Parameters)】 [[编程基础概念: 函数参数]]
    /// * `title: String` - 任务的标题。传入的是一个 `String` 类型的值。
    ///                     【所有权】: 调用者提供的 `title` 的所有权会被【转移】到这个函数内部，最终存储在新创建的 `Task` 中。
    /// * `description: Option<String>` - 任务的可选描述。
    ///                     【所有权】: 如果传入的是 `Some(description_string)`，`description_string` 的所有权也会被转移。
    /// * `completed: bool` - 任务的初始完成状态。
    ///                     【类型】: `bool` 是【基本类型 (Primitive Type)】，实现了 `Copy` 特性。[[Rust语法特性/概念: Copy Trait]]
    ///                     【所有权】: 传递 `bool` 值时，发生的是【拷贝 (Copy)】而不是【移动 (Move)】，调用者仍然可以使用原来的 `completed` 变量。
    ///
    /// # 【返回值 (Return Value)】 [[编程基础概念: 函数返回值]]
    /// * `-> Self` - 表示这个函数返回一个【调用者类型】的实例，在这里就是 `Task`。[[关键语法要素: Self]]
    ///             `Self` 是 `Task` 的别名，用在 `impl Task` 块内部。
    ///
    /// # 【示例 (Example Usage)】(这部分通常用于文档生成)
    /// ```rust
    /// // 创建一个标题为 "学习Rust"，描述为 "学习Axum框架"，状态为未完成的任务
    /// let task = Task::new("学习Rust".to_string(), Some("学习Axum框架".to_string()), false);
    /// // `to_string()` 用于从字符串字面量 (&str) 创建一个拥有所有权的 String
    /// ```
    pub fn new(title: String, description: Option<String>, completed: bool) -> Self {
        // `pub fn` 定义公共函数 [[关键语法要素: pub, fn]]
        // --- 获取当前时间戳 ---
        // 1. `SystemTime::now()`: 调用 `SystemTime` 的关联函数 `now` 获取当前的系统时间。[[Rust语法基础: 调用关联函数]]
        // 2. `.duration_since(UNIX_EPOCH)`: 计算从 Unix 纪元开始到 `now()` 所经过的【持续时间 (Duration)】。
        //    这可能返回一个 `Result`，因为如果 `now()` 的时间早于 `UNIX_EPOCH` (理论上不可能在现代系统发生)，会出错。
        // 3. `.expect("获取系统时间失败")`: 处理 `Result`。[[Rust语法特性/概念: Result 枚举, Error Handling]]
        //    - 如果 `duration_since` 成功 (返回 `Ok(duration)`)，`.expect` 会提取出里面的 `duration` 值。
        //    - 如果 `duration_since` 失败 (返回 `Err(error)`)，程序会【恐慌 (panic!)】，停止执行并打印提供的错误消息 `"获取系统时间失败"`。
        //      对于预期不会失败的操作，`expect` 是一种简洁的处理方式，但在生产代码中应更谨慎处理错误 (例如使用 `match` 或 `?` 操作符)。
        // 4. `.as_secs()`: 将 `Duration` 转换为【整数秒数 (u64)】。
        let now: u64 = SystemTime::now() // 获取当前时间点
            .duration_since(UNIX_EPOCH) // 计算距离纪元的时间差
            .expect("系统时间早于 UNIX EPOCH，这不应该发生") // 处理潜在错误
            .as_secs(); // 转换为 u64 秒数

        // --- 创建并返回 Task 实例 ---
        // 使用【结构体字面量 (Struct Literal)】语法创建 `Task` 实例。[[关键语法要素: 结构体字面量]]
        Self { // `Self` 指代 `Task` 类型
            // `id`: 调用 `Uuid` 的关联函数 `new_v4()` 生成一个版本 4 的 UUID (基于随机数)。[[Rust语法基础: 调用关联函数]]
            id: Uuid::new_v4(),
            // `title`: 这是【字段初始化简写 (Field Init Shorthand)】。[[关键语法要素: 字段初始化简写]]
            //          因为参数名 `title` 和字段名 `title` 相同，可以省略 `title: title`，直接写 `title`。
            title,
            // `description`: 同上，使用了字段初始化简写。
            description,
            // `completed`: 同上。
            completed,
            // `created_at`: 将上面计算出的时间戳赋值给 `created_at` 字段。
            created_at: now,
            // `updated_at`: 新创建的任务，其最后更新时间就是创建时间。
            updated_at: now,
        } // 这个结构体实例是函数的【返回值】。在 Rust 中，如果函数的最后一条语句是一个表达式且没有分号，它就成为函数的返回值。[[Rust语法特性/概念: 隐式返回]]
    }

    /// 更新任务的【方法】
    ///
    /// 【功能】: 修改现有 `Task` 实例的属性。
    /// 【调用方式】: 通过一个 `Task` 实例来调用，例如 `my_task.update(payload);`
    ///
    /// # 【参数】
    /// * `&mut self` - 这是方法的【接收者 (Receiver)】。[[关键语法要素: &mut self]]
    ///    - `self`: 代表调用这个方法的那个 `Task` 实例本身。
    ///    - `&`: 表示我们接收的是对该实例的【引用 (Reference)】，而不是获取它的所有权。这意味着方法结束后，实例仍然有效。[[Rust语法特性/概念: 引用与借用]]
    ///    - `mut`: 表示这是一个【可变引用 (Mutable Reference)】。这意味着我们【可以】在这个方法内部修改 `self` (即 `Task` 实例) 的字段值。[[Rust语法特性/概念: 可变性]]
    ///      【所有权/借用规则】: 在同一作用域内，对于一个数据，你只能拥有【一个】可变引用，或者【多个】不可变引用，但不能同时拥有可变和不可变引用。这是 Rust 保证内存安全的核心机制之一。
    /// * `payload: UpdateTaskPayload` - 包含要更新字段的数据载荷。
    ///                    【所有权】: `payload` 的所有权被【转移】到这个方法中。方法结束后，外部不能再使用这个 `payload`。
    pub fn update(&mut self, payload: UpdateTaskPayload) {
        // --- 逐个字段更新 ---
        // 更新标题:
        // 检查 `payload.title` (类型是 `Option<String>`) 是否为 `Some`。
        if let Some(title) = payload.title {
            // 如果 payload 提供了 title (是 Some(String))
            // `payload.title` 的所有权在这里被【移动】到 `title` 变量中。
            self.title = title; // 将 `Task` 实例的 `title` 字段更新为新的值。
            // 这里发生了另一次所有权移动，`title` 变量的值移动到了 `self.title`。
        }
        // 如果 `payload.title` 是 `None`，则 `if let` 条件不满足，这部分代码不执行，`self.title` 保持不变。

        // 更新描述:
        // `payload.description` 的类型也是 `Option<String>` (由 `double_option` 处理后)。
        // 注意：这里没有使用 `if let`。这意味着无论 `payload.description` 是 `Some(new_desc)` 还是 `None`，都会执行赋值。
        // - 如果 `payload.description` 是 `Some(new_desc)`，则 `self.description` 更新为 `Some(new_desc)`。
        // - 如果 `payload.description` 是 `None` (意味着客户端想清除描述或未提供该字段，由`double_option`统一处理为`None`表示清除)，则 `self.description` 更新为 `None`。
        // 【所有权】: `payload.description` (Option<String>) 的所有权被移动到 `self.description`。
        self.description = payload.description;

        // 更新完成状态:
        // 检查 `payload.completed` (类型是 `Option<bool>`) 是否为 `Some`。
        if let Some(completed) = payload.completed {
            // 如果 payload 提供了 completed (是 Some(bool))
            // `payload.completed` 的值 (bool) 被【拷贝】到 `completed` 变量 (因为 bool 实现了 Copy Trait)。
            self.completed = completed; // 更新 `Task` 实例的 `completed` 字段。这里也是值的拷贝。
        }
        // 如果 `payload.completed` 是 `None`，则 `self.completed` 保持不变。

        // --- 更新 "最后更新时间" 字段 ---
        // 无论是否有字段被实际修改，只要调用了 `update` 方法，就应该更新 `updated_at` 时间戳。
        self.updated_at = SystemTime::now() // 获取当前时间
            .duration_since(UNIX_EPOCH) // 计算时间差
            .expect("系统时间错误") // 处理错误
            .as_secs(); // 转换为秒数
        // 将计算出的新时间戳赋值给 `self.updated_at`。
    }
}

// --- 请求载荷结构体 (Request Payload Structs) ---
// 这些结构体专门用于【接收和解析】来自客户端（例如前端浏览器或 API 测试工具）的 HTTP 请求体中的 JSON 数据。
// 它们使用了 `#[derive(Deserialize)]`，因此 `serde` 知道如何将 JSON 映射到这些结构体的字段上。

/// 创建任务请求载荷 (Request Payload for Creating Tasks)
///
/// 【用途】: 当客户端发送 `POST /tasks` 请求创建新任务时，请求体中的 JSON 数据会被【反序列化】成这个结构体的实例。
/// 【编程概念】: 这通常被称为 DTO (Data Transfer Object)，专门用于数据传输。
///
/// 【内存结构示意图 (简化)】 - 与 Task 类似，但字段不同
/// CreateTaskPayload {
///   title:       [String 数据] --> 内存地址 D --> "新任务标题" (堆)
///   description: [Option<String> 数据] --> Option::Some(内存地址 E --> "新任务描述" (堆)) 或 Option::None
///   completed:   [bool 数据] --> true / false
/// }
///
/// 【`derive` 宏解释】
/// - `Debug`: 允许打印调试信息。
/// - `Deserialize`: 【核心功能】让 `serde` 可以将 JSON 解析为此结构体。[[关键语法要素: derive 宏, Trait]]
///   例如，JSON `{"title": "写代码", "completed": false}` 会被解析成一个 `CreateTaskPayload` 实例。
#[derive(Debug, Deserialize)]
pub struct CreateTaskPayload {
    /// 任务标题 (Field Definition)
    /// 【类型】: `String`
    /// 【必需性】: 这个字段在传入的 JSON 中是【必需的】。如果 JSON 缺少 `title` 字段，反序列化会失败。
    pub title: String,

    /// 任务描述 (可选) (Field Definition)
    /// 【类型】: `Option<String>`
    /// 【必需性】: 这个字段在传入的 JSON 中是【可选的】。
    /// 【`serde` 属性】: `#[serde(default)]` [[关键语法要素: 属性宏]]
    ///    - 这个属性告诉 `serde`，如果在 JSON 中【找不到】 `description` 字段，或者该字段的值是 `null`，
    ///      就使用该字段类型的【默认值】。
    ///    - `Option<T>` 的默认值是 `None`。[[Rust语法特性/概念: Default Trait]]
    ///    - 因此，如果请求的 JSON 是 `{"title": "任务A"}` 或 `{"title": "任务A", "description": null}`，
    ///      反序列化后的 `CreateTaskPayload` 实例中，`description` 字段都会是 `None`。
    #[serde(default)] // 如果 JSON 中没有提供 "description"，则默认为 None
    pub description: Option<String>,

    /// 任务完成状态 (可选，默认为 false) (Field Definition)
    /// 【类型】: `bool`
    /// 【必需性】: 这个字段在传入的 JSON 中是【可选的】。
    /// 【`serde` 属性】: `#[serde(default)]`
    ///    - 同样，如果 JSON 中【找不到】 `completed` 字段，或者该字段的值是 `null` (虽然 JSON 标准中布尔值通常不为 null)，
    ///      就使用 `bool` 类型的【默认值】。
    ///    - `bool` 的默认值是 `false`。[[Rust语法特性/概念: Default Trait]]
    ///    - 因此，如果请求的 JSON 是 `{"title": "任务B"}`，反序列化后的 `completed` 字段会是 `false`。
    #[serde(default)] // 如果 JSON 中没有提供 "completed"，则默认为 false
    pub completed: bool,
}

/// 更新任务请求载荷 (Request Payload for Updating Tasks)
///
/// 【用途】: 当客户端发送 `PUT /tasks/:id` 或 `PATCH /tasks/:id` 请求更新现有任务时，请求体中的 JSON 数据会被【反序列化】成这个结构体的实例。
/// 【设计哲学】: 更新操作通常只需要提供【需要修改】的字段。因此，这个结构体的所有字段都是 `Option<T>` 类型，表示它们都是【可选的】。
///
/// 【内存结构示意图 (简化)】 - 所有字段都是 Option
/// UpdateTaskPayload {
///   title:       [Option<String> 数据] --> Option::Some(内存地址 F --> "更新后标题" (堆)) 或 Option::None
///   description: [Option<String> 数据] --> Option::Some(内存地址 G --> "更新后描述" (堆)) 或 Option::None (由 double_option 处理)
///   completed:   [Option<bool> 数据] --> Option::Some(true / false) 或 Option::None
/// }
///
#[derive(Debug, Deserialize)]
pub struct UpdateTaskPayload {
    /// 任务标题 (可选) (Field Definition)
    /// 【类型】: `Option<String>`
    /// 【作用】: 如果 JSON 中提供了 `title` 字段 (且不为 null)，则该值为 `Some(String)`；否则为 `None`。
    /// 【`serde` 行为】: 默认情况下，如果 JSON 中缺少 `title` 或值为 `null`，`serde` 会将其反序列化为 `None`。
    pub title: Option<String>,

    /// 任务描述 (可选，特殊处理) (Field Definition)
    /// 【类型】: `Option<String>` (看起来是，但 `serde` 行为被定制了)
    /// 【复杂性来源】: 对于可选字段的更新，通常有三种意图：
    ///    1. **不修改 (No Change)**: 请求中根本不包含该字段。
    ///    2. **设置新值 (Set Value)**: 请求中包含该字段，并有一个非 null 的值。
    ///    3. **清除值/设为Null (Clear Value/Set to Null)**: 请求中包含该字段，但其值为 `null`。
    /// 【标准 `Option<String>` 的问题】: 使用标准的 `#[serde(default)]` 的 `Option<String>` 无法区分情况 1 (不修改) 和 情况 3 (清除值)，因为 JSON 中缺少字段或字段值为 `null` 都会导致结果是 `None`。
    /// 【解决方案】: 使用 `#[serde(default, with = "double_option")]` 属性。[[关键语法要素: 属性宏, serde with]]
    ///    - `default`: 确保即使 JSON 中没有 `description` 字段，反序列化也能成功（会调用 `double_option::deserialize`）。
    ///    - `with = "double_option"`: 告诉 `serde` 使用我们下面定义的 `double_option` 模块中的【自定义序列化/反序列化逻辑】来处理这个字段。
    /// 【`double_option` 效果】:
    ///    - JSON 中无 `description` 字段 -> `double_option::deserialize` -> `Ok(None)` -> `UpdateTaskPayload.description` 为 `None` (解释为：不修改)。
    ///    - JSON 中 `"description": "新描述"` -> `double_option::deserialize` -> `Ok(Some("新描述"))` -> `UpdateTaskPayload.description` 为 `Some("新描述")` (解释为：设置新值)。
    ///    - JSON 中 `"description": null` -> `double_option::deserialize` -> `Ok(None)` -> `UpdateTaskPayload.description` 为 `None` (解释为：清除值)。
    ///   【注意】: 这里的最终 `description: Option<String>` 仍然是单层 `Option`。`double_option` 模块内部处理了双层 `Option` 的逻辑，然后返回一个单层 `Option` 给这个结构体字段。`Task::update` 方法需要根据 `payload.description` 是 `Some` 还是 `None` 来决定如何更新 `Task` 的 `description`。
    #[serde(default, with = "double_option")]
    pub description: Option<String>, // 虽然用了 double_option，但最终类型是 Option<String>

    /// 任务完成状态 (可选) (Field Definition)
    /// 【类型】: `Option<bool>`
    /// 【作用】: 如果 JSON 中提供了 `completed` 字段 (且值为 `true` 或 `false`)，则该值为 `Some(bool)`；否则为 `None`。
    /// 【`serde` 行为】: 默认行为。缺少字段或值为 `null` 会得到 `None`。
    pub completed: Option<bool>,
}

// --- 自定义 Serde 序列化/反序列化逻辑 ---

/// 模块: `double_option` - 处理特殊的 Option<Option<T>> 反序列化
///
/// 【目的】: 解决更新操作中区分"未提供字段"（不修改）和"字段值为 null"（清除值）的问题。
/// 【实现方式】: 提供自定义的 `deserialize` 函数，该函数内部期望解析的是 `Option<Option<T>>`，
///             然后根据解析结果（字段不存在、字段为 null、字段有值）返回一个 `Option<T>`。
/// 【编程概念】: 这是 `serde` 提供的【扩展点】，允许开发者为特定类型或特定字段定制序列化/反序列化的具体行为。
/// 【`mod`】: `mod double_option { ... }` 定义了一个名为 `double_option` 的【内联模块 (Inline Module)】。[[关键语法要素: mod]]
mod double_option {
    // 再次导入需要用到的 `serde` 组件
    use serde::{ Deserialize, Deserializer, Serialize, Serializer };

    /// 自定义【反序列化】函数 (Custom Deserialization Function)
    ///
    /// 【签名解释】:
    /// - `pub fn deserialize`: 定义一个公共函数 `deserialize`。
    /// - `<'de, T, D>`: 定义【泛型参数 (Generic Parameters)】和【生命周期参数 (Lifetime Parameter)】。[[关键语法要素: 泛型, 生命周期]]
    ///    - `'de`: 这是一个【生命周期参数】，与反序列化过程有关，表示反序列化器 `D` 和可能从中借用的数据的生命周期。初学者可以暂时忽略其细节，知道它与借用检查有关即可。
    ///    - `T`: 一个【泛型类型参数】，代表我们想要反序列化的【最终值的类型】(例如 `String`)。它必须实现 `Deserialize<'de>` 特性，意味着 `T` 类型本身也能被反序列化。
    ///    - `D`: 一个【泛型类型参数】，代表【反序列化器 (Deserializer)】的类型 (例如 JSON 反序列化器)。它必须实现 `Deserializer<'de>` 特性。
    /// - `(deserializer: D)`: 函数接收一个反序列化器 `D` 作为参数。
    /// - `-> Result<Option<T>, D::Error>`: 函数的【返回值】是一个 `Result`。[[Rust语法特性/概念: Result 枚举]]
    ///    - `Ok(Option<T>)`: 如果反序列化成功，返回 `Ok`，里面包含一个 `Option<T>`。这个 `Option<T>` 就是我们根据 `Option<Option<T>>` 逻辑转换后的结果。
    ///    - `Err(D::Error)`: 如果反序列化过程中出错，返回 `Err`，里面包含具体的错误信息，错误类型由反序列化器 `D` 定义。
    /// - `where T: Deserialize<'de>, D: Deserializer<'de>`: 对泛型参数的【约束 (Bounds)】。[[关键语法要素: where 子句]]
    ///    - 确保传入的类型 `T` 和 `D` 满足我们需要的特性。
    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
        where
            T: Deserialize<'de>, // T 必须能被反序列化
            D: Deserializer<'de> // D 必须是一个反序列化器
    {
        // --- 核心逻辑 ---
        // 1. 尝试将输入反序列化为 `Option<Option<T>>` 类型。
        //    - 如果 JSON 中【没有】对应的键，`Option::deserialize` 会成功并返回 `Ok(None)`。
        //    - 如果 JSON 中键的值是 `null`， `Option::deserialize` 会成功并返回 `Ok(Some(None))`。
        //    - 如果 JSON 中键的值是有效的 `T` 类型数据 (例如 `"some string"`), `Option::deserialize` 会成功并返回 `Ok(Some(Some(value)))`。
        //    - 如果 JSON 中键的值类型不匹配，会返回 `Err(...)`。
        //    `?` 操作符用于【错误传播】: 如果 `deserialize` 返回 `Err`，`?` 会立即将这个 `Err` 作为当前 `deserialize` 函数的返回值。[[关键语法要素: ? 操作符]]
        let opt_opt: Option<Option<T>> = Option::deserialize(deserializer)?;

        // 2. 使用 `match` 表达式处理 `Option<Option<T>>` 的三种情况。[[关键语法要素: match]]
        Ok(match opt_opt {
            // `Ok(...)` 将 match 的结果包装在 Result::Ok 中返回
            // 情况 1: JSON 中没有该字段 (`opt_opt` is None)
            // 对应意图：客户端不打算修改此字段。
            // 返回值：`None` (表示最终的 `UpdateTaskPayload` 字段值为 `None`)
            None => None, // 外层 None

            // 情况 2: JSON 中字段值为 `null` (`opt_opt` is Some(None))
            // 对应意图：客户端想要清除该字段的值。
            // 返回值：`None` (同样，最终的 `UpdateTaskPayload` 字段值为 `None`)
            // **重要**: 在 `Task::update` 方法中，需要知道这个 `None` 是来自情况1还是情况2吗？
            //         对于 `description: Option<String>` 字段，赋值 `None` 就能达到清除的效果。
            //         所以，对于最终的 `UpdateTaskPayload.description` 来说，这两种情况都映射为 `None` 是可以接受的。
            //         如果需要区分这两种情况，`UpdateTaskPayload.description` 的类型可能需要更复杂的设计，
            //         或者 `Task::update` 需要接收更原始的信息。当前设计是常见的简化处理。
            Some(None) => None, // 内层 None

            // 情况 3: JSON 中字段有值 (`opt_opt` is Some(Some(val)))
            // 对应意图：客户端想要设置新的值。
            // 返回值：`Some(val)` (最终的 `UpdateTaskPayload` 字段值为 `Some(val)`)
            Some(Some(val)) => Some(val), // 内层 Some，提取出值 val
        })
    }

    /// 自定义【序列化】函数 (Custom Serialization Function)
    ///
    /// 【用途】: 虽然 `UpdateTaskPayload` 主要用于【反序列化】(接收请求)，但 `serde` 的 `with` 属性通常需要同时提供序列化和反序列化函数。
    ///         这个函数定义了如何将 `Option<T>` (因为 `UpdateTaskPayload.description` 最终是 `Option<String>`) 序列化回 JSON。
    /// 【注意】: 这个序列化逻辑相对简单，它只是将 `None` 序列化为 `null`，将 `Some(val)` 序列化为 `val`。
    ///         它并没有反映出反序列化时处理 `Option<Option<T>>` 的复杂性。这通常没问题，因为我们不常需要将 `UpdateTaskPayload` 序列化回 JSON。
    ///
    /// 【签名解释】:
    /// - `<S, T>`: 泛型参数。`S` 是序列化器类型，`T` 是要序列化的值的类型。
    /// - `value: &Option<T>`: 接收一个对 `Option<T>` 的【不可变引用】。[[Rust语法特性/概念: 不可变引用]]
    /// - `serializer: S`: 接收一个序列化器 `S`。     
    /// - `-> Result<S::Ok, S::Error>`: 返回一个 `Result`。`Ok` 包含序列化成功的结果类型，`Err` 包含序列化错误。
    /// - `where S: Serializer, T: Serialize`: 约束，`S` 必须是序列化器，`T` 必须能被序列化。
    pub fn serialize<S, T>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer, // S 必须是一个序列化器
            T: Serialize // T 必须能被序列化
    {
        // 使用 match 处理 Option
        match value {
            // 如果值是 None，调用序列化器的 `serialize_none` 方法将其序列化为 JSON null。
            None => serializer.serialize_none(),
            // 如果值是 Some(val)，调用序列化器的 `serialize_some` 方法，并传入内部的值 `val` 进行序列化。
            // `val` 本身 (类型 T) 也必须是可序列化的 (由 `where T: Serialize` 保证)。
            Some(val) => serializer.serialize_some(val),
        }
    }
} // mod double_option 结束
