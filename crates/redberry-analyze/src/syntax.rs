//! Syntactic quality analysis — detects fragments, run-ons, filler words,
//! and contradictory instructions.

use redberry_core::{SyntaxIssue, SyntaxReport};

/// Check the syntactic quality of a prompt.
/// Returns a SyntaxReport with a score from 0.0 (terrible) to 1.0 (well-structured).
pub fn check_syntax(prompt: &str) -> SyntaxReport {
    let lower = prompt.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();
    let word_count = words.len();

    let mut issues = Vec::new();
    let mut penalties: Vec<f32> = Vec::new();

    // 1. Filler word analysis
    let filler_words_found = detect_filler_words(&words);
    let filler_ratio = if word_count > 0 {
        filler_words_found.len() as f32 / word_count as f32
    } else {
        0.0
    };
    if filler_ratio > 0.15 {
        issues.push(SyntaxIssue::FillerHeavy);
        penalties.push(0.3);
    } else if filler_ratio > 0.1 {
        penalties.push(0.1);
    } else {
        penalties.push(0.0);
    }

    // 2. Fragment detection
    if is_fragment(prompt, word_count) {
        issues.push(SyntaxIssue::Fragment);
        penalties.push(0.3);
    } else {
        penalties.push(0.0);
    }

    // 3. Run-on detection
    if is_run_on(prompt, word_count) {
        issues.push(SyntaxIssue::RunOn);
        penalties.push(0.25);
    } else {
        penalties.push(0.0);
    }

    // 4. Contradiction detection
    if has_contradictions(&lower) {
        issues.push(SyntaxIssue::Contradictory);
        penalties.push(0.4);
    } else {
        penalties.push(0.0);
    }

    // Calculate score: start at 1.0, subtract penalties
    let total_penalty: f32 = penalties.iter().sum();
    let score = (1.0 - total_penalty).clamp(0.0, 1.0);

    SyntaxReport {
        score,
        issues,
        filler_words_found: filler_words_found.iter().map(|s| s.to_string()).collect(),
        filler_ratio,
    }
}

/// Detect filler words that dilute prompt substance.
fn detect_filler_words<'a>(words: &[&'a str]) -> Vec<&'a str> {
    let fillers = [
        "very",
        "really",
        "just",
        "basically",
        "actually",
        "literally",
        "honestly",
        "simply",
        "absolutely",
        "definitely",
        "certainly",
        "obviously",
        "clearly",
        "totally",
        "completely",
        "essentially",
        "truly",
        "quite",
        "rather",
        "pretty",
        "like",
        "um",
        "uh",
        "well",
        "so",
        "anyway",
        "anyways",
    ];

    let mut found = Vec::new();
    for word in words {
        let cleaned = word.trim_matches(|c: char| c.is_ascii_punctuation());
        if fillers.contains(&cleaned) {
            // Don't flag "like" when used as a comparison ("like X")
            // or "so" when used as a conjunction at start
            if cleaned == "like" || cleaned == "so" || cleaned == "well" {
                // Only flag if in the middle of a sentence, not as a connector
                continue;
            }
            found.push(*word);
        }
    }
    found
}

/// Detect sentence fragments — incomplete thoughts.
fn is_fragment(prompt: &str, word_count: usize) -> bool {
    // Very short prompts without verbs are fragments
    if word_count <= 2 {
        return true;
    }

    let lower = prompt.to_lowercase();
    let trimmed = lower.trim();

    // Single-word or two-word prompts are almost always fragments
    if word_count <= 2 && !trimmed.ends_with('?') {
        return true;
    }

    // Check for common verb indicators
    let has_verb_indicator = has_verb(&lower);

    // If the prompt is short and has no verb, it's likely a fragment
    if word_count <= 5 && !has_verb_indicator && !trimmed.ends_with('?') {
        return true;
    }

    false
}

/// Simple heuristic to check if a text likely contains a verb.
fn has_verb(lower: &str) -> bool {
    // Common verb patterns — not exhaustive but catches most imperative prompts
    let common_verbs = [
        "is",
        "are",
        "was",
        "were",
        "be",
        "been",
        "being",
        "have",
        "has",
        "had",
        "do",
        "does",
        "did",
        "will",
        "would",
        "could",
        "should",
        "can",
        "may",
        "might",
        "shall",
        "must",
        "write",
        "create",
        "build",
        "make",
        "fix",
        "add",
        "remove",
        "update",
        "explain",
        "show",
        "tell",
        "help",
        "get",
        "set",
        "run",
        "use",
        "find",
        "give",
        "go",
        "take",
        "know",
        "see",
        "think",
        "want",
        "need",
        "try",
        "keep",
        "let",
        "put",
        "say",
        "turn",
        "call",
        "move",
        "play",
        "work",
        "read",
        "check",
        "test",
        "send",
        "start",
        "stop",
        "open",
        "close",
        "implement",
        "deploy",
        "configure",
        "install",
        "compile",
        "parse",
        "convert",
        "handle",
        "generate",
        "analyze",
        "optimize",
        "refactor",
        "debug",
        "design",
        "list",
        "describe",
    ];

    let words: Vec<&str> = lower.split_whitespace().collect();
    for word in &words {
        let cleaned = word.trim_matches(|c: char| c.is_ascii_punctuation());
        if common_verbs.contains(&cleaned) {
            return true;
        }
        // Check for -ing, -ed, -es endings (verb-like patterns)
        if cleaned.len() > 4
            && (cleaned.ends_with("ing")
                || cleaned.ends_with("ed")
                || cleaned.ends_with("ize")
                || cleaned.ends_with("ate")
                || cleaned.ends_with("ify"))
        {
            return true;
        }
    }
    false
}

