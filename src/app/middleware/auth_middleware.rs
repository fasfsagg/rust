// 文件路径: src/app/middleware/auth_middleware.rs

// /--------------------------------------------------------------------------------------------------\
// |                            【模块功能图示】 (auth_middleware.rs)                                 |
// |--------------------------------------------------------------------------------------------------|
// |                                                                                                  |
// | [HTTP 请求 (Request)] (从客户端到达，可能前往受保护路由)                                             |
// |      |                                                                                           |
// |      V                                                                                           |
// | [jwt_auth_middleware(request, next) 函数开始]                                                     |
// |   1. [提取 `Authorization` 请求头]                                                                 |
// |      |                                                                                           |
// |      |--- (头部缺失?) --- 是 ---> [返回 Err(AppError::MissingToken)] (请求短路)                     |
// |      |                                                                                           |
// |      |--- (头部存在，但格式不正确? e.g., 不是 "Bearer <token>") --- 是 ---> [返回 Err(AppError::InvalidToken)] (请求短路) |
// |      |                                                                                           |
// |      V (头部存在且格式基本正确, 提取 token 字符串)                                                      |
// |   2. [解码和验证 JWT (jsonwebtoken::decode)]                                                       |
// |      - 使用预设的 JWT_SECRET 和验证规则 (HS256 算法, 检查过期时间 `exp`)。                         |
// |      |                                                                                           |
// |      |--- (解码/验证失败? e.g., 签名无效, 已过期, 格式错误) --- 是 ---> [返回 Err(AppError::InvalidToken)] (请求短路) |
// |      |                                                                                           |
// |      V (JWT 有效, 解码出 Claims)                                                                   |
// |   3. [将 `Claims` 注入请求扩展]                                                                    |
// |      - `request.extensions_mut().insert(token_data.claims)`                                     |
// |      |                                                                                           |
// |      V                                                                                           |
// |   4. [调用 `next.run(request).await`] (将请求 (已带有 Claims) 传递给下一个中间件或目标 Handler)       |
// |      |                                                                                           |
// |      V (下一个组件返回 Response)                                                                   |
// | [返回 `Ok(Response)`] (将来自下一个组件的响应继续向外传递)                                           |
// |                                                                                                  |
// \--------------------------------------------------------------------------------------------------/
//
// 【模块核心职责】 (Module Core Responsibilities)
// 1. **提供 JWT (JSON Web Token) 认证**: 此中间件是应用程序认证机制的核心组成部分。
//    它拦截指向受保护资源的传入 HTTP 请求，并验证请求中是否包含有效 JWT。
// 2. **令牌提取与验证 (Token Extraction and Validation)**:
//    - 从 HTTP 请求的 `Authorization` 头部提取 `Bearer` 类型的令牌。
//    - 使用 `jsonwebtoken` crate 和预配置的密钥 (JWT_SECRET) 对提取的令牌进行解码和验证，包括检查签名是否正确、令牌是否过期等。
// 3. **声明注入 (Claims Injection)**: 如果 JWT 验证成功，中间件会将从令牌中解码出的用户声明 (`Claims` 结构体，通常包含用户ID等信息)
//    注入到当前请求的“扩展 (extensions)”中。这使得后续的请求处理函数 (handler) 可以方便地访问到已认证用户的信息。
// 4. **请求短路 (Request Short-circuiting)**: 如果在任何阶段认证失败（例如，没有提供令牌、令牌格式错误、令牌无效或已过期），
//    中间件会立即停止请求处理链的进一步执行（即“短路”），并直接返回一个相应的认证错误 HTTP 响应 (通过 `AppError` 实现)。
// 5. **传递控制权 (Passing Control)**: 如果认证成功，中间件会将请求（现在已附加了用户声明）传递给处理链中的下一个组件（可能是另一个中间件或最终的路由处理函数）。
//
// 【关键技术点】 (Key Technologies)
// - **Axum 中间件**:
//   - **函数式中间件**: 实现为一个异步函数 `async fn(Request, Next) -> Result<Response, AppError>`。
//     这是 Axum 中创建中间件的常见方式之一 (通过 `axum::middleware::from_fn` 包装后应用到路由)。
//   - `axum::extract::Request`: 代表传入的 HTTP 请求。中间件可以读取和修改它。
//   - `axum::middleware::Next`: 一个特殊类型，代表请求处理链中的“下一个”组件。调用 `next.run(request).await` 会将请求传递下去并等待其响应。
//   - `axum::response::Response`: 中间件的返回值类型，通常是 `Result<Response, AppError>`，表示它可以直接返回一个响应（例如错误响应）或在成功时传递由 `Next` 返回的响应。
// - **HTTP 头部处理**:
//   - `request.headers().get(header::AUTHORIZATION)`: 从请求中获取 `Authorization` HTTP 头部的值。
//   - **Bearer Token Scheme**: 遵循 RFC 6750 定义的 Bearer Token 方案，即令牌在 `Authorization` 头中以 "Bearer <token>" 的形式提供。
// - **`jsonwebtoken` Crate**: 用于 JWT 的解码 (`decode`) 和验证 (`Validation`)。
//   - `DecodingKey::from_secret()`: 从字节序列创建用于验证签名的密钥。
//   - `Validation::new(Algorithm::HS256)`: 创建验证规则，指定期望的签名算法 (HS256) 并启用默认的验证检查 (如 `exp` 过期声明)。
// - **请求扩展 (`request.extensions_mut().insert()`)**: Axum (基于 `http` crate) 的一种机制，允许在请求处理的不同阶段（如中间件和处理器之间）安全地传递任意类型的数据。
//   `Claims` 被插入后，后续的处理器可以通过 `Extension<Claims>` 提取器来访问它。
// - **错误处理 (`AppError`)**: 中间件在认证失败时返回自定义的 `AppError` (如 `MissingToken`, `InvalidToken`)，这些错误会被 Axum 转换为适当的 HTTP 错误响应。

