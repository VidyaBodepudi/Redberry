//! Vagueness scoring — detects hedge words, low specificity, ambiguous pronouns,
//! missing constraints, and open-ended questions.

use redberry_core::{PromptDecomposition, PromptIntent, VaguenessFlag, VaguenessReport};

/// Score the vagueness of a prompt. Returns a VaguenessReport with a score
/// from 0.0 (perfectly specific) to 1.0 (hopelessly vague).
pub fn score_vagueness(prompt: &str, decomposition: &PromptDecomposition) -> VaguenessReport {
    let lower = prompt.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();
    let word_count = words.len();

    let mut flags = Vec::new();
    let mut score_components: Vec<f32> = Vec::new();

    // 1. Hedge word density
    let hedge_words_found = detect_hedge_words(&words);
    let hedge_density = if word_count > 0 {
        hedge_words_found.len() as f32 / word_count as f32
    } else {
        0.0
    };
    if hedge_density > 0.1 {
        flags.push(VaguenessFlag::ExcessiveHedging);
        score_components.push(hedge_density.min(1.0));
    } else {
        score_components.push(0.0);
    }

    // 2. Specificity ratio (entities / words)
    let specificity_ratio = if word_count > 0 {
        decomposition.entities.len() as f32 / word_count as f32
    } else {
        0.0
    };
    if specificity_ratio < 0.05 && word_count > 5 {
        flags.push(VaguenessFlag::LowSpecificity);
        score_components.push(0.7); // significant penalty
    } else if specificity_ratio < 0.1 && word_count > 10 {
        score_components.push(0.3); // mild penalty
    } else {
        score_components.push(0.0);
    }

    // 3. Open-ended question detection
    if is_open_ended(&lower, decomposition) {
        flags.push(VaguenessFlag::OpenEndedQuestion);
        score_components.push(0.6);
    } else {
        score_components.push(0.0);
    }

    // 4. Ambiguous pronouns
    let ambiguous_pronouns = detect_ambiguous_pronouns(&words, decomposition);
    if !ambiguous_pronouns.is_empty() {
        let pronoun_ratio = ambiguous_pronouns.len() as f32 / word_count.max(1) as f32;
        if pronoun_ratio > 0.1 {
            flags.push(VaguenessFlag::AmbiguousPronouns);
            score_components.push(0.5);
        } else {
            score_components.push(0.2);
        }
    } else {
        score_components.push(0.0);
    }

    // 5. Missing constraints (instructions without scope)
    if decomposition.intent == PromptIntent::Instruction && decomposition.constraints.is_empty() {
        // Check if the prompt at least has entities to provide some direction
        if decomposition.entities.is_empty() {
            flags.push(VaguenessFlag::MissingConstraints);
            score_components.push(0.7);
        } else if word_count < 10 {
            flags.push(VaguenessFlag::MissingConstraints);
            score_components.push(0.4);
        } else {
            score_components.push(0.1);
        }
    } else {
        score_components.push(0.0);
    }

    // 6. Too short
    if word_count <= 3 {
        flags.push(VaguenessFlag::TooShort);
        score_components.push(0.8);
    } else if word_count <= 6 {
        score_components.push(0.3);
    } else {
        score_components.push(0.0);
    }

    // Calculate final score as a blend of the maximum single penalty and the weighted average
    let weights = [0.15, 0.20, 0.20, 0.15, 0.20, 0.10];
    let weighted_score: f32 = score_components
        .iter()
        .zip(weights.iter())
        .map(|(s, w)| s * w)
        .sum::<f32>()
        / weights.iter().sum::<f32>();

    let max_score = score_components.iter().copied().fold(0.0f32, f32::max);

    // Normalize to 0.0–1.0 (70% max, 30% average)
    let final_score = (max_score * 0.7 + weighted_score * 0.3).clamp(0.0, 1.0);

    VaguenessReport {
        score: final_score,
        flags,
        hedge_words_found: hedge_words_found.iter().map(|s| s.to_string()).collect(),
        specificity_ratio,
        ambiguous_pronouns: ambiguous_pronouns.iter().map(|s| s.to_string()).collect(),
    }
}

/// Detect hedge words and phrases in the word list.
fn detect_hedge_words<'a>(words: &[&'a str]) -> Vec<&'a str> {
    let single_hedges = [
        "maybe",
        "perhaps",
        "possibly",
        "probably",
        "might",
        "somewhat",
        "apparently",
        "supposedly",
        "allegedly",
        "arguably",
    ];

    let mut found = Vec::new();
    let text = words.join(" ");

    // Single-word hedges
    for word in words {
        let cleaned = word.trim_matches(|c: char| c.is_ascii_punctuation());
        if single_hedges.contains(&cleaned) {
            found.push(*word);
        }
    }

    // Multi-word hedge phrases
    let multi_hedges = [
        "sort of",
        "kind of",
        "kinda",
        "i guess",
        "i think",
        "i suppose",
        "not sure",
        "not really sure",
        "more or less",
        "in a way",
        "to some extent",
        "if possible",
        "if you can",
        "or something",
        "or whatever",
        "or anything",
        "i don't know",
        "i dunno",
    ];
    for phrase in &multi_hedges {
        if text.contains(phrase) {
            // Add first word of phrase as marker
            found.push(phrase.split_whitespace().next().unwrap_or(phrase));
        }
    }

    found
}

