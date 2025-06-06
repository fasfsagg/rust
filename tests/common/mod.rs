use axum::Router;
use axum_tutorial::{
    app::AppState,
    config::AppConfig,
    db::{establish_connection, run_migrations},
    startup,
};
use once_cell::sync::Lazy;
use sea_orm::DatabaseConnection;
use std::env;
use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

// Initialize tracing for tests if not already initialized
// Using ONCE_CELL to ensure this only runs once.
static TRACING: Lazy<()> = Lazy::new(|| {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    // let formatter = fmt::layer().json(); // Example: JSON output
    let formatter = fmt::layer().compact(); // More concise output for tests
    tracing_subscriber::registry()
        .with(filter)
        .with(formatter)
        .init();
});

/// Test context containing the app router and state.
pub struct TestContext {
    pub app_state: AppState,
    pub router: Router,
    // Optionally, store the db_conn separately if needed for direct db ops in tests
    // pub db_conn: DatabaseConnection,
    // db_path: Option<String>, // To store path for file-based DBs for teardown
}

/// Sets up the application for testing.
///
/// This function initializes a test-specific configuration, establishes an
/// in-memory SQLite database connection (or a file-based one if needed),
/// runs migrations, and creates the application state and router.
///
/// # Returns
/// A `TestContext` containing the `AppState` and `Router`.
pub async fn setup_test_app() -> TestContext {
    // Ensure tracing is initialized for test runs
    Lazy::force(&TRACING);

    // Override DATABASE_URL for tests to use an in-memory SQLite database
    // Generate a unique name to ensure test isolation if not using :memory:
    // For now, always use :memory:
    let test_db_url = "sqlite::memory:";
    // let random_id = uuid::Uuid::new_v4().to_string();
    // let test_db_file = format!("./test_db_{}.sqlite", random_id);
    // let test_db_url_file = format!("sqlite:{}?mode=rwc", test_db_file);

    env::set_var("DATABASE_URL", test_db_url);
    // If using a file DB, also set a unique path for it
    // env::set_var("DATABASE_URL", &test_db_url_file);


    // Load or create a test-specific AppConfig
    // We can use from_env() and it will pick up the overridden DATABASE_URL
    // Or construct one manually for more control
    let mut config = AppConfig::from_env();
    // Ensure JWT secret is consistent for tests if needed, or use the one from_env
    config.jwt_secret = "test_jwt_secret_key_for_deterministic_tests".to_string();
    config.jwt_expiration_seconds = 3600; // 1 hour, or shorter for expiration tests

    // Establish database connection
    let db_conn = establish_connection().await.expect("Failed to establish test DB connection");

    // Run migrations
    run_migrations(&db_conn).await.expect("Failed to run migrations on test DB");

    // Create AppState
    let app_state = AppState {
        db: db_conn.clone(), // Clone for AppState
        config: config.clone(),
    };

    // Create router using startup::init_app logic but with our test-specific AppState parts
    // Note: startup::init_app itself creates AppState. We might need to adjust startup::init_app
    // or replicate parts of its logic if it doesn't allow injecting a pre-built db_conn/config easily.
    // For now, let's assume we can pass db_conn and config to a modified init_app or build router directly.

    // Re-using startup::init_app:
    // init_app expects AppConfig, and internally calls establish_connection based on env var.
    // This is fine as we've set DATABASE_URL. It will also create AppState.
    // We need to ensure the config used by init_app is our test config.
    // The `config` variable here is passed to `init_app`.
    let router = startup::init_app(config).await; // init_app creates its own AppState internally.

    // To use the AppState we created (e.g. if we wanted to return db_conn separately):
    // We would need init_app to accept a pre-configured AppState or components.
    // For now, the AppState created *inside* init_app is the one the router uses.
    // To return *that* AppState, we'd need to modify init_app or have another way.
    // Let's adjust to what init_app provides. init_app should return AppState as well or make it accessible.
    // For now, the router is the main thing. We can recreate AppState for direct DB access if needed,
    // or modify init_app.
    // A simpler approach for tests: init_app creates and returns the router, and we create a
    // new AppState instance *just for test assertions or direct DB manipulation* if needed,
    // ensuring it uses the same config (especially DB_URL).

    // Let's assume for now the router is the primary artifact needed for request testing.
    // If tests need to directly query DB, they can establish their own connection
    // to the same :memory: instance (though multiple :memory: are distinct).
    // The best way is for init_app to return AppState or for setup_test_app to create the router
    // by composing routes and middleware with its own AppState.
    // The current init_app in startup.rs creates AppState and passes it to routes.
    // We need that AppState.
    // A temporary solution: make init_app also return AppState or make AppState accessible from Router extensions.
    // Create AppState using the established connection and test config
    let app_state = AppState {
        db: db_conn, // Use the connection we established
        config: config.clone(),
    };

    // Build the router using the routes and middleware setup from startup.rs
    // This replicates parts of init_app but ensures our AppState instance is used.
    let middleware_stack = tower::ServiceBuilder::new()
        .layer(axum_tutorial::app::middleware::trace_layer()) // Adjust path as needed
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        );

    let router = axum_tutorial::routes::create_routes(app_state.clone())
        .layer(middleware_stack);

    TestContext {
        app_state,
        router,
    }
}