// --- 导入依赖 ---
// `use axum::{...}`: 从 `axum` crate 导入所需的组件。
use axum::{
    extract::Request,     // `Request` 类型，代表传入的 HTTP 请求。中间件函数接收此类型的参数。
    http::{header, StatusCode}, // `header`模块包含常用的 HTTP 头部名称常量 (如 `AUTHORIZATION`)。`StatusCode` 用于测试。
    middleware::Next,     // `Next` 类型，代表请求处理链中的下一个中间件或最终的请求处理函数。
    response::Response,   // `Response` 类型，代表将要发送回客户端的 HTTP 响应。中间件函数返回此类型 (通常在 `Result` 中)。
};
// `use jsonwebtoken::{...}`: 从 `jsonwebtoken` crate 导入 JWT 解码和验证相关的组件。
use jsonwebtoken::{
    decode,             // `decode` 函数，用于解码并验证 JWT 字符串。
    DecodingKey,        // `DecodingKey` 类型，用于包装验证 JWT 签名时所需的密钥。
    Validation,         // `Validation` 结构体，用于配置 JWT 验证的规则 (例如指定算法、是否检查过期时间等)。
    Algorithm,          // `Algorithm` 枚举，用于指定 JWT 使用的签名算法 (本项目中使用 HS256)。
};
// `use crate::app::model::Claims;`
//   - 导入在 `src/app/model/auth_dtos.rs` 中定义的 `Claims` 结构体。
//   - 当 JWT 被成功解码和验证后，其载荷 (payload) 部分将被反序列化为此 `Claims` 结构体的实例。
use crate::app::model::Claims;
// `use crate::error::AppError;`
//   - 导入在 `src/error.rs` 中定义的 `AppError` 枚举。
//   - 当认证失败时 (例如令牌缺失、无效、过期)，此中间件将返回一个合适的 `AppError` 变体。
use crate::error::AppError;

// `const BEARER_PREFIX: &str = "Bearer ";`
//   - 定义一个常量字符串，表示 "Bearer " 这个前缀。
//   - HTTP `Authorization` 头部在使用 Bearer Token 方案时，其值的格式通常是 "Bearer <token_string>"。
//   - 此常量用于检查头部格式和提取实际的令牌字符串。
const BEARER_PREFIX: &str = "Bearer ";

