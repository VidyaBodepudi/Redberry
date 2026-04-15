//! Core types shared across all Redberry crates.

use serde::{Deserialize, Serialize};

// =============================================================================
// Prompt Decomposition
// =============================================================================

/// The detected intent class of a prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromptIntent {
    /// A question expecting information ("What is...", "How does...")
    Question,
    /// A directive to produce output ("Write...", "Create...", "Build...")
    Instruction,
    /// A creative/open-ended request ("Imagine...", "Come up with...")
    Creative,
    /// A debugging/troubleshooting request ("Fix...", "Why does X fail...")
    Debug,
    /// A request for explanation ("Explain...", "Walk me through...")
    Explanation,
    /// Intent could not be classified
    Unknown,
}

/// A decomposed view of a user prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptDecomposition {
    /// The raw prompt text.
    pub raw_text: String,

    /// Classified intent of the prompt.
    pub intent: PromptIntent,

    /// Extracted entities (proper nouns, technical terms, code references).
    pub entities: Vec<String>,

    /// Extracted explicit constraints ("in Python", "under 100 lines").
    pub constraints: Vec<String>,

    /// References to prior conversation context ("as mentioned", "like before").
    pub context_references: Vec<String>,

    /// Word count of the prompt.
    pub word_count: usize,
}

// =============================================================================
// Vagueness Analysis
// =============================================================================

/// A specific vagueness issue detected in a prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VaguenessFlag {
    /// Too many hedge words ("maybe", "kinda", "sort of")
    ExcessiveHedging,
    /// Low ratio of concrete nouns/entities to total words
    LowSpecificity,
    /// Open-ended question with no constraints ("tell me about X")
    OpenEndedQuestion,
    /// Excessive pronouns without clear referents ("it", "this", "that")
    AmbiguousPronouns,
    /// Instruction with no scope boundaries ("write a program")
    MissingConstraints,
    /// Very short prompt that lacks necessary detail
    TooShort,
}

/// Results of vagueness analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaguenessReport {
    /// Overall vagueness score (0.0 = perfectly specific, 1.0 = hopelessly vague).
    pub score: f32,

    /// Individual issues detected.
    pub flags: Vec<VaguenessFlag>,

    /// Detected hedge words.
    pub hedge_words_found: Vec<String>,

    /// Specificity ratio (entities / total_words).
    pub specificity_ratio: f32,

    /// Ambiguous pronouns found without clear referents.
    pub ambiguous_pronouns: Vec<String>,
}

// =============================================================================
// Syntactic Analysis
// =============================================================================

/// A specific syntactic issue detected in a prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyntaxIssue {
    /// Sentence fragment (incomplete sentence)
    Fragment,
    /// Run-on sentence (no punctuation over multiple clauses)
    RunOn,
    /// Excessive filler words diluting the prompt
    FillerHeavy,
    /// Contradictory instructions detected
    Contradictory,
}

/// Results of syntactic quality analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxReport {
    /// Overall syntax quality score (0.0 = terrible, 1.0 = well-structured).
    pub score: f32,

    /// Individual issues detected.
    pub issues: Vec<SyntaxIssue>,

    /// Filler words detected.
    pub filler_words_found: Vec<String>,

    /// Filler-to-substance ratio.
    pub filler_ratio: f32,
}

// =============================================================================
// Full Prompt Analysis
// =============================================================================

/// Complete analysis results for a user prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptAnalysis {
    /// Structural decomposition of the prompt.
    pub decomposition: PromptDecomposition,

    /// Vagueness analysis results.
    pub vagueness: VaguenessReport,

    /// Syntactic quality results.
    pub syntax: SyntaxReport,

    /// Context drift score (0.0 = completely off-topic, 1.0 = perfectly on-topic).
    /// `None` if no context was provided for comparison.
    pub drift_score: Option<f32>,

    /// Semantic coherence score (similarity to context centroid).
    /// `None` if no context was provided.
    pub coherence_score: Option<f32>,

    /// How many bad prompts the user has sent consecutively in this session.
    pub consecutive_bad: u32,
}

