//! Prompt decomposition — extracts intent, entities, constraints, and context references.

use redberry_core::{PromptDecomposition, PromptIntent};
use regex::Regex;
use unicode_segmentation::UnicodeSegmentation;

/// Decompose a prompt into its semantic components.
pub fn decompose(prompt: &str) -> PromptDecomposition {
    let word_count = prompt.unicode_words().count();
    let intent = classify_intent(prompt);
    let entities = extract_entities(prompt);
    let constraints = extract_constraints(prompt);
    let context_references = extract_context_references(prompt);

    PromptDecomposition {
        raw_text: prompt.to_string(),
        intent,
        entities,
        constraints,
        context_references,
        word_count,
    }
}

/// Classify the intent of a prompt based on leading verbs and structure.
fn classify_intent(prompt: &str) -> PromptIntent {
    let lower = prompt.to_lowercase();
    let trimmed = lower.trim();

    // Question patterns
    if trimmed.ends_with('?')
        || trimmed.starts_with("what ")
        || trimmed.starts_with("who ")
        || trimmed.starts_with("where ")
        || trimmed.starts_with("when ")
        || trimmed.starts_with("which ")
        || trimmed.starts_with("is ")
        || trimmed.starts_with("are ")
        || trimmed.starts_with("does ")
        || trimmed.starts_with("do ")
        || trimmed.starts_with("can ")
        || trimmed.starts_with("could ")
        || trimmed.starts_with("would ")
        || trimmed.starts_with("should ")
    {
        // Distinguish explanation-questions from pure questions
        if trimmed.starts_with("how ") || trimmed.contains("how does") || trimmed.contains("how do")
        {
            return PromptIntent::Explanation;
        }
        // "Why" questions are often about explanation/debugging
        if trimmed.starts_with("why ") {
            if trimmed.contains("fail")
                || trimmed.contains("error")
                || trimmed.contains("crash")
                || trimmed.contains("break")
                || trimmed.contains("bug")
                || trimmed.contains("wrong")
                || trimmed.contains("not working")
            {
                return PromptIntent::Debug;
            }
            return PromptIntent::Explanation;
        }
        return PromptIntent::Question;
    }

    // Creative patterns
    let creative_verbs = [
        "imagine",
        "brainstorm",
        "come up with",
        "generate ideas",
        "invent",
        "design",
        "propose",
        "suggest",
        "dream up",
    ];
    if creative_verbs.iter().any(|v| trimmed.starts_with(v)) {
        return PromptIntent::Creative;
    }

    // Instruction patterns (broad — catch-all for imperatives)
    let instruction_verbs = [
        "write",
        "create",
        "build",
        "implement",
        "make",
        "add",
        "remove",
        "update",
        "modify",
        "change",
        "convert",
        "transform",
        "refactor",
        "optimize",
        "generate",
        "setup",
        "set up",
        "configure",
        "install",
        "deploy",
        "test",
        "run",
        "execute",
        "compile",
        "list",
        "show",
        "print",
        "output",
        "calculate",
        "compute",
        "parse",
        "format",
        "sort",
        "filter",
        "merge",
        "split",
    ];
    if instruction_verbs.iter().any(|v| trimmed.starts_with(v)) {
        return PromptIntent::Instruction;
    }

    // Debug patterns
    let debug_verbs = [
        "fix",
        "debug",
        "troubleshoot",
        "diagnose",
        "resolve",
        "investigate",
    ];
    let debug_nouns = [
        "error",
        "bug",
        "issue",
        "problem",
        "crash",
        "failure",
        "exception",
        "stack trace",
        "segfault",
        "panic",
    ];
    // Require word boundaries for debug nouns to avoid matching mid-word
    if debug_verbs.iter().any(|v| trimmed.starts_with(v))
        || debug_nouns.iter().any(|n| {
            let padded = format!(" {} ", n);
            let check = format!(" {} ", trimmed.replace(|c: char| !c.is_alphanumeric(), " "));
            check.contains(&padded)
        })
    {
        return PromptIntent::Debug;
    }

    // Explanation patterns
    let explain_verbs = [
        "explain",
        "describe",
        "walk me through",
        "break down",
        "elaborate",
        "clarify",
        "tell me how",
        "tell me why",
        "help me understand",
    ];
    if explain_verbs.iter().any(|v| trimmed.starts_with(v)) || trimmed.starts_with("how ") {
        return PromptIntent::Explanation;
    }

    PromptIntent::Unknown
}