/*
// Example of teardown for file-based DBs
pub async fn tear_down_test_db(_context: TestContext) { // Mark context as unused for now
    // context.db_conn.close().await.expect("Failed to close test DB connection");
    // if let Some(path) = context.db_path {
    //     std::fs::remove_file(path).expect("Failed to delete test DB file");
    // }
}
*/

// --- HTTP Request Utilities for Tests ---
// Moved from auth_tests.rs

// Make items public for use in other test modules
pub use http_body_util::BodyExt;
pub use tower::ServiceExt; // For `oneshot`

pub use http::{Request, Response, header, StatusCode};
pub use axum::body::Body; // Axum's body type
pub use serde::Serialize;
pub use serde::de::DeserializeOwned; // For deserializing response bodies
// Removed: use hyper_util::client::legacy::Client;
// Removed: use hyper_util::rt::TokioExecutor;


#[derive(Debug)] // Added Debug for easier test failures
pub struct TestResponse {
    status: StatusCode,
    body_bytes: Option<bytes::Bytes>, // Store body to allow multiple reads if necessary
}

impl TestResponse {
    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub async fn json_body<T: DeserializeOwned>(&self) -> T {
        let bytes = self.body_bytes.as_ref().expect("Response body already consumed or not captured");
        serde_json::from_slice(bytes).unwrap_or_else(|e| {
            panic!("Failed to deserialize JSON body: {:?}, body: '{}'", e, String::from_utf8_lossy(bytes));
        })
    }

    #[allow(dead_code)]
    pub async fn text_body(&self) -> String {
        let bytes = self.body_bytes.as_ref().expect("Response body already consumed or not captured");
        String::from_utf8(bytes.to_vec()).expect("Failed to read body as text")
    }
}

async fn process_response(response: Response<Body>) -> TestResponse {
    let status = response.status();
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    TestResponse { status, body_bytes: Some(body_bytes) }
}

// Generic request function for POST
pub async fn request_post<T: Serialize>(
    uri: &str,
    payload: &T,
    token: Option<&str>,
    router: Router,
) -> TestResponse {
    let body_json = serde_json::to_string(payload).unwrap();
    let mut builder = Request::builder().method("POST").uri(uri)
        .header(header::CONTENT_TYPE, "application/json");

    if let Some(t) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {}", t));
    }

    let req = builder.body(Body::from(body_json)).unwrap();

    let response = router.oneshot(req).await.unwrap();
    process_response(response).await
}

// Common Error Response Structs for test assertions
#[derive(serde::Deserialize, Debug)]
pub struct ErrorResponseDetail {
    pub message: String,
    pub code: u16,
}
#[derive(serde::Deserialize, Debug)]
pub struct ErrorResponse {
    pub error: ErrorResponseDetail,
}

// Generic request function for GET
pub async fn request_get(
    uri: &str,
    token: Option<&str>,
    router: Router,
) -> TestResponse {
    let mut builder = Request::builder().method("GET").uri(uri);
    if let Some(t) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {}", t));
    }
    let req = builder.body(Body::empty()).unwrap();
    let response = router.oneshot(req).await.unwrap();
    process_response(response).await
}

// Generic request function for PUT
pub async fn request_put<T: Serialize>(
    uri: &str,
    payload: &T,
    token: Option<&str>,
    router: Router,
) -> TestResponse {
    let body_json = serde_json::to_string(payload).unwrap();
    let mut builder = Request::builder().method("PUT").uri(uri)
        .header(header::CONTENT_TYPE, "application/json");

    if let Some(t) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {}", t));
    }

    let req = builder.body(Body::from(body_json)).unwrap();

    let response = router.oneshot(req).await.unwrap();
    process_response(response).await
}

// Generic request function for DELETE
pub async fn request_delete(
    uri: &str,
    token: Option<&str>,
    router: Router,
) -> TestResponse {
    let mut builder = Request::builder().method("DELETE").uri(uri);
    if let Some(t) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {}", t));
    }
    let req = builder.body(Body::empty()).unwrap();
    let response = router.oneshot(req).await.unwrap();
    process_response(response).await
}

[end of tests/common/mod.rs]
