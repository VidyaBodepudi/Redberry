//! Configuration loading and model preset management.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::RedberryError;

/// Top-level configuration file structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    pub redberry: RedberryConfig,
}

/// Core Redberry configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedberryConfig {
    /// Snark intensity level (1–5). Default: 3.
    /// 1 = polite but pointed
    /// 2 = passive-aggressive
    /// 3 = snarky constructive (default)
    /// 4 = full roast
    /// 5 = unhinged
    #[serde(default = "default_sass_level")]
    pub sass_level: u8,

    /// Cosine similarity threshold — below this triggers context drift warning.
    /// Range: 0.0–1.0. Default: 0.3.
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f32,

    /// Vagueness score threshold — above this triggers vagueness roast.
    /// Range: 0.0–1.0. Default: 0.6.
    #[serde(default = "default_vagueness_threshold")]
    pub vagueness_threshold: f32,

    /// Path to the SQLite context database.
    #[serde(default = "default_context_db_path")]
    pub context_db_path: String,

    /// Session time-to-live in hours. Default: 24.
    #[serde(default = "default_session_ttl_hours")]
    pub session_ttl_hours: u32,

    /// Model configuration.
    #[serde(default)]
    pub model: ModelConfig,
}

/// Model configuration — either a preset name or custom paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Preset name: "compact", "balanced", or "quality".
    #[serde(default = "default_preset")]
    pub preset: String,

    /// Custom ONNX model path (overrides preset).
    pub onnx_path: Option<String>,

    /// Custom tokenizer.json path (overrides preset).
    pub tokenizer_path: Option<String>,

    /// Embedding dimension — for MRL-capable models, can truncate to smaller dims.
    /// Common values: 256 (fast), 384 (compact models), 768 (full precision).
    pub embedding_dim: Option<usize>,
}

/// Resolved model paths ready for use by the embedding engine.
#[derive(Debug, Clone)]
pub struct ResolvedModelConfig {
    pub name: String,
    pub onnx_path: PathBuf,
    pub tokenizer_path: PathBuf,
    pub embedding_dim: usize,
}

/// Available model presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelPreset {
    /// Tier 1: Standard (bge-small-en-v1.5 INT8, ~33MB), 384 dims
    Tier1,
    /// Tier 2: Quality (bge-base-en-v1.5 INT8, ~110MB), 768 dims
    Tier2,
}

// === Default value functions for serde ===

fn default_sass_level() -> u8 {
    3
}

fn default_similarity_threshold() -> f32 {
    0.3
}

fn default_vagueness_threshold() -> f32 {
    0.6
}

fn default_context_db_path() -> String {
    "~/.local/share/redberry/context.db".to_string()
}

fn default_session_ttl_hours() -> u32 {
    24
}

fn default_preset() -> String {
    "tier1".to_string()
}

// === Implementations ===

impl Default for RedberryConfig {
    fn default() -> Self {
        Self {
            sass_level: default_sass_level(),
            similarity_threshold: default_similarity_threshold(),
            vagueness_threshold: default_vagueness_threshold(),
            context_db_path: default_context_db_path(),
            session_ttl_hours: default_session_ttl_hours(),
            model: ModelConfig::default(),
        }
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            preset: default_preset(),
            onnx_path: None,
            tokenizer_path: None,
            embedding_dim: None,
        }
    }
}

impl ModelPreset {
    /// Parse a preset name string.
    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "tier1" | "standard" => Some(Self::Tier1),
            "tier2" | "quality" => Some(Self::Tier2),
            _ => None,
        }
    }

    /// Human-readable model name for this preset.
    pub fn model_name(&self) -> &'static str {
        match self {
            Self::Tier1 => "Tier 1 - Standard (bge-small-en-v1.5 INT8)",
            Self::Tier2 => "Tier 2 - Quality (bge-base-en-v1.5 INT8)",
        }
    }

    /// HuggingFace model ID for downloading.
    pub fn hf_model_id(&self) -> &'static str {
        match self {
            Self::Tier1 => "Xenova/bge-small-en-v1.5",
            Self::Tier2 => "Xenova/bge-base-en-v1.5",
        }
    }

    /// Default embedding dimension for this preset.
    pub fn default_dim(&self) -> usize {
        match self {
            Self::Tier1 => 384,
            Self::Tier2 => 768,
        }
    }

    /// Approximate model size in MB.
    pub fn approx_size_mb(&self) -> u32 {
        match self {
            Self::Tier1 => 33,
            Self::Tier2 => 110,
        }
    }
}

impl RedberryConfig {
    /// Load configuration from the default path (~/.config/redberry/config.toml).
    /// Falls back to defaults if the file doesn't exist.
    pub fn load() -> Result<Self, RedberryError> {
        let config_path = Self::default_config_path();
        if config_path.exists() {
            Self::load_from(&config_path)
        } else {
            tracing::info!(
                "No config file found at {}, using defaults",
                config_path.display()
            );
            Ok(Self::default())
        }
    }