// =============================================================================
// Redberry Verdict
// =============================================================================

/// The final verdict Redberry delivers to the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RedberryVerdict {
    /// The prompt passes muster — but don't let it go to your head.
    Approved {
        /// A backhanded compliment acknowledging the prompt is acceptable.
        backhanded_compliment: String,
    },

    /// The prompt has issues that need fixing.
    NeedsWork {
        /// The snarky critique.
        roast: String,
        /// Specific, actionable suggestions for improvement.
        suggestions: Vec<String>,
    },

    /// The prompt drifted away from the conversation context.
    ContextDrift {
        /// Snarky commentary on the topic change.
        snark: String,
        /// How far the prompt drifted (0.0 = completely different topic).
        drift_score: f32,
        /// What the conversation was previously about.
        prev_topic: String,
        /// What the new prompt seems to be about.
        new_topic: String,
    },

    /// The prompt is too vague to produce a quality response.
    TooVague {
        /// Mocking commentary on the vagueness.
        mockery: String,
        /// What specific elements are missing.
        missing_elements: Vec<String>,
    },

    /// The user is repeatedly failing to write a good prompt.
    Fatigue {
        /// The extreme mockery directed at the user.
        roast: String,
        /// Number of consecutive bad prompts.
        consecutive_bad: u32,
    },
}

impl RedberryVerdict {
    /// Whether the prompt was approved (even if backhanded).
    pub fn is_approved(&self) -> bool {
        matches!(self, Self::Approved { .. })
    }

    /// Get the primary message text from any verdict variant.
    pub fn message(&self) -> &str {
        match self {
            Self::Approved {
                backhanded_compliment,
            } => backhanded_compliment,
            Self::NeedsWork { roast, .. } => roast,
            Self::ContextDrift { snark, .. } => snark,
            Self::TooVague { mockery, .. } => mockery,
            Self::Fatigue { roast, .. } => roast,
        }
    }
}

// =============================================================================
// Session Context
// =============================================================================

/// A stored context message with its embedding.
#[derive(Debug, Clone)]
pub struct ContextMessage {
    /// The message text.
    pub text: String,
    /// Pre-computed embedding vector.
    pub embedding: Vec<f32>,
    /// Sarcastic verdict generated by Redberry (Null if no verdict applied).
    pub snark_response: Option<String>,
    /// Raw vagueness penalty heuristic (0.0 - 1.0).
    pub metrics_vagueness: f32,
    /// Raw syntax fault penalty (0.0 - 1.0).
    pub metrics_syntax: f32,
    /// Semantic drift penalty (0.0 - 1.0) where 1.0 is totally disjointed drift.
    pub metrics_drift: f32,
    /// Unix Epoch timestamp of prompt execution
    pub created_at: Option<i64>,
}

/// A session's conversation context.
#[derive(Debug, Clone)]
pub struct SessionContext {
    /// Session identifier.
    pub session_id: String,
    /// Ordered list of context messages.
    pub messages: Vec<ContextMessage>,
    /// Number of consecutive bad prompts.
    pub consecutive_bad: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verdict_is_approved() {
        let approved = RedberryVerdict::Approved {
            backhanded_compliment: "Not bad.".to_string(),
        };
        assert!(approved.is_approved());

        let needs_work = RedberryVerdict::NeedsWork {
            roast: "Try harder.".to_string(),
            suggestions: vec![],
        };
        assert!(!needs_work.is_approved());
    }

    #[test]
    fn test_verdict_message() {
        let v = RedberryVerdict::TooVague {
            mockery: "What even is this?".to_string(),
            missing_elements: vec!["specifics".to_string()],
        };
        assert_eq!(v.message(), "What even is this?");
    }

    #[test]
    fn test_verdict_serialization() {
        let v = RedberryVerdict::ContextDrift {
            snark: "Nice pivot.".to_string(),
            drift_score: 0.15,
            prev_topic: "Rust".to_string(),
            new_topic: "cooking".to_string(),
        };
        let json = serde_json::to_string(&v).unwrap();
        assert!(json.contains("ContextDrift"));
        assert!(json.contains("Nice pivot."));
    }
}
