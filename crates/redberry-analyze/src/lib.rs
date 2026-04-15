//! # Redberry Analyze
//!
//! Prompt decomposition and linguistic quality analysis.
//! Pure heuristic NLP — no ML dependencies.

pub mod decompose;
pub mod syntax;
pub mod vagueness;

use redberry_core::PromptAnalysis;

/// Run the full analysis pipeline on a prompt.
///
/// This performs decomposition, vagueness scoring, and syntactic analysis.
/// Drift/coherence scores are set to `None` — those require embeddings
/// from the `redberry-embed` crate.
pub fn analyze_prompt(prompt: &str) -> PromptAnalysis {
    let decomposition = decompose::decompose(prompt);
    let vagueness = vagueness::score_vagueness(prompt, &decomposition);
    let syntax = syntax::check_syntax(prompt);

    PromptAnalysis {
        decomposition,
        vagueness,
        syntax,
        drift_score: None,
        coherence_score: None,
        consecutive_bad: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_pipeline_specific_prompt() {
        let result = analyze_prompt(
            "Write a Python async function that fetches data from the GitHub API \
             using aiohttp, with retry logic and a 30-second timeout.",
        );
        // A well-specified prompt should have low vagueness
        assert!(
            result.vagueness.score < 0.5,
            "Specific prompt should have low vagueness, got {}",
            result.vagueness.score
        );
        assert!(result.vagueness.flags.is_empty());
        assert!(!result.decomposition.entities.is_empty());
        assert!(!result.decomposition.constraints.is_empty());
    }

    #[test]
    fn test_full_pipeline_vague_prompt() {
        let result = analyze_prompt("maybe do something with that thing from before I guess");
        // A vague prompt should have high vagueness
        assert!(
            result.vagueness.score > 0.5,
            "Vague prompt should have high vagueness, got {}",
            result.vagueness.score
        );
        assert!(!result.vagueness.flags.is_empty());
    }

    #[test]
    fn test_drift_and_coherence_are_none() {
        let result = analyze_prompt("test prompt");
        assert!(result.drift_score.is_none());
        assert!(result.coherence_score.is_none());
    }
}
