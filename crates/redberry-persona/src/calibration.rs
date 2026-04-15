//! Sass level calibration.

/// Tone intensity derived from the sass_level configuration (1-5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SassTone {
    /// Level 1: Polite but pointed
    Polite,
    /// Level 2: Passive-aggressive
    PassiveAggressive,
    /// Level 3: Snarky constructive (Default)
    Snarky,
    /// Level 4: Full roast
    Roast,
    /// Level 5: Unhinged
    Unhinged,
}

impl SassTone {
    /// Convert an integer sass level to a tone.
    pub fn from_level(level: u8) -> Self {
        match level {
            1 => Self::Polite,
            2 => Self::PassiveAggressive,
            3 => Self::Snarky,
            4 => Self::Roast,
            _ => Self::Unhinged,
        }
    }

    /// Apply caps/exclamation/formatting modifiers based on the tone.
    pub fn format_message(&self, message: &str) -> String {
        match self {
            Self::Polite => format!("Suggestion: {}", message),
            Self::PassiveAggressive => format!("... {}", message),
            Self::Snarky => message.to_string(), // Base templates are written in snark mode
            Self::Roast => {
                let mut upper = message.to_uppercase();
                if !upper.ends_with('?') && !upper.ends_with('!') {
                    upper.pop(); // Remove period
                    upper.push('!');
                }
                upper
            }
            Self::Unhinged => {
                let mut unhinged = message.to_uppercase();
                // Replace periods with multiple exclamation marks
                unhinged = unhinged.replace('.', "!!1!");
                if !unhinged.ends_with('!') && !unhinged.ends_with('?') {
                    unhinged.push_str("!!!!!");
                }
                unhinged
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_polite() {
        let msg = "This is bad.";
        assert_eq!(
            SassTone::Polite.format_message(msg),
            "Suggestion: This is bad."
        );
    }

    #[test]
    fn test_format_roast() {
        let msg = "This is bad.";
        assert_eq!(SassTone::Roast.format_message(msg), "THIS IS BAD!");

        let msg2 = "Are you serious?";
        assert_eq!(SassTone::Roast.format_message(msg2), "ARE YOU SERIOUS?");
    }

    #[test]
    fn test_format_unhinged() {
        let msg = "This is bad. Very bad.";
        assert_eq!(
            SassTone::Unhinged.format_message(msg),
            "THIS IS BAD!!1! VERY BAD!!1!"
        );
    }
}
