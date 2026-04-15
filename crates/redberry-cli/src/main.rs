use anyhow::Result;
use clap::{Parser, Subcommand};
use redberry_analyze::analyze_prompt;
use redberry_core::config::ModelPreset;
use redberry_core::RedberryConfig;
use redberry_embed::{ensure_model_files, EmbeddingEngine};
use redberry_persona::PersonalityEngine;
use std::process::Command;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Parser)]
#[command(name = "redberry")]
#[command(about = "Contrarian Conversationalist Engine & MCP Server", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Download required model files for Redberry based on the config.
    Setup {
        /// Force a specific preset (tier1, tier2)
        #[arg(short, long)]
        preset: Option<String>,
    },
    /// Run the Redberry MCP server (default stdio transport).
    Serve,
    /// Manually analyze a prompt and see the verdict.
    Analyze {
        /// The prompt to analyze.
        prompt: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_target(false);

    tracing_subscriber::registry()
        .with(filter)
        .with(layer)
        .init();

    let cli = Cli::parse();
    let config = RedberryConfig::load().unwrap_or_else(|e| {
        error!("Config load error, using defaults: {}", e);
        RedberryConfig::default()
    });

    match cli.command {
        Commands::Setup { preset } => {
            let preset_str = if let Some(p) = preset {
                p
            } else {
                use std::io::{self, Write};
                println!("Welcome to the Redberry installation.");
                println!("Please select your embedding engine tier:");
                println!("  [1] Tier 1 - Standard (bge-small-en-v1.5, ~33MB, Lightning Fast)");
                println!("  [2] Tier 2 - Quality (bge-base-en-v1.5, ~110MB, Maximum Reasoning)");
                print!("Enter your choice [1 or 2, default is 1]: ");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                match input.trim() {
                    "2" => "tier2".to_string(),
                    _ => "tier1".to_string(),
                }
            };

            let preset_enum = ModelPreset::parse_str(&preset_str).ok_or_else(|| {
                anyhow::anyhow!("Unknown preset '{}'. Use tier1 or tier2.", preset_str)
            })?;

            info!("Starting setup for preset: {}", preset_enum.model_name());
            let models_dir = RedberryConfig::default_models_dir();

            tokio::task::spawn_blocking(move || ensure_model_files(preset_enum, &models_dir))
                .await
                .unwrap()?;

            info!("Setup complete. Redberry is ready.");
        }
        Commands::Serve => {
            info!("Starting Redberry Server...");

            // Re-exec into the redberry-server binary.
            // When installed, redberry-server should be alongside redberry in PATH.
            // For dev, we just run the cargo bin via Command if it exists, but typically
            // a single workspace might just spawn the server binary directly.

            // To support both development (cargo run) and installed binary (cargo install),
            // let's try calling `redberry-server` in PATH, or fallback to cargo run.
            let status = Command::new("redberry-server")
                .spawn()
                .or_else(|_| {
                    Command::new("cargo")
                        .args(["run", "--bin", "redberry-server", "--quiet"])
                        .spawn()
                })?
                .wait()?;

            if !status.success() {
                error!("Server exited with status: {}", status);
            }
        }
        Commands::Analyze { prompt } => {
            info!("Analyzing prompt: '{}'", prompt);

            let resolved_model = config.resolve_model()?;
            let persona = PersonalityEngine::new(config);
            
            // CLI Fallback behavior
            let mut fallback = true;

            if resolved_model.onnx_path.exists() {
                if let Ok(engine) = EmbeddingEngine::load(resolved_model) {
                    let db_path = redberry_core::RedberryConfig::load().unwrap_or_default().resolved_db_path();
                    if let Ok(mut cache) = redberry_embed::ContextCache::new(&db_path) {
                        let session_id = "cli_default_session".to_string();
                        
                        if let Ok(verdict) = redberry_pipeline::evaluate_pipeline(&prompt, &session_id, &engine, &mut cache, &persona) {
                            println!("\n========= [ Final Verdict ] =========");
                            let verdict_json = serde_json::to_string_pretty(&verdict).unwrap();
                            println!("{}\n", verdict_json);
                            fallback = false;
                        }
                    }
                }
            }
            
            if fallback {
                info!("No model/cache found or engine faulted. Running stateless text analysis...");
                let analysis = analyze_prompt(&prompt);
                
                println!("\n========= [ Analysis Report ] =========");
                let analysis_json = serde_json::to_string_pretty(&analysis)?;
                println!("{}\n", analysis_json);

                println!("========= [ Final Verdict ] =========");
                let verdict = persona.generate_verdict(&analysis);
                let verdict_json = serde_json::to_string_pretty(&verdict)?;
                println!("{}\n", verdict_json);
            }
        }
    }

    Ok(())
}
