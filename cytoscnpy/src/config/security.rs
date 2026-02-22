use serde::Deserialize;

/// Configuration for advanced secrets scanning (Secret Scanning).
#[derive(Debug, Deserialize, Clone)]
pub struct SecretsConfig {
    /// Minimum Shannon entropy threshold (0.0-8.0) for high-entropy detection.
    /// Higher values = more random. API keys typically have entropy > 4.0.
    #[serde(default = "default_entropy_threshold")]
    pub entropy_threshold: f64,
    /// Minimum string length to check for high entropy.
    #[serde(default = "default_min_length")]
    pub min_length: usize,
    /// Whether to enable entropy-based detection.
    #[serde(default = "default_entropy_enabled")]
    pub entropy_enabled: bool,
    /// Whether to scan comments for secrets (default: true).
    /// Secrets in comments are often accidentally committed credentials.
    #[serde(default = "default_scan_comments")]
    pub scan_comments: bool,
    /// Whether to skip docstrings in entropy scanning (default: true).
    /// Uses AST-based detection to identify actual docstrings.
    #[serde(default = "default_skip_docstrings")]
    pub skip_docstrings: bool,
    /// Custom secret patterns defined by user.
    #[serde(default)]
    pub patterns: Vec<CustomSecretPattern>,
    /// Minimum confidence score to report (0-100).
    /// Findings below this threshold are filtered out.
    #[serde(default = "default_min_score")]
    pub min_score: u8,
    /// Additional suspicious variable names for AST-based detection.
    /// These extend the built-in list (password, secret, key, token, etc.).
    #[serde(default)]
    pub suspicious_names: Vec<String>,
}

fn default_entropy_threshold() -> f64 {
    4.5
}

fn default_min_length() -> usize {
    16
}

fn default_entropy_enabled() -> bool {
    true
}

fn default_scan_comments() -> bool {
    true
}

fn default_skip_docstrings() -> bool {
    false
}

fn default_min_score() -> u8 {
    50
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self {
            entropy_threshold: default_entropy_threshold(),
            min_length: default_min_length(),
            entropy_enabled: default_entropy_enabled(),
            scan_comments: default_scan_comments(),
            skip_docstrings: default_skip_docstrings(),
            patterns: Vec::new(),
            min_score: default_min_score(),
            suspicious_names: Vec::new(),
        }
    }
}

/// Configuration for danger rules and taint analysis.
#[derive(Debug, Deserialize, Default, Clone)]
pub struct DangerConfig {
    /// Whether to enable taint analysis for danger detection.
    pub enable_taint: Option<bool>,
    /// Severity threshold for reporting danger findings.
    pub severity_threshold: Option<String>,
    /// List of rule IDs to exclude from danger scanning.
    pub excluded_rules: Option<Vec<String>>,
    /// Custom taint sources.
    pub custom_sources: Option<Vec<String>>,
    /// Custom taint sinks.
    pub custom_sinks: Option<Vec<String>>,
}

/// A custom secret pattern defined in TOML configuration.
#[derive(Debug, Deserialize, Clone)]
pub struct CustomSecretPattern {
    /// Name/description of the secret type.
    pub name: String,
    /// Regular expression pattern.
    pub regex: String,
    /// Severity level (LOW, MEDIUM, HIGH, CRITICAL).
    #[serde(default = "default_severity")]
    pub severity: String,
    /// Optional rule ID (auto-generated if not provided).
    pub rule_id: Option<String>,
}

fn default_severity() -> String {
    "HIGH".to_owned()
}
