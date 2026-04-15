//! # Redberry Embed
//!
//! Pure-Rust ONNX embedding engine with context caching.
//! Implementation: Phase 2.

pub mod cache;
pub mod engine;
pub mod setup;

pub use cache::ContextCache;
pub use engine::EmbeddingEngine;
pub use setup::ensure_model_files;

/// Cosine similarity and semantic drift detection.
pub mod similarity {
    /// Compute cosine similarity between two embedding vectors.
    ///
    /// Returns a value from -1.0 (opposite) to 1.0 (identical).
    /// Returns 0.0 if either vector has zero magnitude.
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(a.len(), b.len(), "Vectors must have equal length");
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        dot / (norm_a * norm_b)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_identical_vectors() {
            let v = vec![1.0, 2.0, 3.0];
            let sim = cosine_similarity(&v, &v);
            assert!((sim - 1.0).abs() < 1e-6);
        }

        #[test]
        fn test_orthogonal_vectors() {
            let a = vec![1.0, 0.0, 0.0];
            let b = vec![0.0, 1.0, 0.0];
            let sim = cosine_similarity(&a, &b);
            assert!(sim.abs() < 1e-6);
        }

        #[test]
        fn test_opposite_vectors() {
            let a = vec![1.0, 2.0, 3.0];
            let b = vec![-1.0, -2.0, -3.0];
            let sim = cosine_similarity(&a, &b);
            assert!((sim - (-1.0)).abs() < 1e-6);
        }

        #[test]
        fn test_zero_vector() {
            let a = vec![1.0, 2.0, 3.0];
            let b = vec![0.0, 0.0, 0.0];
            assert_eq!(cosine_similarity(&a, &b), 0.0);
        }
    }
}
