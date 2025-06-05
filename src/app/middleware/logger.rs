// 文件路径: src/app/middleware/logger.rs

// /---------------------------------------------------------------------------------------------------------\
// |                                    【日志中间件模块】 (logger.rs)                                     |
// |---------------------------------------------------------------------------------------------------------|
// |                                                                                                         |
// | [Axum HTTP 请求]                                                                                         |
// |      |                                                                                                  |
// |      V (请求进入中间件栈)                                                                                   |
// | [TraceLayer 中间件 (由 `trace_layer()` 函数创建)]                                                        |
// |   - `on_request`: 当请求到达时, 记录请求信息 (如方法, URI) 到 `tracing` 系统。                           |
// |      |                                                                                                  |
// |      V (请求传递给后续中间件或 Handler)                                                                   |
// | [应用 Handler 处理请求]                                                                                  |
// |      |                                                                                                  |
// |      V (Handler 返回响应)                                                                                |
// | [TraceLayer 中间件]                                                                                     |
// |   - `on_response`: 当响应生成时, 记录响应信息 (如状态码, 延迟) 到 `tracing` 系统。                       |
// |   - `on_body_chunk` (可选): 如果配置, 记录响应体数据块信息。                                            |
// |   - `on_failure`: 如果请求处理过程中发生错误 (根据分类器定义), 记录错误信息。                             |
// |      |                                                                                                  |
// |      V (响应继续向外传递)                                                                                 |
// | [Axum HTTP 响应]                                                                                         |
// |                                                                                                         |
// | [tracing 日志系统 (由 `setup_logger()` 配置)]                                                            |
// |   - `EnvFilter`: 根据 `RUST_LOG` 环境变量或默认设置过滤日志事件。                                         |
// |   - `fmt::layer`: 将符合条件的日志事件格式化为人类可读的文本。                                            |
// |   - (输出到控制台)                                                                                       |
// |                                                                                                         |
// \---------------------------------------------------------------------------------------------------------/
//
// 【模块核心职责】 (Module Core Responsibilities)
// 1. **配置全局日志系统 (Configure Global Logging System)**:
//    - `setup_logger()` 函数负责初始化和配置 `tracing` 日志框架。这包括设置日志事件的过滤级别 (例如，通过 `RUST_LOG` 环境变量)、
//      日志的格式化方式 (例如，使其更易读或输出为 JSON)，以及日志的输出目标 (通常是控制台标准输出/错误)。
//    - 一旦全局日志系统被初始化，应用程序中的任何地方（包括其他库）使用 `tracing` 的宏 (如 `info!`, `debug!`, `error!`) 发出的日志事件，
//      都将由这个配置好的系统来处理。
// 2. **提供 HTTP 请求跟踪中间件 (Provide HTTP Request Tracing Middleware)**:
//    - `trace_layer()` 函数创建并返回一个 `tower_http::trace::TraceLayer` 实例。
//    - `TraceLayer` 是一个 Axum (Tower) 中间件，它能够自动记录有关每个传入 HTTP 请求和对应传出响应的详细信息。
//      例如，它可以记录请求的方法 (GET, POST)、URI、HTTP 版本、响应的状态码、处理请求所花费的时间 (延迟) 等。
//    - 这个中间件帮助开发者监控应用的 HTTP 流量，调试问题，以及了解请求的生命周期。
//
// 【关键技术点】 (Key Technologies)
// - **`tracing` Crate**: Rust 生态中一个强大且可扩展的框架，用于检测 (instrumenting) 应用程序以收集结构化的、事件驱动的诊断信息。
//   它不仅仅是简单的文本日志，还可以支持分布式追踪、指标收集等。
// - **`tracing_subscriber` Crate**: `tracing` 的一个配套库，提供了构建和配置“订阅者 (Subscriber)”的工具。
//   订阅者负责处理由 `tracing` 产生的日志事件 (也称为 spans 和 events)，决定如何过滤、格式化和记录它们。
//   - **`Registry`**: 一个基础的订阅者，可以将多个处理层 (Layers) 组合起来。
//   - **`Layer`**: 一个可组合的组件，用于向订阅者添加特定的行为，如过滤、格式化、发送到特定输出。
//   - **`EnvFilter`**: 一个常用的 `Layer`，它允许通过环境变量 (通常是 `RUST_LOG`) 来动态配置日志级别和目标过滤规则。
//   - **`fmt::Layer`**: 一个 `Layer`，负责将日志事件格式化为人类可读的文本，并输出到控制台 (标准输出/错误)。
// - **`tower_http::trace::TraceLayer`**: `tower-http` 库提供的一个中间件，专门用于 HTTP 服务的请求跟踪。
//   它与 `tracing` 框架集成，当 HTTP 请求通过它时，会自动发出包含请求和响应详细信息的 `tracing` 事件。
//   - **`MakeSpan`**: `TraceLayer` 使用 `MakeSpan` trait 的实现来为每个请求创建一个 `tracing::Span`。Span 代表一个工作单元的生命周期 (例如一个 HTTP 请求的处理过程)。
//   - **`OnRequest`, `OnResponse`, `OnFailure`**: 这些 traits (或其默认实现如 `DefaultOnRequest`) 允许自定义在请求开始、响应生成、处理失败等不同阶段记录哪些信息以及如何记录。
// - **Axum 中间件集成**: `TraceLayer` 作为一个实现了 `tower::Layer` trait 的类型，可以无缝集成到 Axum 的中间件栈中。