    /// Load configuration from a specific file path.
    pub fn load_from(path: &Path) -> Result<Self, RedberryError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            RedberryError::Config(format!(
                "Failed to read config file {}: {}",
                path.display(),
                e
            ))
        })?;
        let config_file: ConfigFile = toml::from_str(&content).map_err(|e| {
            RedberryError::Config(format!(
                "Failed to parse config file {}: {}",
                path.display(),
                e
            ))
        })?;
        config_file.redberry.validate()?;
        Ok(config_file.redberry)
    }

    /// Validate the configuration values are within acceptable ranges.
    pub fn validate(&self) -> Result<(), RedberryError> {
        if self.sass_level < 1 || self.sass_level > 5 {
            return Err(RedberryError::Config(format!(
                "sass_level must be 1–5, got {}",
                self.sass_level
            )));
        }
        if self.similarity_threshold < 0.0 || self.similarity_threshold > 1.0 {
            return Err(RedberryError::Config(format!(
                "similarity_threshold must be 0.0–1.0, got {}",
                self.similarity_threshold
            )));
        }
        if self.vagueness_threshold < 0.0 || self.vagueness_threshold > 1.0 {
            return Err(RedberryError::Config(format!(
                "vagueness_threshold must be 0.0–1.0, got {}",
                self.vagueness_threshold
            )));
        }
        Ok(())
    }

    /// Resolve the model configuration into concrete paths.
    pub fn resolve_model(&self) -> Result<ResolvedModelConfig, RedberryError> {
        // If custom paths are provided, use those directly
        if let (Some(onnx), Some(tok)) = (&self.model.onnx_path, &self.model.tokenizer_path) {
            let dim = self.model.embedding_dim.unwrap_or(768);
            return Ok(ResolvedModelConfig {
                name: "tier1".to_string(),
                onnx_path: expand_tilde(onnx),
                tokenizer_path: expand_tilde(tok),
                embedding_dim: dim,
            });
        }

        // Otherwise, resolve from preset
        let preset = ModelPreset::parse_str(&self.model.preset).ok_or_else(|| {
            RedberryError::Config(format!(
                "Invalid model preset '{}'. Valid: tier1, tier2",
                self.model.preset
            ))
        })?;

        let models_dir = Self::default_models_dir();
        let dim = self.model.embedding_dim.unwrap_or(preset.default_dim());

        Ok(ResolvedModelConfig {
            name: preset.model_name().to_string(),
            onnx_path: models_dir.join("model.onnx"),
            tokenizer_path: models_dir.join("tokenizer.json"),
            embedding_dim: dim,
        })
    }

    /// Default config file path: ~/.config/redberry/config.toml
    pub fn default_config_path() -> PathBuf {
        dirs_fallback("config").join("redberry").join("config.toml")
    }

    /// Default models directory: ~/.local/share/redberry/models/
    pub fn default_models_dir() -> PathBuf {
        dirs_fallback("data").join("redberry").join("models")
    }

    /// Default data directory: ~/.local/share/redberry/
    pub fn default_data_dir() -> PathBuf {
        dirs_fallback("data").join("redberry")
    }

    /// Resolve the context DB path, expanding ~ to home.
    pub fn resolved_db_path(&self) -> PathBuf {
        expand_tilde(&self.context_db_path)
    }
}

/// Expand ~ to the user's home directory.
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(stripped);
        }
    }
    PathBuf::from(path)
}

/// Get a standard directory (config or data) with fallback.
fn dirs_fallback(kind: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    match kind {
        "config" => PathBuf::from(&home).join(".config"),
        "data" => PathBuf::from(&home).join(".local").join("share"),
        _ => PathBuf::from(&home),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RedberryConfig::default();
        assert_eq!(config.sass_level, 3);
        assert_eq!(config.similarity_threshold, 0.3);
        assert_eq!(config.vagueness_threshold, 0.6);
        assert_eq!(config.session_ttl_hours, 24);
        assert_eq!(config.model.preset, "tier1");
    }

    #[test]
    fn test_validation_sass_level() {
        let mut config = RedberryConfig {
            sass_level: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        config.sass_level = 6;
        assert!(config.validate().is_err());

        let config = RedberryConfig {
            sass_level: 5,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_thresholds() {
        let mut config = RedberryConfig {
            similarity_threshold: -0.1,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        config.similarity_threshold = 1.1;
        assert!(config.validate().is_err());

        let config = RedberryConfig {
            similarity_threshold: 0.5,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_parse_model_preset() {
        assert_eq!(ModelPreset::parse_str("tier1"), Some(ModelPreset::Tier1));
        assert_eq!(ModelPreset::parse_str("tier2"), Some(ModelPreset::Tier2));
        // Case insensitive
        assert_eq!(ModelPreset::parse_str("TIER1"), Some(ModelPreset::Tier1));
        // Invalid
        assert_eq!(ModelPreset::parse_str("unknown"), None);
    }

    #[test]
    fn test_resolve_model_preset() {
        let config = RedberryConfig::default();
        let resolved = config.resolve_model().unwrap();
        assert_eq!(resolved.name, "Tier 1 - Standard (bge-small-en-v1.5 INT8)");
        assert_eq!(resolved.embedding_dim, 384);
    }

    #[test]
    fn test_resolve_model_custom() {
        let mut config = RedberryConfig::default();
        config.model.onnx_path = Some("/tmp/custom.onnx".to_string());
        config.model.tokenizer_path = Some("/tmp/tokenizer.json".to_string());
        config.model.embedding_dim = Some(512);

        let resolved = config.resolve_model().unwrap();
        assert_eq!(resolved.name, "custom");
        assert_eq!(resolved.embedding_dim, 512);
    }

    #[test]
    fn test_expand_tilde() {
        let expanded = expand_tilde("~/test/path");
        assert!(!expanded.to_str().unwrap().starts_with("~/"));

        let no_tilde = expand_tilde("/absolute/path");
        assert_eq!(no_tilde, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn test_config_roundtrip() {
        let config = RedberryConfig::default();
        let config_file = ConfigFile {
            redberry: config.clone(),
        };
        let toml_str = toml::to_string_pretty(&config_file).unwrap();
        let parsed: ConfigFile = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.redberry.sass_level, config.sass_level);
        assert_eq!(
            parsed.redberry.similarity_threshold,
            config.similarity_threshold
        );
    }
}