/// Detect run-on sentences (long prompts without punctuation breaks).
fn is_run_on(prompt: &str, word_count: usize) -> bool {
    if word_count < 20 {
        return false;
    }

    // Count sentence breaks (periods, semicolons, colons with space after)
    let break_count = prompt
        .chars()
        .zip(prompt.chars().skip(1))
        .filter(|(c, next)| {
            (*c == '.' || *c == ';' || *c == ':' || *c == '!' || *c == '?')
                && (next.is_whitespace() || next.is_uppercase())
        })
        .count();

    // Also count commas as partial breaks
    let comma_count = prompt.matches(',').count();

    // If there are lots of words but very few sentence breaks, it's a run-on
    let total_breaks = break_count + (comma_count / 2); // commas are half-breaks
    let words_per_break = if total_breaks > 0 {
        word_count as f32 / total_breaks as f32
    } else {
        word_count as f32
    };

    // More than 30 words per break is a run-on
    words_per_break > 30.0
}

/// Detect contradictory instructions in a prompt.
fn has_contradictions(lower: &str) -> bool {
    // Common contradiction patterns
    let contradiction_pairs = [
        ("must be fast", "no optimization"),
        ("keep it simple", "comprehensive"),
        ("short", "detailed and exhaustive"),
        ("brief", "cover everything"),
        ("don't use", "make sure to use"),
        ("avoid", "make sure to include"),
        ("without any", "with all"),
        ("no tests", "with full test coverage"),
        ("no comments", "well-documented"),
        ("synchronous", "async"),
    ];

    for (a, b) in &contradiction_pairs {
        if lower.contains(a) && lower.contains(b) {
            return true;
        }
    }

    // Check for "but not" / "but also" contradiction patterns
    if lower.contains("but not") && lower.contains("but also") {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_well_structured_prompt() {
        let report = check_syntax(
            "Write a Rust function that accepts a vector of integers and returns \
             the median value. Handle empty vectors by returning None.",
        );
        assert!(
            report.score > 0.6,
            "Well-structured prompt should score high, got {}",
            report.score
        );
        assert!(report.issues.is_empty());
    }

    #[test]
    fn test_filler_heavy_prompt() {
        let report = check_syntax(
            "basically just really honestly actually truly simply obviously \
             definitely write some code",
        );
        assert!(
            report.filler_ratio > 0.1,
            "Filler-heavy prompt should have high filler ratio, got {}",
            report.filler_ratio
        );
        assert!(report.issues.contains(&SyntaxIssue::FillerHeavy));
    }

    #[test]
    fn test_fragment_detection() {
        let report = check_syntax("rust async");
        assert!(report.issues.contains(&SyntaxIssue::Fragment));
    }

    #[test]
    fn test_not_fragment_imperative() {
        let report = check_syntax("Write a function");
        assert!(
            !report.issues.contains(&SyntaxIssue::Fragment),
            "Imperative sentence should not be a fragment"
        );
    }

    #[test]
    fn test_run_on_detection() {
        let report = check_syntax(
            "write a function that does this and then it should also handle errors \
             and make sure it works with async and also it needs to be compatible \
             with the old API and the new one and don't forget about logging and \
             metrics and also it should be fast and use minimal memory and be \
             thread-safe and work on all platforms",
        );
        assert!(
            report.issues.contains(&SyntaxIssue::RunOn),
            "Long unpunctuated prompt should be flagged as run-on"
        );
    }

    #[test]
    fn test_contradiction_detection() {
        let report =
            check_syntax("Write a synchronous function but make it async for better performance");
        assert!(report.issues.contains(&SyntaxIssue::Contradictory));
    }

    #[test]
    fn test_no_contradictions() {
        let report =
            check_syntax("Write an async function in Rust using tokio with error handling");
        assert!(!report.issues.contains(&SyntaxIssue::Contradictory));
    }

    #[test]
    fn test_has_verb_imperative() {
        assert!(has_verb("write a program"));
        assert!(has_verb("implementing the solution"));
    }

    #[test]
    fn test_has_verb_negative() {
        assert!(!has_verb("rust async"));
        assert!(!has_verb("hello world"));
    }
}