// --- 导入依赖 ---
// `use tower_http::trace::{ self, TraceLayer };`
//   - 从 `tower_http` crate 的 `trace` 模块中导入。
//   - `self` (即 `tower_http::trace`): 允许我们使用该模块下的其他项，例如 `DefaultOnRequest`, `DefaultOnResponse` 等。
//   - `TraceLayer`: 这是核心的 HTTP 请求跟踪中间件类型。
use tower_http::trace::{ self, TraceLayer }; // `self` 关键字允许我们使用 `trace::DefaultOnRequest` 这样的路径
// `use tracing::Level;`
//   - 从 `tracing` crate 导入 `Level` 枚举。
//   - `Level` 定义了不同的日志严重性级别，如 `Level::ERROR`, `Level::WARN`, `Level::INFO`, `Level::DEBUG`, `Level::TRACE`。
//     日志级别用于过滤日志消息，例如，如果设置为 `INFO`，则只有 `INFO`、`WARN`、`ERROR` 级别的消息会被记录。
use tracing::Level;
// `use tracing_subscriber::{ ... };`
//   - 从 `tracing_subscriber` crate 导入一系列用于配置日志订阅者的组件。
//   - `layer::SubscriberExt`: 一个扩展 trait，为订阅者类型 (如 `Registry`) 添加 `.with(layer)` 方法，用于组合不同的处理层。
//   - `util::SubscriberInitExt`: 一个扩展 trait，为构建好的订阅者添加 `.init()` 方法，用于将其设置为全局默认的日志处理器。
//   - `EnvFilter`: 一个日志层，它根据环境变量 (通常是 `RUST_LOG`) 的值来过滤哪些日志事件应该被处理。
use tracing_subscriber::{ layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, fmt }; // fmt 也需要显式导入

