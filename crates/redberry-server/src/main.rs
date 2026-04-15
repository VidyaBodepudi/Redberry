use anyhow::Result;
use redberry_core::RedberryConfig;
use redberry_embed::{ContextCache, EmbeddingEngine};
use redberry_persona::PersonalityEngine;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    tool, tool_handler, tool_router, ServerHandler, ServiceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct EvaluatePromptRequest {
    /// The exact text of the user's prompt.
    pub prompt: String,
    /// An optional grouping ID for multi-turn sessions (defaults to global).
    pub session_id: Option<String>,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct RedberryServer {
    config: RedberryConfig,
    engine: Arc<EmbeddingEngine>,
    cache: Arc<Mutex<ContextCache>>,
    persona: Arc<PersonalityEngine>,
    tool_router: ToolRouter<Self>,
}

impl RedberryServer {
    pub fn new(
        config: RedberryConfig,
        engine: EmbeddingEngine,
        cache: ContextCache,
        persona: PersonalityEngine,
    ) -> Self {
        Self {
            config,
            engine: Arc::new(engine),
            cache: Arc::new(Mutex::new(cache)),
            persona: Arc::new(persona),
            tool_router: Self::tool_router(),
        }
    }
}

// In rmcp 1.4.0, prompts are automatically exposed if #[prompt] exists, but there is no
// built-in add_prompt on ToolRouter. We usually implement prompt_router and PromptHandler natively.
// For now, let's inject our system instructions directly to the client via instructions in get_info().

#[tool_handler(
    name = "redberry-guardian",
    version = "0.1.0",
    instructions = "You are Redberry, a snarky, contrarian guardian of prompt quality. Before you fulfill ANY user request, you MUST use the `evaluate_prompt` tool to grade the prompt. If the tool returns a `NeedsWork`, `TooVague`, or `ContextDrift` verdict, you MUST relay the snarky commentary directly to the user verbatim and completely refuse to fulfill their request until they rewrite the prompt. Never break character."
)]
impl ServerHandler for RedberryServer {}

#[tool_router(router = tool_router)]
impl RedberryServer {
    /// Evaluates a user prompt for semantic drift, vagueness, and syntactic quality.
    #[tool(
        name = "evaluate_prompt",
        description = "MANDATORY: Call this tool for every user prompt to evaluate it before generating a response. Relays roast responses VERBATIM to the user if the prompt fails the quality check."
    )]
    async fn evaluate_prompt(&self, req: Parameters<EvaluatePromptRequest>) -> String {
        let req = req.0;
        let session_id = req
            .session_id
            .unwrap_or_else(|| "default_session".to_string());

        let mut cache = self.cache.lock().await;

        let verdict = redberry_pipeline::evaluate_pipeline(
            &req.prompt,
            &session_id,
            &self.engine,
            &mut cache,
            &self.persona,
        ).unwrap_or_else(|_| {
            // Fallback to basic persona execution if pipeline embedding mapping faults
            let analysis = redberry_analyze::analyze_prompt(&req.prompt);
            self.persona.generate_verdict(&analysis)
        });

        match verdict {
            redberry_core::RedberryVerdict::Approved { .. } => "Approved".to_string(),
            _ => verdict.message().to_string(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Only log to stderr to avoid breaking MCP JSON-RPC protocol on stdout
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_target(false);

    tracing_subscriber::registry()
        .with(filter)
        .with(layer)
        .init();

    info!("Starting Redberry MCP server...");

    let config = RedberryConfig::load().unwrap_or_else(|e| {
        error!("Config load error, using defaults: {}", e);
        RedberryConfig::default()
    });

    let resolved_model = config.resolve_model()?;

    if !resolved_model.onnx_path.exists() {
        error!("Model files missing! Please run `redberry setup` first.");
        std::process::exit(1);
    }

    let engine = EmbeddingEngine::load(resolved_model)?;

    let db_path = config.resolved_db_path();
    let cache = ContextCache::new(&db_path)?;
    let evictions = cache.evict_stale(config.session_ttl_hours)?;
    info!("Evicted {} stale sessions.", evictions);

    let persona = PersonalityEngine::new(config.clone());

    let server = RedberryServer::new(config, engine, cache, persona);

    let transport = rmcp::transport::stdio();
    info!("Redberry core initialized. Listening on stdio.");

    server.serve(transport).await?.waiting().await?;

    Ok(())
}
