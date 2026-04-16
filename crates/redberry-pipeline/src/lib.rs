use redberry_analyze::analyze_prompt;
use redberry_core::{ContextMessage, RedberryError, RedberryVerdict};
use redberry_embed::{EmbeddingEngine, ContextCache};
use redberry_persona::PersonalityEngine;
use tracing::info;

/// Evaluates a user prompt for semantic drift, vagueness, and syntactic quality,
/// caches it to the local SQLite DB, and returns the unified response.
pub fn evaluate_pipeline(
    prompt: &str,
    session_id: &str,
    engine: &EmbeddingEngine,
    cache: &mut ContextCache,
    persona: &PersonalityEngine,
) -> Result<RedberryVerdict, RedberryError> {
    let mut analysis = analyze_prompt(prompt);
    let mut current_fatigue = 0;

    let embedding = engine.embed_text(prompt)
        .map_err(|e| RedberryError::Embedding(e.to_string()))?;

    if let Ok(Some(ctx)) = cache.get_context(session_id) {
        current_fatigue = ctx.consecutive_bad;
        analysis.consecutive_bad = current_fatigue;

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

            let similarity = redberry_embed::similarity::cosine_similarity(&embedding, &centroid);
            
            // Map similarity [-1.0, 1.0] to coherence [0.0, 1.0]
            let coherence = ((similarity + 1.0) / 2.0).clamp(0.0, 1.0);
            let drift = 1.0 - coherence;

            analysis.coherence_score = Some(coherence);
            analysis.drift_score = Some(drift);
            
            info!("Calculated Coherence: {}, Drift: {}", coherence, drift);
        }
    }

    let verdict = persona.generate_verdict(&analysis);
    
    let next_fatigue = if verdict.is_approved() {
        0
    } else {
        current_fatigue + 1
    };

    let msg = ContextMessage {
        text: prompt.to_string(),
        embedding: embedding.clone(),
        snark_response: Some(verdict.message().to_string()),
        metrics_vagueness: analysis.vagueness.score,
        metrics_syntax: analysis.syntax.score,
        metrics_drift: analysis.drift_score.unwrap_or(0.0),
        metrics_coherence: analysis.coherence_score.unwrap_or(1.0),
        metrics_specificity: analysis.vagueness.specificity_ratio,
        created_at: None,
    };
    
    // Append the evaluated message into the SQLite persistence map
    let _ = cache.append_messages(session_id, &[msg], next_fatigue);

    Ok(verdict)
}
