use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents a secret finding with confidence scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretFinding {
    /// Description of the finding.
    pub message: String,
    /// Unique rule identifier (e.g., "CSP-S101").
    pub rule_id: String,
    /// Category of the rule.
    pub category: String,
    /// File where the secret was found.
    pub file: PathBuf,
    /// Line number (1-indexed).
    pub line: usize,
    /// Severity level (e.g., "HIGH", "CRITICAL").
    pub severity: String,
    /// The matched value (redacted for security).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_value: Option<String>,
    /// Entropy score (if entropy-based detection).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entropy: Option<f64>,
    /// Confidence score (0-100). Higher = more confident it's a real secret.
    pub confidence: u8,
}
