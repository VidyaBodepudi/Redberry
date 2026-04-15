//! Pure-Rust embedding engine using tract-onnx and tokenizers.

use redberry_core::config::ResolvedModelConfig;
use redberry_core::RedberryError;
use tokenizers::Tokenizer;
use tract_onnx::prelude::*;

/// The Embedding Engine.
pub struct EmbeddingEngine {
    tokenizer: Tokenizer,
    #[allow(clippy::type_complexity)]
    model: SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>,
    config: ResolvedModelConfig,
}

impl EmbeddingEngine {
    /// Load the model and tokenizer from the resolved configuration.
    pub fn load(config: ResolvedModelConfig) -> Result<Self, RedberryError> {
        tracing::info!("Loading tokenizer from {}", config.tokenizer_path.display());
        let mut tokenizer = Tokenizer::from_file(&config.tokenizer_path)
            .map_err(|e| RedberryError::Embedding(format!("Failed to load tokenizer: {}", e)))?;

        // Disable padding if present, we'll handle token arrays dynamically
        tokenizer.with_padding(None);

        tracing::info!("Loading ONNX model from {}...", config.onnx_path.display());
        let model = tract_onnx::onnx()
            .model_for_path(&config.onnx_path)
            .map_err(|e| RedberryError::Embedding(format!("Failed to load ONNX model: {}", e)))?
            .into_optimized()
            .map_err(|e| RedberryError::Embedding(format!("Failed to optimize model: {}", e)))?
            .into_runnable()
            .map_err(|e| RedberryError::Embedding(format!("Failed to make runnable: {}", e)))?;

        tracing::info!("Model loaded successfully.");

        Ok(Self {
            tokenizer,
            model,
            config,
        })
    }

    /// Generate an embedding for a single string.
    pub fn embed_text(&self, text: &str) -> Result<Vec<f32>, RedberryError> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| RedberryError::Embedding(format!("Failed to tokenize text: {}", e)))?;

        let tokens = encoding.get_ids();
        let len = tokens.len();

        // Convert tokens to i64 (required by most ONNX models)
        let input_ids: Vec<i64> = tokens.iter().map(|&x| x as i64).collect();
        let attention_mask: Vec<i64> = vec![1; len];
        let token_type_ids: Vec<i64> = vec![0; len]; // Often needed for BERT models

        let input_ids_tensor = tract_ndarray::Array2::from_shape_vec((1, len), input_ids)
            .unwrap()
            .into_tensor()
            .into_tvalue();
        let attention_mask_tensor = tract_ndarray::Array2::from_shape_vec((1, len), attention_mask)
            .unwrap()
            .into_tensor()
            .into_tvalue();

        // Check how many inputs the model expects (some need token_type_ids, some don't)
        let num_inputs = self.model.model().inputs.len();
        let mut inputs = tvec![input_ids_tensor, attention_mask_tensor];

        if num_inputs == 3 {
            let token_type_ids_tensor =
                tract_ndarray::Array2::from_shape_vec((1, len), token_type_ids)
                    .unwrap()
                    .into_tensor()
                    .into_tvalue();
            inputs.push(token_type_ids_tensor);
        } else if !(2..=3).contains(&num_inputs) {
            return Err(RedberryError::Embedding(format!(
                "Unsupported number of model inputs: {}. Expected 2 or 3.",
                num_inputs
            )));
        }

        // Run inference
        let result = self
            .model
            .run(inputs)
            .map_err(|e| RedberryError::Embedding(format!("Inference failed: {}", e)))?;

        // Extract output tensor. Usually the first output is last_hidden_state: [batch(1), seq_len, hidden_size]
        let embeddings_tensor = &result[0];
        let view = embeddings_tensor.to_array_view::<f32>().map_err(|e| {
            RedberryError::Embedding(format!("Failed to extract f32 array from output: {}", e))
        })?;

        // Mean pooling over the sequence dimension
        // view.shape() is [1, seq_len, hidden_size]
        let shape = view.shape();
        if shape.len() != 3 {
            return Err(RedberryError::Embedding(format!(
                "Expected output tensor of rank 3, got rank {}",
                shape.len()
            )));
        }

        let seq_len = shape[1];
        let hidden_size = shape[2];
        let mut mean_pooled = vec![0.0f32; hidden_size];
        let out_slice = view.as_slice().unwrap();

        for (h_idx, val) in mean_pooled.iter_mut().take(hidden_size).enumerate() {
            let mut sum = 0.0;
            for t_idx in 0..seq_len {
                let emb = out_slice[t_idx * hidden_size + h_idx];
                sum += emb;
            }
            *val = sum / (seq_len as f32);
        }

        // L2 Normalization
        let norm: f32 = mean_pooled.iter().map(|&x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut mean_pooled {
                *x /= norm;
            }
        }

        // Apply MRL dimensional truncation if configured and supported
        if self.config.embedding_dim < hidden_size {
            mean_pooled.truncate(self.config.embedding_dim);

            // Re-normalize after truncation (standard practice for Matryoshka)
            let new_norm: f32 = mean_pooled.iter().map(|&x| x * x).sum::<f32>().sqrt();
            if new_norm > 0.0 {
                for x in &mut mean_pooled {
                    *x /= new_norm;
                }
            }
        }

        Ok(mean_pooled)
    }

    /// Generate embeddings for a batch of strings.
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, RedberryError> {
        // Simple sequential processing for now.
        // A true batched implementation would require padding all inputs to the same length
        // and passing a batch_size > 1 tensor to tract.
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed_text(text)?);
        }
        Ok(results)
    }
}
