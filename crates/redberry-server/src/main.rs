use anyhow::Result;
use redberry_analyze::analyze_prompt;
use redberry_core::{ContextMessage, RedberryConfig};
use redberry_embed::{ContextCache, EmbeddingEngine};
use redberry_persona::PersonalityEngine;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    tool, tool_handler, tool_router,
    ServerHandler, ServiceExt,
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
        let session_id = req.session_id.unwrap_or_else(|| "default_session".to_string());

        let mut analysis = analyze_prompt(&req.prompt);

        // Perform semantic embedding and drift calculation
        match self.engine.embed_text(&req.prompt) {
            Ok(embedding) => {
                let mut cache = self.cache.lock().await;
                if let Ok(Some(ctx)) = cache.get_context(&session_id) {
                    if !ctx.messages.is_empty() {
                        let recent_messages = ctx.messages.iter().rev().take(5).collect::<Vec<_>>();
                        let mut centroid = vec![0.0f32; embedding.len()];
                        for msg in &recent_messages {
                            for (i, &v) in msg.embedding.iter().enumerate() {
                                centroid[i] += v;
                            }
                        }
                        for v in &mut centroid {
                            *v /= recent_messages.len() as f32;
                        }

                        let drift = redberry_embed::similarity::cosine_similarity(&embedding, &centroid);
                        analysis.drift_score = Some(drift);
                    }
                }

                let msg = ContextMessage {
                    text: req.prompt.clone(),
                    embedding,
                };
                let _ = cache.append_messages(&session_id, &[msg]);
            }
            Err(e) => {
                error!("Embedding inference failed: {}", e);
            }
        }

        let verdict = self.persona.generate_verdict(&analysis);
        serde_json::to_string_pretty(&verdict).unwrap_or_else(|_| "{}".to_string())
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