// `const JWT_SECRET: &str = "...";`
//   - 定义一个常量字符串，用作 JWT 签名和验证的密钥。
//   - **TODO**: 这是一个【严重的安全隐患】！将密钥硬编码在代码中非常不安全，容易泄露。
//     在生产环境中，JWT 密钥必须：
//       1. 非常复杂且难以猜测 (例如，一个长随机字符串)。
//       2. 从安全的环境变量或配置文件中加载 (例如，通过 `AppConfig`)。
//       3. 绝不能提交到版本控制系统 (例如，使用 `.gitignore` 忽略包含密钥的配置文件)。
//   - 此处使用占位符密钥是为了项目能运行，但必须在部署前替换为安全的管理方式。
//   - 这个密钥必须与 `AuthService::login_user` 中用于【编码】JWT 的密钥完全相同。
const JWT_SECRET: &str = "your-placeholder-super-secret-key-that-must-be-changed"; // ⚠️ 极不安全! 必须替换!

// `pub async fn jwt_auth_middleware(...) -> Result<Response, AppError>`
//   - `pub async fn`: 定义一个公共的 (public) 异步 (async) 函数 `jwt_auth_middleware`。
//     这是我们的 JWT 认证中间件的核心实现。
//   - `mut req: Request`: 中间件接收的第一个参数是 `axum::extract::Request` 类型的 HTTP 请求。
//     - `mut`: 请求 `req` 被声明为可变的 (`mut`)，因为我们稍后可能需要修改它（通过 `req.extensions_mut().insert(...)` 来添加 `Claims`）。
//     - 在 Axum 0.7+ 版本中，中间件的签名通常是 `async fn(Request, Next) -> Result<Response, AppError>` 或 `async fn(State<S>, Request, Next) -> ...` 等。
//       注意：Axum 的早期版本 (如 0.6) 中间件可能使用 `http::Request<B>` 类型。当前代码使用 `axum::extract::Request`，这是 Axum 0.7+ 中对 `http::Request` 的封装，提供了更便捷的提取器等功能。
//       如果 `Request` 是 `axum::extract::Request`，它本身可能无法直接修改 `extensions`。通常，中间件会操作 `http::Request` 的部分，
//       或者如果需要修改并传递给 `Next`，可能需要从 `axum::extract::Request` 中提取出 `http::Request` 部分。
//       不过，`req.extensions_mut()` 是 `http::Request` 的方法。Axum 的 `Request` 类型通过 `DerefMut` 实现了对内部 `http::Request` 的可变访问。
//   - `next: Next`: 中间件接收的第二个参数是 `axum::middleware::Next`。
//     - `Next` 代表请求处理链中的“下一个”组件（可能是另一个中间件，或者最终的路由处理函数）。
//     - 调用 `next.run(req).await` 会将请求 `req` (可能已被当前中间件修改) 传递给下一个组件，并异步等待其返回 `Response`。
//   - `-> Result<Response, AppError>`: 中间件的返回类型。
//     - `Result<Response, AppError>`: 表示中间件的执行结果。
//       - `Ok(Response)`: 如果认证成功并通过 `next.run(req).await` 获得了来自后续处理的响应，则将此响应包装在 `Ok` 中返回。
//       - `Err(AppError)`: 如果认证失败（例如令牌无效、缺失），则中间件会“短路”请求处理链，直接返回一个 `AppError`。
//         由于 `AppError` 实现了 `IntoResponse`，Axum 会将这个 `AppError` 转换为相应的 HTTP 错误响应并发送给客户端。
/// JWT 认证中间件。
///
/// 负责从请求的 `Authorization` 头部提取 Bearer Token，验证其有效性（签名、过期时间等），
/// 如果有效，则将解码后的 `Claims` 注入到请求的扩展中，供后续的 Handler 使用。
/// 如果无效或缺失，则返回相应的认证错误 (`AppError`)，从而阻止请求到达目标 Handler。
pub async fn jwt_auth_middleware(
    mut req: Request, // 传入的 HTTP 请求 (可变，因为要修改其 extensions)
    next: Next,       // 代表处理链中的下一个组件
) -> Result<Response, AppError> { // 返回 Result，可以是成功响应或应用错误
    // 日志记录：表明中间件已被触发。
    println!("MIDDLEWARE: JWT 认证中间件已触发。");

    // --- 步骤 1: 从 Authorization 头部提取 Token ---
    // `req.headers()`: 获取请求的 HTTP 头部，类型是 `&http::HeaderMap`。
    // `.get(header::AUTHORIZATION)`:尝试从头部映射中获取名为 `Authorization` 的头部。
    //   - `header::AUTHORIZATION` 是 `http::header`模块中定义的常量 `HeaderName`。
    //   - `.get()` 返回 `Option<&HeaderValue>`。如果头部不存在，则返回 `None`。
    // `.and_then(|header_value| ...)`: `Option` 的一个方法，如果前面的 `Option` 是 `Some(header_value)`，
    //   则执行闭包 `|header_value| ...`，闭包的返回值 (一个 `Option`) 会成为 `and_then` 的最终结果。
    //   如果前面的 `Option` 是 `None`，则 `and_then` 直接返回 `None`，不执行闭包。
    //   - `header_value.to_str()`: `HeaderValue` 类型有一个 `to_str()` 方法，尝试将其值转换为 `&str` (字符串切片)。
    //     因为 HTTP 头部的值可能包含非 UTF-8 字符，所以这个转换可能失败，返回 `Result<&str, ToStrError>`。
    //   - `.ok()`: 将 `Result<&str, ToStrError>` 转换为 `Option<&str>`。如果 `to_str()` 成功，则返回 `Some(&str)`；如果失败，则返回 `None`。
    // 最终，`auth_header_str_option` 是 `Option<&str>` 类型，包含了 `Authorization` 头部的值（如果存在且是有效字符串）。
    let auth_header_str_option = req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|header_value| header_value.to_str().ok());

    // `if let Some(auth_value_str) = auth_header_str_option { ... } else { ... }`:
    //   - 使用 `if let` 对 `Option` 进行模式匹配。
    //   - 如果 `auth_header_str_option` 是 `Some(auth_value_str)` (即头部存在且是有效字符串)，则执行 `if` 块。
    //   - 否则 (头部缺失或无效)，执行 `else` 块。
    if let Some(auth_value_str) = auth_header_str_option {
        // 检查头部值是否以 "Bearer " 前缀开头 (忽略大小写比较可能更健壮，但这里是精确匹配)。
        // `BEARER_PREFIX` 是我们定义的常量 "Bearer "。
        if !auth_value_str.starts_with(BEARER_PREFIX) {
            // 如果没有 "Bearer " 前缀，说明令牌格式不正确。
            println!("MIDDLEWARE: 无效的令牌格式 (缺少 'Bearer ' 前缀)。");
            // 直接返回 `Err(AppError::InvalidToken)`，这将短路请求处理。
            // `AppError::invalid_token()` 是我们定义的辅助函数，用于创建 `AppError::InvalidToken` 实例。
            return Err(AppError::invalid_token());
        }
        // `&auth_value_str[BEARER_PREFIX.len()..]`: 提取实际的 JWT 字符串。
        //   - `BEARER_PREFIX.len()`: 获取 "Bearer " 前缀的长度。
        //   - `&auth_value_str[...]`: 使用字符串切片语法，从前缀之后的位置开始，一直到字符串末尾，得到令牌本身。
        let token_str = auth_value_str.trim_start_matches(BEARER_PREFIX); // 更健壮的方式是 trim 前缀

        // --- 步骤 2: 解码和验证 JWT ---
        // `DecodingKey::from_secret(JWT_SECRET.as_ref())`: 创建用于解码和验证的密钥。
        //   - `JWT_SECRET.as_ref()`: 将 `JWT_SECRET` (类型 `&str`) 转换为 `&[u8]` (字节切片)，因为密钥通常是字节序列。
        //   - `DecodingKey::from_secret(...)`: `jsonwebtoken` 提供的函数，用于从原始密钥字节创建 `DecodingKey` 实例。
        //     此密钥必须与生成 JWT 时使用的 `EncodingKey` 相匹配。
        let decoding_key = DecodingKey::from_secret(JWT_SECRET.as_ref());
        // `Validation::new(Algorithm::HS256)`: 创建一个 `Validation` 对象，用于配置验证规则。
        //   - `Algorithm::HS256`: 指定期望的 JWT 签名算法是 HS256。如果令牌使用了其他算法，验证会失败。
        //   - `Validation::new(...)` 默认会检查 `exp` (过期时间) 声明 (如果存在)。
        //     可以进一步配置 `Validation` 对象，例如设置容忍的时间偏差 (`leeway`)、验证 `aud` (受众) 或 `iss` (签发者) 等。
        let validation_rules = Validation::new(Algorithm::HS256);

        // `decode::<Claims>(token_str, &decoding_key, &validation_rules)`: 尝试解码并验证 JWT。
        //   - `decode`: `jsonwebtoken` 提供的核心函数。
        //   - `::<Claims>`: 使用“涡轮鱼”语法 (turbofish) 显式指定泛型参数，告诉 `decode` 函数期望将 JWT 的载荷部分
        //     反序列化为我们定义的 `Claims` 结构体类型。`Claims` 必须实现 `serde::Deserialize`。
        //   - `token_str`: 要解码的 JWT 字符串。
        //   - `&decoding_key`: 解码密钥的引用。
        //   - `&validation_rules`: 验证规则的引用。
        //   - `decode` 函数返回 `Result<TokenData<C>, jsonwebtoken::errors::Error>`，其中 `C` 是 `Claims` 类型。
        //     `TokenData<C>` 包含头部 (`header`) 和声明 (`claims`)。
        match decode::<Claims>(token_str, &decoding_key, &validation_rules) {
            // `Ok(token_data)`: 如果 JWT 解码和验证都成功。`token_data` 是 `TokenData<Claims>` 类型。
            Ok(token_data_wrapper) => {
                // 日志记录：令牌验证成功，打印 `sub` (主体/用户ID) 声明。
                println!("MIDDLEWARE: 令牌验证成功，用户 sub: {}", token_data_wrapper.claims.sub);

                // --- 步骤 3: 将 Claims 注入请求扩展 ---
                // `req.extensions_mut()`: 获取对当前 HTTP 请求 `req` 的可变扩展数据存储的引用。
                //   请求扩展 (`http::Extensions`) 是一个类型安全的映射，允许在请求处理的不同阶段（如中间件和处理器之间）存储和传递任意类型的数据。
                // `.insert(token_data_wrapper.claims)`: 将解码后的 `Claims` 实例 (`token_data_wrapper.claims`) 插入到请求扩展中。
                //   - `token_data_wrapper.claims` 的类型是 `Claims`。
                //   - `Claims` 结构体需要实现 `Clone`，因为 `insert` 通常会获取值的所有权或克隆它。
                //     (我们在 `auth_dtos.rs` 中为 `Claims` 派生了 `Clone`)
                //   后续的请求处理函数可以通过 `Extension<Claims>` 提取器来获取这个 `Claims` 实例。
                req.extensions_mut().insert(token_data_wrapper.claims); // 将 Claims 存入，供 Handler 使用

                // --- 步骤 4: 调用下一个中间件或处理器 ---
                // `next.run(req).await`: 将（可能已修改的）请求 `req` 传递给请求处理链中的下一个组件。
                //   - `.await`: 等待下一个组件处理完成并返回响应 `Response`。
                // `Ok(...)`: 将从 `next` 返回的 `Response` 包装在 `Result::Ok` 中，作为此中间件的成功结果返回。
                Ok(next.run(req).await)
            }
            // `Err(jwt_error)`: 如果 JWT 解码或验证失败 (例如签名不匹配、已过期、格式错误等)。
            // `jwt_error` 是 `jsonwebtoken::errors::Error` 类型。
            Err(jwt_error) => {
                // 日志记录：令牌无效，并打印具体的 JWT 错误。
                println!("MIDDLEWARE: 无效的令牌: {:?} (种类: {:?})", jwt_error, jwt_error.kind());
                // 根据 JWT 错误类型，可以返回更具体的 `AppError`，但这里统一返回 `InvalidToken`。
                // 例如，可以检查 `jwt_error.kind()`:
                // match jwt_error.kind() {
                //     jsonwebtoken::errors::ErrorKind::ExpiredSignature => Err(AppError::TokenExpired), // 假设有此变体
                //     _ => Err(AppError::InvalidToken),
                // }
                // 当前，`jsonwebtoken` 的错误通常都映射为 `AppError::InvalidToken`。
                Err(AppError::invalid_token()) // 短路请求，返回无效令牌错误
            }
        }
    } else { // `auth_header_str_option` 是 `None`，即 `Authorization` 头部缺失或无法转换为字符串。
        // 日志记录：缺少认证头部。
        println!("MIDDLEWARE: 缺少 Authorization 请求头。");
        // 短路请求，返回缺少令牌错误。
        Err(AppError::missing_token())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::extract::Request;
    use axum::http::{HeaderValue, StatusCode};
    use axum::response::Response;
    use jsonwebtoken::{encode, Header, EncodingKey};
    use std::sync::Arc;
    use chrono::{Utc, Duration};
    use crate::app::model::Claims; // Ensure Claims is in scope

    // Helper to create a valid token
    fn generate_test_token(sub: String, secret: &str, expiration_hours: i64) -> String {
        let now = Utc::now();
        let iat = now.timestamp() as usize;
        let exp = (now + Duration::hours(expiration_hours)).timestamp() as usize;
        let claims = Claims { sub, exp, iat };
        encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_ref())).unwrap()
    }

    // Mock 'Next' handler that checks for claims and returns Ok or specific status
    async fn mock_next_handler(req: Request) -> Response {
        if req.extensions().get::<Claims>().is_some() {
            Response::builder().status(StatusCode::OK).body(Body::empty()).unwrap()
        } else {
            Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Body::from("Claims not found by mock_next_handler")).unwrap()
        }
    }

    #[tokio::test]
    async fn test_jwt_middleware_valid_token() {
        let valid_token = generate_test_token("user123".to_string(), JWT_SECRET, 1);
        let mut req = Request::builder()
            .header(header::AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", valid_token)).unwrap())
            .body(Body::empty())
            .unwrap();

        let next = Next::new(Arc::new(move |req| Box::pin(mock_next_handler(req))));

        let result = jwt_auth_middleware(req, next).await;
        assert!(result.is_ok(), "Middleware should pass for valid token");
        if let Ok(response) = result {
            assert_eq!(response.status(), StatusCode::OK, "Next handler should have been called successfully");
        }
    }

    #[tokio::test]
    async fn test_jwt_middleware_missing_header() {
        let req = Request::builder().body(Body::empty()).unwrap();
        let next = Next::new(Arc::new(move |req| Box::pin(mock_next_handler(req))));

        let result = jwt_auth_middleware(req, next).await;
        assert!(result.is_err(), "Middleware should fail if auth header is missing");
        match result.err().unwrap() {
            AppError::MissingToken => {} // Expected
            _ => panic!("Expected MissingToken error"),
        }
    }

    #[tokio::test]
    async fn test_jwt_middleware_invalid_token_format_no_bearer() {
        let req = Request::builder()
            .header(header::AUTHORIZATION, HeaderValue::from_static("InvalidTokenFormat"))
            .body(Body::empty())
            .unwrap();
        let next = Next::new(Arc::new(move |req| Box::pin(mock_next_handler(req))));

        let result = jwt_auth_middleware(req, next).await;
        assert!(result.is_err(), "Middleware should fail for invalid token format");
        match result.err().unwrap() {
            AppError::InvalidToken => {} // Expected
            _ => panic!("Expected InvalidToken error for bad format"),
        }
    }

    #[tokio::test]
    async fn test_jwt_middleware_token_verification_failure_wrong_secret() {
        let token_wrong_secret = generate_test_token("user456".to_string(), "a-different-secret", 1);
        let mut req = Request::builder()
            .header(header::AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", token_wrong_secret)).unwrap())
            .body(Body::empty())
            .unwrap();
        let next = Next::new(Arc::new(move |req| Box::pin(mock_next_handler(req))));

        let result = jwt_auth_middleware(req, next).await;
        assert!(result.is_err(), "Middleware should fail for token signed with wrong secret");
        match result.err().unwrap() {
            AppError::InvalidToken => {} // Expected
            _ => panic!("Expected InvalidToken error for verification failure"),
        }
    }

    #[tokio::test]
    async fn test_jwt_middleware_expired_token() {
        let expired_token = generate_test_token("user789".to_string(), JWT_SECRET, -1); // Expired 1 hour ago
        let mut req = Request::builder()
            .header(header::AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", expired_token)).unwrap())
            .body(Body::empty())
            .unwrap();
        let next = Next::new(Arc::new(move |req| Box::pin(mock_next_handler(req))));

        let result = jwt_auth_middleware(req, next).await;
        assert!(result.is_err(), "Middleware should fail for expired token");
         match result.err().unwrap() {
            AppError::InvalidToken => {} // Expected (jsonwebtoken crate maps ExpiredSignature to InvalidToken)
            _ => panic!("Expected InvalidToken error for expired token"),
        }
    }
}
