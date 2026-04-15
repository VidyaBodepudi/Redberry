//! # Redberry Core
//!
//! Shared types, configuration, and error handling for the Redberry
//! contrarian conversationalist engine.

pub mod config;
pub mod error;
pub mod types;

pub use config::RedberryConfig;
pub use error::RedberryError;
pub use types::*;
