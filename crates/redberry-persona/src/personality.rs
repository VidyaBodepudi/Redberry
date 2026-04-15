//! Personality engine — verdict generation from analysis results.

use crate::calibration::SassTone;
use crate::templates::TemplateConfig;
use redberry_core::{PromptAnalysis, RedberryConfig, RedberryVerdict, SyntaxIssue, VaguenessFlag};

/// The personality engine that converts analysis into a snarky verdict.
pub struct PersonalityEngine {
    config: RedberryConfig,
    templates: TemplateConfig,
    tone: SassTone,
}

impl PersonalityEngine {
    /// Create a new personality engine based on the given configuration.
    pub fn new(config: RedberryConfig) -> Self {
        let templates = TemplateConfig::load_default();
        let tone = SassTone::from_level(config.sass_level);
        Self {
            config,
            templates,
            tone,
        }
    }

    /// Run the engine over the prompt analysis to yield a final verdict.
    pub fn generate_verdict(&self, analysis: &PromptAnalysis) -> RedberryVerdict {
        // Priority 0: Severe User Fatigue
        if analysis.consecutive_bad >= 3 {
            let msg = self.format_with_tone(
                &TemplateConfig::pick_random(&self.templates.fatigue.level_3),
                analysis,
            );
            return RedberryVerdict::Fatigue {
                roast: msg,
                consecutive_bad: analysis.consecutive_bad,
            };
        }

        // Priority 1: Context Drift (if available and severe)
        if let Some(drift) = analysis.drift_score {
            if drift < self.config.similarity_threshold {
                let is_high = drift < (self.config.similarity_threshold / 2.0);
                let template_list = if is_high {
                    &self.templates.drift.high.snark
                } else {
                    &self.templates.drift.low.snark
                };

                let msg =
                    self.format_with_tone(&TemplateConfig::pick_random(template_list), analysis);
                return RedberryVerdict::ContextDrift {
                    snark: msg,
                    drift_score: drift,
                    prev_topic: "prior discussion".to_string(), // Need LLM summarization for better topic extraction
                    new_topic: "whatever this is".to_string(),
                };
            }
        }

        // Priority 2: Vagueness
        if analysis.vagueness.score > self.config.vagueness_threshold {
            let is_high = analysis.vagueness.score > 0.8;
            let template_list = if is_high {
                &self.templates.vagueness.high.mockery
            } else {
                &self.templates.vagueness.low.mockery
            };

            let mut missing = Vec::new();
            if analysis.vagueness.flags.contains(&VaguenessFlag::TooShort) {
                missing.push("More words. Effort.".to_string());
            }
            if analysis
                .vagueness
                .flags
                .contains(&VaguenessFlag::MissingConstraints)
            {
                missing.push(
                    "Specific constraints (e.g., framework, length, exact behavior)".to_string(),
                );
            }
            if analysis
                .vagueness
                .flags
                .contains(&VaguenessFlag::LowSpecificity)
            {
                missing.push("Specific entities instead of generic nouns".to_string());
            }

            let msg = self.format_with_tone(&TemplateConfig::pick_random(template_list), analysis);
            return RedberryVerdict::TooVague {
                mockery: msg,
                missing_elements: missing,
            };
        }

        // Priority 3: Syntactic Issues
        if !analysis.syntax.issues.is_empty() {
            let template_list = if analysis.syntax.issues.contains(&SyntaxIssue::Contradictory) {
                &self.templates.syntax.contradictions.mockery
            } else if analysis.syntax.issues.contains(&SyntaxIssue::RunOn) {
                &self.templates.syntax.run_ons.mockery
            } else {
                &self.templates.syntax.fragments.mockery
            };

            let mut suggestions = Vec::new();
            for issue in &analysis.syntax.issues {
                match issue {
                    SyntaxIssue::Fragment => {
                        suggestions.push("Write a complete sentence.".to_string())
                    }
                    SyntaxIssue::RunOn => {
                        suggestions.push("Use punctuation. Full stops are free.".to_string())
                    }
                    SyntaxIssue::FillerHeavy => {
                        suggestions.push("Remove filler words that dilute the point.".to_string())
                    }
                    SyntaxIssue::Contradictory => {
                        suggestions.push("Resolve your contradictory requirements.".to_string())
                    }
                }
            }

            let msg = self.format_with_tone(&TemplateConfig::pick_random(template_list), analysis);
            return RedberryVerdict::NeedsWork {
                roast: msg,
                suggestions,
            };
        }

        // If it passes all tests
        let msg = self.format_with_tone(
            &TemplateConfig::pick_random(&self.templates.approved.compliments),
            analysis,
        );
        RedberryVerdict::Approved {
            backhanded_compliment: msg,
        }
    }

    fn format_with_tone(&self, msg: &str, analysis: &PromptAnalysis) -> String {
        let mut formatted = msg.to_string();

        // Entity injection
        if formatted.contains("{{entity}}") {
            let entity_text = if !analysis.decomposition.entities.is_empty() {
                analysis.decomposition.entities[0].to_string()
            } else {
                "‘whatever you are trying to build’".to_string()
            };
            formatted = formatted.replace("{{entity}}", &entity_text);
        }

        self.tone.format_message(&formatted)
    }
}
