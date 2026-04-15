//! Model setup and downloading.

use redberry_core::config::ModelPreset;
use redberry_core::RedberryError;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Download necessary model files for a given preset if they don't exist.
pub fn ensure_model_files(preset: ModelPreset, models_dir: &Path) -> Result<(), RedberryError> {
    if !models_dir.exists() {
        fs::create_dir_all(models_dir)?;
    }

    let onnx_path = models_dir.join("model.onnx");
    let tokenizer_path = models_dir.join("tokenizer.json");

    let (onnx_url, tokenizer_url) = match preset {
        ModelPreset::Tier1 => (
            "https://huggingface.co/Xenova/bge-small-en-v1.5/resolve/main/onnx/model_quantized.onnx",
            "https://huggingface.co/Xenova/bge-small-en-v1.5/resolve/main/tokenizer.json",
        ),
        ModelPreset::Tier2 => (
            "https://huggingface.co/Xenova/bge-base-en-v1.5/resolve/main/onnx/model_quantized.onnx",
            "https://huggingface.co/Xenova/bge-base-en-v1.5/resolve/main/tokenizer.json",
        ),
    };

    if !tokenizer_path.exists() {
        tracing::info!("Downloading tokenizer from {}...", tokenizer_url);
        download_file(tokenizer_url, &tokenizer_path)?;
    }

    if !onnx_path.exists() {
        tracing::info!(
            "Downloading ONNX model (~{} MB) from {}...",
            preset.approx_size_mb(),
            onnx_url
        );
        download_file(onnx_url, &onnx_path)?;
    }

    Ok(())
}

fn download_file(url: &str, dest: &Path) -> Result<(), RedberryError> {
    let response = reqwest::blocking::get(url)
        .map_err(|e| RedberryError::Model(format!("Failed to download {}: {}", url, e)))?;

    if !response.status().is_success() {
        return Err(RedberryError::Model(format!(
            "HTTP {} when downloading {}",
            response.status(),
            url
        )));
    }

    let bytes = response
        .bytes()
        .map_err(|e| RedberryError::Model(format!("Failed to read response body: {}", e)))?;

    let mut file = fs::File::create(dest)?;
    file.write_all(&bytes)?;

    tracing::info!("Successfully saved {}", dest.display());
    Ok(())
}