/// Detect open-ended questions that lack sufficient constraints.
fn is_open_ended(lower: &str, decomposition: &PromptDecomposition) -> bool {
    let open_patterns = [
        "tell me about",
        "what do you think about",
        "what are your thoughts on",
        "what can you tell me about",
        "talk about",
        "discuss",
        "give me some",
        "give me an overview",
        "what is",
    ];

    let starts_open = open_patterns.iter().any(|p| lower.trim().starts_with(p));

    if starts_open {
        // If it has constraints or many entities, it's not truly open-ended
        if !decomposition.constraints.is_empty() || decomposition.entities.len() >= 2 {
            return false;
        }
        return true;
    }

    false
}

/// Detect pronouns that don't have clear referents within the prompt.
fn detect_ambiguous_pronouns<'a>(
    words: &[&'a str],
    decomposition: &PromptDecomposition,
) -> Vec<&'a str> {
    let ambiguous_pronouns_list = ["it", "this", "that", "these", "those", "they", "them"];

    let mut found = Vec::new();

    // If the prompt has entities, pronouns might have referents — be lenient
    let has_entities = !decomposition.entities.is_empty();

    for word in words {
        let cleaned = word.trim_matches(|c: char| c.is_ascii_punctuation());
        if ambiguous_pronouns_list.contains(&cleaned) {
            // If no entities in the prompt, these pronouns are definitely ambiguous
            if !has_entities {
                found.push(*word);
            }
            // If the prompt has context references, pronouns might refer to prior context
            else if decomposition.context_references.is_empty() {
                // Pronoun with entities but no context refs — could go either way
                // Only flag if there are many pronouns relative to entities
                found.push(*word);
            }
        }
    }

    // Only return as "ambiguous" if there are notably more pronouns than entities
    if has_entities && found.len() <= decomposition.entities.len() {
        return Vec::new();
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decompose::decompose;

    #[test]
    fn test_specific_prompt_low_vagueness() {
        let prompt = "Write a Rust function that parses a JSON file into a HashMap<String, Vec<i32>> using serde, with proper error handling";
        let d = decompose(prompt);
        let report = score_vagueness(prompt, &d);
        assert!(
            report.score < 0.4,
            "Specific prompt should have low vagueness, got {}",
            report.score
        );
    }

    #[test]
    fn test_vague_prompt_high_vagueness() {
        let prompt = "maybe do something with that thing from before I guess";
        let d = decompose(prompt);
        let report = score_vagueness(prompt, &d);
        assert!(
            report.score > 0.3,
            "Vague prompt should have high vagueness, got {}",
            report.score
        );
        assert!(report.flags.contains(&VaguenessFlag::ExcessiveHedging));
    }

    #[test]
    fn test_hedge_words_detected() {
        let words: Vec<&str> = "maybe perhaps we could sort of do something"
            .split_whitespace()
            .collect();
        let hedges = detect_hedge_words(&words);
        assert!(hedges.len() >= 2, "Should find multiple hedge words");
    }

    #[test]
    fn test_open_ended_question() {
        let prompt = "tell me about programming";
        let d = decompose(prompt);
        let report = score_vagueness(prompt, &d);
        assert!(report.flags.contains(&VaguenessFlag::OpenEndedQuestion));
    }

    #[test]
    fn test_constrained_question_not_open_ended() {
        let prompt = "tell me about the borrow checker in Rust and how it prevents data races";
        let d = decompose(prompt);
        let report = score_vagueness(prompt, &d);
        assert!(
            !report.flags.contains(&VaguenessFlag::OpenEndedQuestion),
            "Constrained question should not be flagged as open-ended"
        );
    }

    #[test]
    fn test_too_short() {
        let prompt = "help";
        let d = decompose(prompt);
        let report = score_vagueness(prompt, &d);
        assert!(report.flags.contains(&VaguenessFlag::TooShort));
        assert!(report.score > 0.3);
    }

    #[test]
    fn test_missing_constraints_instruction() {
        let prompt = "write a program";
        let d = decompose(prompt);
        let report = score_vagueness(prompt, &d);
        assert!(report.flags.contains(&VaguenessFlag::MissingConstraints));
    }

    #[test]
    fn test_instruction_with_constraints_ok() {
        let prompt = "Write a Python script that downloads images from a URL list, with concurrent downloads using asyncio, under 50 lines";
        let d = decompose(prompt);
        let report = score_vagueness(prompt, &d);
        assert!(
            !report.flags.contains(&VaguenessFlag::MissingConstraints),
            "Instruction with constraints should not be flagged"
        );
    }

    #[test]
    fn test_ambiguous_pronouns() {
        let prompt = "do that thing with it and make it work like that";
        let d = decompose(prompt);
        let report = score_vagueness(prompt, &d);
        assert!(!report.ambiguous_pronouns.is_empty());
    }
}