// `pub fn setup_logger()`
//   - `pub fn`: 定义一个公共函数 `setup_logger`。这意味着它可以从项目中的其他模块被调用 (通常是 `startup.rs`)。
//   - 此函数不接收参数，也不返回任何值 (`()` 类型，即 unit type)。它的作用是执行一个副作用：初始化全局日志系统。
/// 设置应用程序的全局日志系统。
///
/// 【功能】: 此函数负责初始化 `tracing` 日志框架，配置日志事件如何被过滤、格式化和输出。
///          它通常在应用程序启动的最早期阶段被调用一次 (例如，在 `startup.rs` 的 `init_app` 函数的开头)。
///          一旦全局日志系统被初始化，应用程序中任何地方（包括其他依赖库）使用 `tracing` 的宏
///          (如 `info!`, `debug!`, `error!`) 发出的日志事件，都将由这个配置好的系统来处理。
///
/// 【实现细节】:
/// 1. **创建环境过滤器 (`EnvFilter`)**:
///    - `EnvFilter::try_from_default_env()`: 尝试从名为 `RUST_LOG` 的环境变量中读取日志过滤指令。
///      环境变量可以用来动态控制日志的详细程度，例如:
///        - `RUST_LOG=info` : 显示所有 `INFO`、`WARN`、`ERROR` 级别的日志。
///        - `RUST_LOG=debug` : 显示所有 `DEBUG` 及以上级别的日志。
///        - `RUST_LOG=axum_tutorial=trace` : 仅显示名为 `axum_tutorial` (我们应用的 crate 名) 的模块发出的 `TRACE` 及以上级别日志。
///        - `RUST_LOG=info,axum_tutorial=debug,sea_orm=warn` : 更复杂的组合，默认 INFO，本应用 DEBUG，SeaORM WARN。
///    - `.unwrap_or_else(|_| EnvFilter::new("info"))`: 如果 `RUST_LOG` 环境变量没有设置，或者其内容无效，
///      则 `try_from_default_env()` 会失败。`unwrap_or_else` 允许我们提供一个备用方案：
///      在这种情况下，创建一个新的 `EnvFilter`，其默认规则是 `"info"` (即只显示 INFO 及以上级别的日志)。
///      `|_|` 是一个不关心错误具体内容的闭包。
///
/// 2. **构建和初始化订阅者 (`tracing_subscriber::registry()...init()`)**:
///    - `tracing_subscriber::registry()`: 创建一个基础的“订阅者注册表 (subscriber registry)”。
///      可以将注册表看作是可以附加不同日志处理层 (layers) 的一个容器或构建器。
///    - `.with(env_filter)`: 使用 `SubscriberExt` trait 提供的 `.with()` 方法，将前面创建的 `env_filter` 作为第一层添加到注册表中。
///      这意味着所有日志事件首先会经过 `EnvFilter` 的过滤。
///    - `.with(fmt::layer().pretty())`: 再添加一个格式化层。
///      - `fmt::layer()`: 创建一个将日志事件格式化为文本并输出到标准输出/错误的层。
///      - `.pretty()`: (可选) 使输出的格式更加美观、易读，通常带有颜色和多行显示。
///    - `.init()`: 使用 `SubscriberInitExt` trait 提供的 `.init()` 方法，将这个构建好的、包含多个层的订阅者
///      设置为【全局默认】的日志事件处理器。在此调用之后，程序中所有 `tracing` 日志宏的输出都将由这个订阅者处理。
///      一个程序通常只应调用一次 `.init()`。
pub fn setup_logger() {
    // 步骤 1: 创建环境过滤器 (EnvFilter)
    // 尝试从 RUST_LOG 环境变量读取日志级别配置。
    // 如果 RUST_LOG 未设置或无效，则默认使用 "info" 级别 (即 INFO, WARN, ERROR 会被记录)。
    // 例如，你可以运行 `RUST_LOG=debug cargo run` 来看到 DEBUG 级别的日志。
    // 或者 `RUST_LOG=axum_tutorial=trace cargo run` 来只看本应用的 trace 日志。
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info")); // 如果环境变量无效或未设置，则默认为 "info"

    // 步骤 2 & 3: 构建订阅者并将其初始化为全局默认处理器
    tracing_subscriber::registry() // 创建一个订阅者注册表 (registry)
        .with(env_filter) // 添加环境过滤器层，用于根据 RUST_LOG 或默认指令过滤日志
        .with(fmt::layer().pretty()) // 添加格式化层，将日志事件格式化为人类可读的文本并输出到控制台 (启用 .pretty() 以获得更美观的输出)
        .init(); // 将配置好的订阅者设置为全局默认

    // 使用 tracing::info! 宏记录一条信息，表明日志系统已成功初始化。
    // 这条日志本身也会被上面配置的订阅者处理。
    tracing::info!("日志系统已初始化 (默认级别: INFO，可通过 RUST_LOG 环境变量覆盖，例如 `RUST_LOG=debug` 或 `RUST_LOG=axum_tutorial=trace`)");
}

