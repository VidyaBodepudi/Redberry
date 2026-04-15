//! Template loading and selection.

use rand::seq::IndexedRandom;
use serde::{Deserialize, Serialize};

/// Extracted content from templates.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    pub approved: ApprovedTemplates,
    pub vagueness: VaguenessTemplates,
    pub syntax: SyntaxTemplates,
    pub drift: DriftTemplates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovedTemplates {
    pub compliments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaguenessTemplates {
    pub low: VaguenessLevelTemplates,
    pub high: VaguenessLevelTemplates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaguenessLevelTemplates {
    pub mockery: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxTemplates {
    pub fragments: SyntaxCategoryTemplates,
    pub run_ons: SyntaxCategoryTemplates,
    pub contradictions: SyntaxCategoryTemplates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxCategoryTemplates {
    pub mockery: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftTemplates {
    pub low: DriftLevelTemplates,
    pub high: DriftLevelTemplates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftLevelTemplates {
    pub snark: Vec<String>,
}

impl TemplateConfig {
    /// Load the default templates compiled into the binary.
    pub fn load_default() -> Self {
        let toml_str = include_str!("../assets/templates.toml");
        toml::from_str(toml_str).expect("Failed to parse embedded default templates")
    }

    /// Helper to pick a random string from a list.
    pub fn pick_random(list: &[String]) -> String {
        if list.is_empty() {
            return "No snark available.".to_string();
        }
        let mut rng = rand::rng();
        list.choose(&mut rng).unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_default() {
        let config = TemplateConfig::load_default();
        assert!(!config.approved.compliments.is_empty());
        assert!(!config.vagueness.high.mockery.is_empty());
        assert!(!config.syntax.contradictions.mockery.is_empty());
        assert!(!config.drift.high.snark.is_empty());
    }

    #[test]
    fn test_pick_random() {
        let list = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let item = TemplateConfig::pick_random(&list);
        assert!(list.contains(&item));
    }
}