/// Extract entities: proper nouns, technical terms, code references.
fn extract_entities(prompt: &str) -> Vec<String> {
    let mut entities = Vec::new();

    // Backtick-wrapped code references: `something`
    let code_re = Regex::new(r"`([^`]+)`").unwrap();
    for cap in code_re.captures_iter(prompt) {
        entities.push(cap[1].to_string());
    }

    // Capitalized words that aren't sentence starters (heuristic for proper nouns/tech terms)
    let words: Vec<&str> = prompt.split_whitespace().collect();
    for (i, word) in words.iter().enumerate() {
        let cleaned = word.trim_matches(|c: char| c.is_ascii_punctuation());
        if cleaned.is_empty() {
            continue;
        }

        // Skip first word (likely sentence start) unless it looks like a proper noun
        let first_char = cleaned.chars().next().unwrap();
        if i == 0 && first_char.is_uppercase() {
            // Only include if it's a known tech term pattern
            if is_tech_term(cleaned) {
                entities.push(cleaned.to_string());
            }
            continue;
        }

        // Mid-sentence capitalized words are likely entities
        if first_char.is_uppercase() && cleaned.len() > 1 {
            entities.push(cleaned.to_string());
        }

        // ALL-CAPS words (acronyms): API, SQL, HTTP, etc.
        if cleaned.len() >= 2
            && cleaned
                .chars()
                .all(|c| c.is_uppercase() || c.is_ascii_digit())
            && !entities.contains(&cleaned.to_string())
        {
            entities.push(cleaned.to_string());
        }
    }

    // Quoted strings as entities
    let quote_re = Regex::new(r#""([^"]+)""#).unwrap();
    for cap in quote_re.captures_iter(prompt) {
        let quoted = cap[1].to_string();
        if !entities.contains(&quoted) {
            entities.push(quoted);
        }
    }

    entities.sort();
    entities.dedup();
    entities
}

/// Check if a word looks like a known technology/proper noun.
fn is_tech_term(word: &str) -> bool {
    let tech_terms = [
        "Python",
        "Rust",
        "JavaScript",
        "TypeScript",
        "Java",
        "Go",
        "Ruby",
        "Swift",
        "Kotlin",
        "SQL",
        "HTML",
        "CSS",
        "React",
        "Vue",
        "Angular",
        "Node",
        "Docker",
        "Kubernetes",
        "AWS",
        "Azure",
        "GCP",
        "Git",
        "GitHub",
        "Linux",
        "Windows",
        "Mac",
        "API",
        "REST",
        "GraphQL",
        "HTTP",
        "JSON",
        "YAML",
        "TOML",
        "XML",
        "Redis",
        "Postgres",
        "MongoDB",
        "SQLite",
        "Terraform",
        "Ansible",
        "Nginx",
        "Apache",
        "Django",
        "Flask",
        "FastAPI",
        "Express",
        "Tokio",
        "Axum",
        "Actix",
        "Rocket",
        "ONNX",
        "PyTorch",
        "TensorFlow",
        "CUDA",
        "WebSocket",
        "gRPC",
        "OAuth",
        "JWT",
        "MCP",
        "LLM",
        "GPT",
        "Claude",
        "Gemini",
    ];
    tech_terms.contains(&word)
}

/// Extract explicit constraints from a prompt.
fn extract_constraints(prompt: &str) -> Vec<String> {
    let mut constraints = Vec::new();
    let lower = prompt.to_lowercase();

    // Language constraints: "in Python", "using Rust", "with JavaScript"
    let lang_re =
        Regex::new(r"(?i)\b(in|using|with|via)\s+(python|rust|javascript|typescript|java|go|ruby|swift|kotlin|c\+\+|c#|php|perl|scala|haskell|elixir|clojure|dart|r\b)")
            .unwrap();
    for cap in lang_re.captures_iter(prompt) {
        constraints.push(format!("{} {}", &cap[1], &cap[2]));
    }

    // Size/length constraints: "under 100 lines", "less than 50", "at most 200"
    let size_re =
        Regex::new(r"(?i)(under|less than|at most|no more than|within|maximum|max)\s+(\d+)\s*(lines?|words?|characters?|tokens?|bytes?|kb|mb|seconds?|ms|minutes?)?")
            .unwrap();
    for cap in size_re.captures_iter(prompt) {
        constraints.push(cap[0].to_string());
    }

    // Framework/library constraints: "using aiohttp", "with tokio", "using react"
    let framework_re = Regex::new(
        r"(?i)\b(using|with|via)\s+([a-zA-Z][a-zA-Z0-9_-]+(?:\.[a-zA-Z][a-zA-Z0-9_-]+)*)\b",
    )
    .unwrap();
    for cap in framework_re.captures_iter(prompt) {
        let lib_name = &cap[2];
        // Avoid duplicating language constraints or common words
        let skip_words = [
            "the", "a", "an", "this", "that", "it", "no", "my", "your", "and", "or", "but",
            "retry", "error", "timeout",
        ];
        if !skip_words.contains(&lib_name.to_lowercase().as_str())
            && !constraints.iter().any(|c| c.contains(lib_name))
        {
            constraints.push(format!("{} {}", &cap[1], lib_name));
        }
    }

    // Explicit requirement markers
    let requirement_patterns = [
        "must ",
        "should ",
        "needs to ",
        "has to ",
        "required to ",
        "make sure ",
        "ensure ",
    ];
    for pattern in &requirement_patterns {
        if let Some(pos) = lower.find(pattern) {
            // Extract the clause after the marker (up to period, comma, or end)
            let rest = &prompt[pos..];
            let end = rest
                .find(|c: char| ['.', ',', ';'].contains(&c))
                .unwrap_or(rest.len());
            let clause = rest[..end].trim().to_string();
            if clause.split_whitespace().count() <= 15 {
                constraints.push(clause);
            }
        }
    }

    constraints.sort();
    constraints.dedup();
    constraints
}

/// Extract references to prior conversation context.
fn extract_context_references(prompt: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let lower = prompt.to_lowercase();

    let patterns = [
        "as mentioned",
        "as discussed",
        "as we discussed",
        "as i said",
        "as you said",
        "from before",
        "from earlier",
        "from the previous",
        "like before",
        "like earlier",
        "like we discussed",
        "the previous",
        "the earlier",
        "the last",
        "mentioned earlier",
        "mentioned before",
        "discussed earlier",
        "we talked about",
        "you mentioned",
        "i mentioned",
        "that thing",
        "the thing we",
        "the one we",
        "similar to what we",
        "same as before",
        "same as last time",
        "continuing from",
        "building on",
        "going back to",
        "referring to",
        "related to what we",
        "following up on",
        "follow up on",
        "per our discussion",
        "per the earlier",
        "remember when",
        "you know the",
    ];

    for pattern in &patterns {
        if lower.contains(pattern) {
            refs.push(pattern.to_string());
        }
    }

    refs
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Intent Classification Tests ===

    #[test]
    fn test_intent_question() {
        assert_eq!(
            classify_intent("What is the difference between TCP and UDP?"),
            PromptIntent::Question
        );
        assert_eq!(
            classify_intent("Is Rust memory-safe?"),
            PromptIntent::Question
        );
    }

    #[test]
    fn test_intent_instruction() {
        assert_eq!(
            classify_intent("Write a function that sorts a list"),
            PromptIntent::Instruction
        );
        assert_eq!(
            classify_intent("Create a REST API endpoint"),
            PromptIntent::Instruction
        );
        assert_eq!(
            classify_intent("Refactor this code to use async/await"),
            PromptIntent::Instruction
        );
    }

    #[test]
    fn test_intent_debug() {
        assert_eq!(
            classify_intent("Fix the null pointer exception in my code"),
            PromptIntent::Debug
        );
        assert_eq!(
            classify_intent("Why does this function crash with a segfault?"),
            PromptIntent::Debug
        );
        assert_eq!(
            classify_intent("Debug the authentication error"),
            PromptIntent::Debug
        );
    }

    #[test]
    fn test_intent_explanation() {
        assert_eq!(
            classify_intent("Explain how async/await works in Rust"),
            PromptIntent::Explanation
        );
        assert_eq!(
            classify_intent("How does the borrow checker work?"),
            PromptIntent::Explanation
        );
        assert_eq!(
            classify_intent("Walk me through the OAuth flow"),
            PromptIntent::Explanation
        );
    }

    #[test]
    fn test_intent_creative() {
        assert_eq!(
            classify_intent("Brainstorm ideas for a new CLI tool"),
            PromptIntent::Creative
        );
        assert_eq!(
            classify_intent("Imagine a world where all code is self-documenting"),
            PromptIntent::Creative
        );
    }

    // === Entity Extraction Tests ===

    #[test]
    fn test_entities_code_refs() {
        let entities = extract_entities("Use the `tokio::spawn` function with `async move`");
        assert!(entities.contains(&"tokio::spawn".to_string()));
        assert!(entities.contains(&"async move".to_string()));
    }

    #[test]
    fn test_entities_proper_nouns() {
        let entities = extract_entities("Deploy the service to AWS using Docker and Kubernetes");
        assert!(entities.contains(&"AWS".to_string()));
        assert!(entities.contains(&"Docker".to_string()));
        assert!(entities.contains(&"Kubernetes".to_string()));
    }

    #[test]
    fn test_entities_quoted() {
        let entities =
            extract_entities("Create a function called \"processData\" that handles input");
        assert!(entities.contains(&"processData".to_string()));
    }

    // === Constraint Extraction Tests ===

    #[test]
    fn test_constraints_language() {
        let constraints = extract_constraints("Write a web scraper in Python using aiohttp");
        assert!(constraints
            .iter()
            .any(|c| c.to_lowercase().contains("python")));
    }

    #[test]
    fn test_constraints_size() {
        let constraints = extract_constraints("Keep the solution under 100 lines");
        assert!(constraints.iter().any(|c| c.contains("100")));
    }

    #[test]
    fn test_constraints_requirements() {
        let constraints =
            extract_constraints("It must handle errors gracefully and should log all requests");
        assert!(!constraints.is_empty());
    }

    // === Context Reference Tests ===

    #[test]
    fn test_context_refs_found() {
        let refs = extract_context_references(
            "As we discussed earlier, can you update the function from before?",
        );
        assert!(refs.contains(&"as we discussed".to_string()));
        assert!(refs.contains(&"from before".to_string()));
    }

    #[test]
    fn test_context_refs_none() {
        let refs =
            extract_context_references("Write a Python function that calculates fibonacci numbers");
        assert!(refs.is_empty());
    }

    // === Full Decomposition Tests ===

    #[test]
    fn test_decompose_rich_prompt() {
        let d = decompose(
            "Write a Python async function using aiohttp that fetches data from \
             the GitHub API, with retry logic, under 50 lines. As discussed earlier, \
             make sure it handles the `RateLimitError` gracefully.",
        );
        assert_eq!(d.intent, PromptIntent::Instruction);
        assert!(!d.entities.is_empty());
        assert!(!d.constraints.is_empty());
        assert!(!d.context_references.is_empty());
        assert!(d.word_count > 10);
    }

    #[test]
    fn test_decompose_minimal_prompt() {
        let d = decompose("help");
        assert_eq!(d.word_count, 1);
        assert!(d.entities.is_empty());
        assert!(d.constraints.is_empty());
        assert!(d.context_references.is_empty());
    }
}