// `pub fn trace_layer() -> TraceLayer<...>`
//   - `pub fn`: 定义一个公共函数 `trace_layer`。
//   - `-> TraceLayer<...>`: 返回类型是 `tower_http::trace::TraceLayer`。
//     - `TraceLayer` 是一个泛型结构体。它的完整类型签名可能比较复杂，例如 `TraceLayer<SharedClassifier<ServerErrorsAsFailures>, DefaultMakeSpan, DefaultOnRequest, ...>`。
//     - 这些泛型参数定义了 `TraceLayer` 如何对请求进行分类 (成功/失败)、如何为请求创建 `tracing` Span、以及在请求生命周期的不同阶段（如开始、结束、失败）如何记录日志。
//     - 通常，我们不需要在函数签名中显式写出所有这些泛型参数，可以使用 `impl Trait` 或让编译器推断。
//       但如果函数直接返回具体类型，就需要写出来。这里为了清晰，使用了 `TraceLayer` 的具体类型路径，
//       但其泛型参数通常由 `TraceLayer::new_for_http()` 的默认值提供，所以可以简化为 `-> TraceLayer<impl MakeClassifier ...>` 或类似的。
//       当前代码中使用了更具体的类型，这是可以的，但有时会显得冗长。
//       `tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>` 是一个常见的分类器，
//       它将 HTTP 5xx 错误视为服务器端故障，而 4xx 错误视为客户端故障 (不一定记录为 ERROR 级别)。
/// 创建并返回一个用于 HTTP 请求跟踪的 `TraceLayer` 中间件。
///
/// 【功能】: 此函数构造并返回一个配置好的 `tower_http::trace::TraceLayer` 实例。
///          这个 `TraceLayer` 可以作为 Axum 中间件添加到路由中，用于自动记录关于
///          传入 HTTP 请求和对应传出响应的关键信息 (例如方法、路径、状态码、处理延迟等)。
/// 【用途】: 使得为 Web 服务添加详细的请求级日志记录变得非常方便，有助于调试和监控。
///
/// # 【返回值】
/// * `-> TraceLayer<tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>>`:
///   返回一个 `TraceLayer` 实例。
///   - `SharedClassifier<ServerErrorsAsFailures>` 是 `TraceLayer` 使用的一个请求分类器 (classifier)。
///     它定义了如何根据请求和响应来判断一个操作是成功还是失败，以及失败的严重程度。
///     `ServerErrorsAsFailures` 这个具体的分类器会将 HTTP 5xx 状态码（服务器错误）标记为真正的失败 (通常记录为 ERROR 级别)，
///     而 4xx 状态码（客户端错误）可能不会被视为同等级别的失败。
///   - 开发者通常不需要深入关心这些泛型参数的细节，除非需要高度自定义分类或 Span 创建行为。
pub fn trace_layer() -> TraceLayer<tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>> {
    // `TraceLayer::new_for_http()`: 这是 `TraceLayer` 的一个便捷构造函数，
    // 它使用一套适用于典型 HTTP 服务场景的默认配置来创建一个新的 `TraceLayer` 实例。
    // 这些默认配置包括：
    //   - 如何为每个请求创建 `tracing` Span。
    //   - 在请求开始时记录哪些信息。
    //   - 在响应结束时记录哪些信息（包括状态码和延迟）。
    //   - 如何处理和记录失败的请求。
    TraceLayer::new_for_http()
        // `.on_request(trace::DefaultOnRequest::new().level(Level::INFO))`: 自定义请求开始时的日志行为。
        //   - `on_request` 方法允许我们提供一个实现了 `OnRequest` trait 的回调。
        //   - `trace::DefaultOnRequest::new()`: 使用 `tower_http` 提供的默认请求日志记录器。
        //   - `.level(Level::INFO)`: 将请求开始事件的日志级别设置为 `INFO`。
        //     这意味着当一个新请求到达时，会以 `INFO` 级别记录一条日志，如 "INFO request: GET /path HTTP/1.1"。
        .on_request(trace::DefaultOnRequest::new().level(Level::INFO))
        // `.on_response(trace::DefaultOnResponse::new().level(Level::INFO).latency_unit(tower_http::LatencyUnit::Micros))`: 自定义响应生成时的日志行为。
        //   - `on_response` 方法允许提供一个实现了 `OnResponse` trait 的回调。
        //   - `trace::DefaultOnResponse::new()`: 使用默认的响应日志记录器。
        //   - `.level(Level::INFO)`: 将响应事件的日志级别设置为 `INFO`。
        //     例如: "INFO response: 200 OK in 123.45 ms"。
        //   - `.latency_unit(tower_http::LatencyUnit::Micros)`: (可选) 设置响应日志中延迟时间的单位为微秒 (µs)。
        //     默认可能是毫秒 (ms)。根据精度需求选择。
        .on_response(
            trace::DefaultOnResponse::new()
                .level(Level::INFO)
                .latency_unit(tower_http::LatencyUnit::Micros) // 将延迟单位设置为微秒
        )
        // `.on_body_chunk(trace::DefaultOnBodyChunk::new())`: (可选) 配置当处理请求体或响应体的数据块 (chunk) 时如何记录日志。
        //   - 这通常用于调试流式传输的 body，例如查看每个数据块的大小。
        //   - 对于大多数应用，这可能过于详细，可以省略。
        .on_body_chunk(trace::DefaultOnBodyChunk::new())
        // `.on_failure(trace::DefaultOnFailure::new().level(Level::ERROR))`: 自定义当请求被分类为失败时如何记录日志。
        //   - `on_failure` 方法允许提供一个实现了 `OnFailure` trait 的回调。
        //   - `trace::DefaultOnFailure::new()`: 使用默认的失败日志记录器。
        //   - `.level(Level::ERROR)`: 将失败事件的日志级别设置为 `ERROR`。
        //     `TraceLayer` 使用其配置的分类器 (本例中是 `ServerErrorsAsFailures`) 来判断请求是否失败。
        //     通常，HTTP 5xx 错误会被视为失败并以 `ERROR` 级别记录。
        //     HTTP 4xx 错误（客户端错误）可能不会触发 `on_failure`，而是通过 `on_response` 正常记录。
        .on_failure(trace::DefaultOnFailure::new().level(Level::ERROR))
}

[end of src/app/middleware/logger.rs]
