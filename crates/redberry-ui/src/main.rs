use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use redberry_core::RedberryConfig;
use redberry_embed::ContextCache;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tracing::{error, info};

struct AppState {
    cache: Arc<Mutex<ContextCache>>,
}

#[derive(Serialize)]
struct StatsResponse {
    total_prompts: usize,
    avg_vagueness: f32,
    avg_syntax: f32,
    avg_drift: f32,
    avg_coherence: f32,
    avg_specificity: f32,
    total_approved: usize,
    total_rejected: usize,
}

#[derive(Serialize)]
struct EvaluatedPrompt {
    session_id: String,
    text: String,
    snark_response: Option<String>,
    metrics_vagueness: f32,
    metrics_syntax: f32,
    metrics_drift: f32,
    metrics_coherence: f32,
    metrics_specificity: f32,
    created_at: Option<i64>,
}

#[derive(Serialize)]
struct PromptsResponse {
    prompts: Vec<EvaluatedPrompt>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = RedberryConfig::load().unwrap_or_default();
    let db_path = config.resolved_db_path();

    info!(
        "Starting Redberry UI API. Connecting to cache at: {}",
        db_path.display()
    );

    let cache = match ContextCache::new(&db_path) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to initialize DB: {}", e);
            std::process::exit(1);
        }
    };

    let state = Arc::new(AppState {
        cache: Arc::new(Mutex::new(cache)),
    });

    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any);

    let app = Router::new()
        .route("/api/stats", get(get_stats))
        .route("/api/prompts", get(get_prompts))
        .fallback_service(
            ServeDir::new("crates/redberry-ui/dashboard/dist")
                .fallback(ServeFile::new("crates/redberry-ui/dashboard/dist/index.html")),
        )
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8443")
        .await
        .unwrap();
    info!(
        "Redberry UI API listening on {}",
        listener.local_addr().unwrap()
    );
    axum::serve(listener, app).await.unwrap();
}

async fn get_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let cache = state.cache.lock().await;

    // Fixed Vulnerability: Do not pull all rows into memory! 
    // Aggregate queries directly on the DB if it grows to millions of rows.
    let stats_data = cache.get_global_stats().unwrap_or((0, 0.0, 0.0, 0.0, 0.0, 0.0, 0));

    let stats = StatsResponse {
        total_prompts: stats_data.0,
        avg_vagueness: stats_data.1,
        avg_syntax: stats_data.2,
        avg_drift: stats_data.3,
        avg_coherence: stats_data.4,
        avg_specificity: stats_data.5,
        total_approved: stats_data.0.saturating_sub(stats_data.6),
        total_rejected: stats_data.6,
    };

    (StatusCode::OK, Json(stats))
}

async fn get_prompts(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let cache = state.cache.lock().await;
    // Fixed Vulnerability: Limit API vector boundary to latest 1000 items
    let all_messages = cache.get_recent_messages(1000).unwrap_or_default();

    let prompts = all_messages
        .into_iter()
        .map(|(session_id, msg)| EvaluatedPrompt {
            session_id,
            text: msg.text,
            snark_response: msg.snark_response,
            metrics_vagueness: msg.metrics_vagueness,
            metrics_syntax: msg.metrics_syntax,
            metrics_drift: msg.metrics_drift,
            metrics_coherence: msg.metrics_coherence,
            metrics_specificity: msg.metrics_specificity,
            created_at: msg.created_at,
        })
        .collect();

    let res = PromptsResponse { prompts };
    (StatusCode::OK, Json(res))
}
