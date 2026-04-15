//! Error types for Redberry.

use thiserror::Error;

/// Top-level error type for all Redberry operations.
#[derive(Debug, Error)]
pub enum RedberryError {
    /// Configuration loading or validation error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Embedding model loading or inference error.
    #[error("Embedding error: {0}")]
    Embedding(String),

    /// Prompt analysis error.
    #[error("Analysis error: {0}")]
    Analysis(String),

    /// Context cache (SQLite) error.
    #[error("Cache error: {0}")]
    Cache(String),

    /// Model file download or verification error.
    #[error("Model error: {0}")]
    Model(String),

    /// Template loading or rendering error.
    #[error("Template error: {0}")]
    Template(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
