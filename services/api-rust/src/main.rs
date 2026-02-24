use axum::{extract::DefaultBodyLimit, routing::{get, post}, Router};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

mod routes;

/// A pending email verification entry.
pub struct VerificationEntry {
    pub email: String,
    pub domain: String,
    pub content_hash: String,
    pub verified: bool,
    pub created_at: Instant,
}

/// Shared application state.
pub struct AppState {
    /// Directory containing trust anchor PEM files.
    pub trust_dir: String,
    /// Path to the prover binary (cargo project root).
    pub prover_dir: String,
    /// Solana RPC URL.
    pub rpc_url: String,
    /// Solana keypair path for submitting transactions.
    pub keypair_path: String,
    /// Solana program ID.
    pub program_id: String,
    /// In-memory email verification state, keyed by token.
    pub verifications: Mutex<HashMap<String, VerificationEntry>>,
}

#[tokio::main]
async fn main() {
    // Load .env from project root (two levels up from services/api)
    let _ = dotenvy::from_path("../../.env");
    tracing_subscriber::fmt::init();

    let state = Arc::new(AppState {
        trust_dir: std::env::var("TRUST_DIR")
            .unwrap_or_else(|_| "../../data/trust".to_string()),
        prover_dir: std::env::var("PROVER_DIR")
            .unwrap_or_else(|_| "../prover".to_string()),
        rpc_url: std::env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8899".to_string()),
        keypair_path: std::env::var("SOLANA_KEYPAIR_PATH")
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_default();
                format!("{home}/.config/solana/id.json")
            }),
        program_id: std::env::var("PROGRAM_ID")
            .unwrap_or_else(|_| "HahVgC9uo73aLw1ouBEvgMT7KmGTS6rovfbKP9zuCtjc".to_string()),
        verifications: Mutex::new(HashMap::new()),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "./static".to_string());

    let app = Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .route("/api/verify", post(routes::verify::verify))
        .route("/api/attest", post(routes::attest::attest))
        .route("/api/prove", post(routes::prove::prove))
        .route("/api/submit", post(routes::submit::submit))
        .route("/api/attestations", get(routes::attestation::list_all))
        .route("/api/attestation/{hash}", get(routes::attestation::lookup))
        .route("/api/identity/start", post(routes::identity::start))
        .route("/api/identity/verify/{token}", get(routes::identity::verify_email))
        .route("/api/identity/status/{token}", get(routes::identity::status))
        .route("/api/identity/attest", post(routes::identity::attest_identity))
        .fallback_service(
            ServeDir::new(&static_dir)
                .not_found_service(ServeFile::new(format!("{static_dir}/index.html"))),
        )
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024)) // 50 MB
        .layer(cors)
        .with_state(state);

    let addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3001".to_string());
    tracing::info!("API listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
