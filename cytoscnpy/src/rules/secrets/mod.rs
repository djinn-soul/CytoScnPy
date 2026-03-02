//! Modular Secret Recognition Engine.

mod compat;
mod entropy;
mod finding;
mod patterns;
mod recognizers;
mod scanner;
mod scoring;
#[cfg(test)]
mod tests;

pub use compat::{scan_secrets, scan_secrets_compat, validate_secrets_config};
pub use entropy::{calculate_entropy, is_high_entropy};
pub use finding::SecretFinding;
pub use patterns::{get_builtin_patterns, BuiltinPattern};
pub use recognizers::{
    AstRecognizer, CustomRecognizer, EntropyRecognizer, RawFinding, RegexRecognizer,
    SecretRecognizer,
};
pub use scanner::SecretScanner;
pub use scoring::{ContextScorer, ScoringAdjustments, ScoringContext};
