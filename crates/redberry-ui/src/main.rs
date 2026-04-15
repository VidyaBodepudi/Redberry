use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use redberry_core::{ContextMessage, RedberryConfig, SessionContext};
use redberry_embed::ContextCache;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
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
        .route("/", get(serve_ui))
        .route("/api/stats", get(get_stats))
        .route("/api/prompts", get(get_prompts))
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

    let all_messages = cache.get_all_messages().unwrap_or_default();
    let total_prompts = all_messages.len();

    let mut avg_vagueness = 0.0;
    let mut avg_syntax = 0.0;
    let mut avg_drift = 0.0;
    let mut total_rejected = 0;

    for (_, msg) in &all_messages {
        avg_vagueness += msg.metrics_vagueness;
        avg_syntax += msg.metrics_syntax;
        avg_drift += msg.metrics_drift;
        if msg.snark_response.is_some() {
            total_rejected += 1;
        }
    }

    if total_prompts > 0 {
        let count = total_prompts as f32;
        avg_vagueness /= count;
        avg_syntax /= count;
        avg_drift /= count;
    }

    let total_approved = total_prompts - total_rejected;

    let stats = StatsResponse {
        total_prompts,
        avg_vagueness,
        avg_syntax,
        avg_drift,
        total_approved,
        total_rejected,
    };

    (StatusCode::OK, Json(stats))
}

async fn get_prompts(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let cache = state.cache.lock().await;
    let all_messages = cache.get_all_messages().unwrap_or_default();

    let prompts = all_messages
        .into_iter()
        .map(|(session_id, msg)| EvaluatedPrompt {
            session_id,
            text: msg.text,
            snark_response: msg.snark_response,
            metrics_vagueness: msg.metrics_vagueness,
            metrics_syntax: msg.metrics_syntax,
            metrics_drift: msg.metrics_drift,
            created_at: msg.created_at,
        })
        .collect();

    let res = PromptsResponse { prompts };
    (StatusCode::OK, Json(res))
}

async fn serve_ui() -> impl IntoResponse {
    // We will inline the raw React HTML file via include_str!
    let html = include_str!("../index.html");
    axum::response::Html(html)
}
