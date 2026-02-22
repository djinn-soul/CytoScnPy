use serde::Deserialize;
use std::fs;
use std::path::Path;

use rustc_hash::FxHashMap;

use crate::constants::{CONFIG_FILENAME, PYPROJECT_FILENAME};

#[derive(Debug, Deserialize, Default, Clone)]
/// Top-level configuration struct.
pub struct Config {
    #[serde(default)]
    /// The main configuration section for CytoScnPy.
    pub cytoscnpy: CytoScnPyConfig,
    /// The path to the configuration file this was loaded from.
    /// Set during `load_from_path`, `None` if using defaults or programmatic config.
    #[serde(skip)]
    pub config_file_path: Option<std::path::PathBuf>,
}

#[derive(Debug, Deserialize, Default, Clone)]
/// Configuration options for CytoScnPy.
pub struct CytoScnPyConfig {
    /// Confidence threshold (0-100).
    pub confidence: Option<u8>,
    /// List of folders to exclude.
    pub exclude_folders: Option<Vec<String>>,
    /// List of folders to include.
    pub include_folders: Option<Vec<String>>,
    /// Whether to include test files.
    pub include_tests: Option<bool>,
    /// Whether to include `IPython` notebooks.
    pub include_ipynb: Option<bool>,
    /// Whether to scan for secrets.
    pub secrets: Option<bool>,
    /// Whether to scan for dangerous code patterns.
    pub danger: Option<bool>,
    /// Whether to scan for code quality issues.
    pub quality: Option<bool>,
    /// Configuration for danger rules and taint analysis.
    #[serde(default)]
    pub danger_config: DangerConfig,
    // New fields for rule configuration
    /// Maximum allowed lines for a function.
    pub max_lines: Option<usize>,
    /// Maximum allowed arguments for a function.
    pub max_args: Option<usize>,
    /// Maximum allowed cyclomatic complexity.
    #[serde(alias = "complexity")]
    pub max_complexity: Option<usize>,
    /// Deprecated: use `max_complexity` instead.
    #[deprecated(since = "1.2.0", note = "use `max_complexity` instead")]
    #[serde(skip_deserializing)]
    pub complexity: Option<usize>,
    /// Maximum allowed indentation depth.
    #[serde(alias = "nesting")]
    pub max_nesting: Option<usize>,
    /// Deprecated: use `max_nesting` instead.
    #[deprecated(since = "1.2.0", note = "use `max_nesting` instead")]
    #[serde(skip_deserializing)]
    pub nesting: Option<usize>,
    /// Minimum allowed Maintainability Index.
    pub min_mi: Option<f64>,
    /// List of rule codes to ignore.
    pub ignore: Option<Vec<String>>,
    /// Per-file ignore overrides (glob -> rule IDs).
    #[serde(alias = "per-file-ignores")]
    pub per_file_ignores: Option<FxHashMap<String, Vec<String>>>,
    /// Fail threshold percentage (0.0-100.0).
    pub fail_threshold: Option<f64>,
    /// Project type tunes export/public-API assumptions for dead-code analysis.
    #[serde(default)]
    pub project_type: Option<ProjectType>,
    /// Track if deprecated keys were used in the configuration.
    #[serde(skip)]
    deprecated_keys_used: bool,
    /// Advanced secrets scanning configuration.
    #[serde(default)]
    pub secrets_config: Box<SecretsConfig>,
    /// Whitelist of symbol names to ignore during dead code detection.
    /// Supports exact names, wildcards (e.g., "test_*"), and regex patterns.
    #[serde(default)]
    pub whitelist: Vec<WhitelistEntry>,
}

impl CytoScnPyConfig {
    /// Returns whether deprecated keys were used in the configuration.
    #[must_use]
    pub fn uses_deprecated_keys(&self) -> bool {
        self.deprecated_keys_used
    }

    /// Sets whether deprecated keys were used (internal use).
    pub(crate) fn set_uses_deprecated_keys(&mut self, value: bool) {
        self.deprecated_keys_used = value;
    }
}

/// Project mode for dead-code export heuristics.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProjectType {
    /// Library-style analysis: treat public symbols as exported API.
    #[default]
    Library,
    /// Application-style analysis: avoid broad public-API export assumptions.
    Application,
}

fn mark_deprecated_keys_for_cytoscnpy_table(config: &mut Config, table: &toml::Value) {
    if table.get("complexity").is_some() || table.get("nesting").is_some() {
        config.cytoscnpy.set_uses_deprecated_keys(true);
    }
}

fn value_at_path<'a>(value: &'a toml::Value, path: &[&str]) -> Option<&'a toml::Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn mark_deprecated_keys_from_content(config: &mut Config, content: &str, path: &[&str]) {
    if let Ok(value) = toml::from_str::<toml::Value>(content) {
        if let Some(cytoscnpy_table) = value_at_path(&value, path) {
            mark_deprecated_keys_for_cytoscnpy_table(config, cytoscnpy_table);
        }
    }
}

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
    4.5 // Increased from 4.0 to reduce false positives on docstrings
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
    50 // Report findings with >= 50% confidence
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
///
/// Note: This struct uses `Option` fields to distinguish between "explicitly disabled" (Some(false))
/// and "not configured" (None).
/// We use this pattern to enforce **secure-by-default** behavior:
/// - `enable_taint`: Defaults to `true` if unused, ensuring security analysis runs unless explicitly disabled.
/// - `severity_threshold`: Defaults to "LOW" to catch all potential issues by default.
/// - `excluded_rules`: Defaults to empty, ensuring no rules are silently skipped.
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

/// A whitelist entry for ignoring false positives in dead code detection.
///
/// Whitelists allow users to mark symbols as "used" even when the static
/// analyzer cannot detect usage. This is useful for:
/// - Dynamically accessed code (e.g., plugin systems, entry points)
/// - Framework-managed code (e.g., Django models, Flask routes)
/// - Public API symbols in libraries
///
/// # Example TOML Configuration
///
/// ```toml
/// [cytoscnpy]
/// whitelist = [
///     { name = "my_plugin_hook" },
///     { name = "test_*", pattern = "wildcard" },
///     { name = "api_.*", pattern = "regex" },
/// ]
/// ```
#[derive(Debug, Deserialize, Clone)]
pub struct WhitelistEntry {
    /// The symbol name or pattern to whitelist.
    pub name: String,

    /// The type of pattern matching to use.
    /// - `exact` (default): Match the name exactly
    /// - `wildcard`: Use glob-style wildcards (e.g., `test_*`)
    /// - `regex`: Use regular expressions
    #[serde(default)]
    pub pattern: Option<WhitelistPattern>,

    /// Optional file path to restrict the whitelist to a specific file.
    /// Supports glob patterns (e.g., `src/api/*.py`).
    #[serde(default)]
    pub file: Option<String>,

    /// Optional category for documentation/organization purposes.
    #[serde(default)]
    pub category: Option<String>,
}

/// Pattern matching type for whitelist entries.
#[derive(Debug, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WhitelistPattern {
    /// Exact string match (default).
    #[default]
    Exact,
    /// Glob-style wildcard matching (e.g., `test_*`, `*_handler`).
    Wildcard,
    /// Regular expression matching.
    Regex,
}

impl WhitelistEntry {
    /// Check if a symbol name matches this whitelist entry.
    ///
    /// # Arguments
    /// * `symbol_name` - The name of the symbol to check.
    /// * `file_path` - Optional file path for file-specific whitelisting.
    ///
    /// # Returns
    /// `true` if the symbol matches this whitelist entry.
    pub fn matches(&self, symbol_name: &str, file_path: Option<&str>) -> bool {
        // Check file restriction first
        if let Some(ref file_pattern) = self.file {
            if let Some(path) = file_path {
                if !Self::matches_file_pattern(file_pattern, path) {
                    return false;
                }
            } else {
                // File pattern specified but no file path provided
                return false;
            }
        }

        // Match based on pattern type
        match self.pattern.unwrap_or_default() {
            WhitelistPattern::Exact => self.name == symbol_name,
            WhitelistPattern::Wildcard => self.matches_wildcard(symbol_name),
            WhitelistPattern::Regex => self.matches_regex(symbol_name),
        }
    }

    fn matches_wildcard(&self, symbol_name: &str) -> bool {
        // Convert glob pattern to regex
        // Simple implementation: only handle * (any characters) and ? (single character)
        let mut regex_pattern = String::new();
        regex_pattern.push('^');
        for ch in self.name.chars() {
            match ch {
                '*' => regex_pattern.push_str(".*"),
                '?' => regex_pattern.push('.'),
                // Escape regex special characters
                '.' | '^' | '$' | '+' | '[' | ']' | '(' | ')' | '{' | '}' | '\\' | '|' => {
                    regex_pattern.push('\\');
                    regex_pattern.push(ch);
                }
                _ => regex_pattern.push(ch),
            }
        }
        regex_pattern.push('$');

        // Use regex crate to match
        match regex::Regex::new(&regex_pattern) {
            Ok(re) => re.is_match(symbol_name),
            Err(_) => false,
        }
    }

    fn matches_regex(&self, symbol_name: &str) -> bool {
        match regex::Regex::new(&self.name) {
            Ok(re) => re.is_match(symbol_name),
            Err(_) => false,
        }
    }

    fn matches_file_pattern(pattern: &str, path: &str) -> bool {
        // Simple glob matching for file paths
        let pattern_lower = pattern.to_lowercase();
        let path_lower = path.to_lowercase();

        // Handle ** for recursive matching
        if pattern_lower.contains("**") {
            let parts: Vec<&str> = pattern_lower.split("**").collect();
            if parts.len() == 2 {
                let prefix = parts[0].trim_end_matches('/');
                let suffix = parts[1].trim_start_matches('/');
                return (prefix.is_empty() || path_lower.starts_with(prefix))
                    && (suffix.is_empty() || path_lower.ends_with(suffix));
            }
        }

        // Handle simple * wildcard
        if pattern_lower.contains('*') {
            let mut regex_pattern = String::new();
            regex_pattern.push('^');
            for ch in pattern_lower.chars() {
                match ch {
                    '*' => regex_pattern.push_str(".*"),
                    '.' | '^' | '$' | '+' | '[' | ']' | '(' | ')' | '{' | '}' | '\\' | '|' => {
                        regex_pattern.push('\\');
                        regex_pattern.push(ch);
                    }
                    _ => regex_pattern.push(ch),
                }
            }
            regex_pattern.push('$');

            match regex::Regex::new(&regex_pattern) {
                Ok(re) => re.is_match(&path_lower),
                Err(_) => false,
            }
        } else {
            // Exact match or prefix match for directories
            path_lower == pattern_lower || path_lower.starts_with(&format!("{pattern_lower}/"))
        }
    }
}

/// Returns built-in default whitelists for common Python modules.
///
/// These whitelists cover symbols that are typically accessed dynamically
/// or through reflection, which static analysis cannot detect.
///
/// Inspired by Vulture's whitelist approach:
/// <https://github.com/jendrikseipp/vulture/tree/main/vulture/whitelists>
#[must_use]
pub fn get_builtin_whitelists() -> Vec<WhitelistEntry> {
    vec![
        // argparse - argument parser attributes
        WhitelistEntry {
            name: "add_argument".into(),
            category: Some("argparse".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "parse_args".into(),
            category: Some("argparse".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "parse_known_args".into(),
            category: Some("argparse".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "set_defaults".into(),
            category: Some("argparse".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "get_default".into(),
            category: Some("argparse".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "add_subparsers".into(),
            category: Some("argparse".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "add_parser".into(),
            category: Some("argparse".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "set_defaults".into(),
            category: Some("argparse".into()),
            ..Default::default()
        },
        // logging - logger methods and attributes
        WhitelistEntry {
            name: "getLogger".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "debug".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "info".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "warning".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "error".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "critical".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "exception".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "log".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "addHandler".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "removeHandler".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "addFilter".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "removeFilter".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "propagate".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "setLevel".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "getEffectiveLevel".into(),
            category: Some("logging".into()),
            ..Default::default()
        },
        // threading - thread attributes
        WhitelistEntry {
            name: "is_alive".into(),
            category: Some("threading".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "getName".into(),
            category: Some("threading".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "setName".into(),
            category: Some("threading".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "isDaemon".into(),
            category: Some("threading".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "setDaemon".into(),
            category: Some("threading".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "start".into(),
            category: Some("threading".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "join".into(),
            category: Some("threading".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "run".into(),
            category: Some("threading".into()),
            ..Default::default()
        },
        // enum - enum attributes
        WhitelistEntry {
            name: "name".into(),
            category: Some("enum".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "value".into(),
            category: Some("enum".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "_value_".into(),
            category: Some("enum".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "_name_".into(),
            category: Some("enum".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "_missing_".into(),
            category: Some("enum".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "_generate_next_value_".into(),
            category: Some("enum".into()),
            ..Default::default()
        },
        // ctypes - foreign function interface
        WhitelistEntry {
            name: "restype".into(),
            category: Some("ctypes".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "argtypes".into(),
            category: Some("ctypes".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "errcheck".into(),
            category: Some("ctypes".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "value".into(),
            category: Some("ctypes".into()),
            ..Default::default()
        },
        // socketserver - server attributes
        WhitelistEntry {
            name: "allow_reuse_address".into(),
            category: Some("socketserver".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "address_family".into(),
            category: Some("socketserver".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "socket_type".into(),
            category: Some("socketserver".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "request_queue_size".into(),
            category: Some("socketserver".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "timeout".into(),
            category: Some("socketserver".into()),
            ..Default::default()
        },
        // ssl - SSL context attributes
        WhitelistEntry {
            name: "check_hostname".into(),
            category: Some("ssl".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "verify_mode".into(),
            category: Some("ssl".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "protocol".into(),
            category: Some("ssl".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "options".into(),
            category: Some("ssl".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "load_cert_chain".into(),
            category: Some("ssl".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "load_verify_locations".into(),
            category: Some("ssl".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "set_ciphers".into(),
            category: Some("ssl".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "wrap_socket".into(),
            category: Some("ssl".into()),
            ..Default::default()
        },
        // string - formatter attributes
        WhitelistEntry {
            name: "parse".into(),
            category: Some("string".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "format_field".into(),
            category: Some("string".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "get_field".into(),
            category: Some("string".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "get_value".into(),
            category: Some("string".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "convert_field".into(),
            category: Some("string".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "format".into(),
            category: Some("string".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "vformat".into(),
            category: Some("string".into()),
            ..Default::default()
        },
        // sys - system attributes
        WhitelistEntry {
            name: "excepthook".into(),
            category: Some("sys".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "displayhook".into(),
            category: Some("sys".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "exitfunc".into(),
            category: Some("sys".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "stdin".into(),
            category: Some("sys".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "stdout".into(),
            category: Some("sys".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "stderr".into(),
            category: Some("sys".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "path".into(),
            category: Some("sys".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "modules".into(),
            category: Some("sys".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "meta_path".into(),
            category: Some("sys".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "path_hooks".into(),
            category: Some("sys".into()),
            ..Default::default()
        },
        // unittest - test methods
        WhitelistEntry {
            name: "setUp".into(),
            category: Some("unittest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "tearDown".into(),
            category: Some("unittest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "setUpClass".into(),
            category: Some("unittest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "tearDownClass".into(),
            category: Some("unittest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "setUpModule".into(),
            category: Some("unittest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "tearDownModule".into(),
            category: Some("unittest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "run".into(),
            category: Some("unittest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "debug".into(),
            category: Some("unittest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "countTestCases".into(),
            category: Some("unittest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "defaultTestResult".into(),
            category: Some("unittest".into()),
            ..Default::default()
        },
        // collections - special methods
        WhitelistEntry {
            name: "__missing__".into(),
            category: Some("collections".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "_asdict".into(),
            category: Some("collections".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "_make".into(),
            category: Some("collections".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "_replace".into(),
            category: Some("collections".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "_fields".into(),
            category: Some("collections".into()),
            ..Default::default()
        },
        // ast - AST visitor methods
        WhitelistEntry {
            name: "visit".into(),
            category: Some("ast".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "generic_visit".into(),
            category: Some("ast".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "visit_*".into(),
            pattern: Some(WhitelistPattern::Wildcard),
            category: Some("ast".into()),
            ..Default::default()
        },
        // pint - physics units
        WhitelistEntry {
            name: "Quantity".into(),
            category: Some("pint".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "UnitRegistry".into(),
            category: Some("pint".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "Measurement".into(),
            category: Some("pint".into()),
            ..Default::default()
        },
        // Django-style patterns (common in web frameworks)
        WhitelistEntry {
            name: "Meta".into(),
            category: Some("django".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "Objects".into(),
            category: Some("django".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "DoesNotExist".into(),
            category: Some("django".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "MultipleObjectsReturned".into(),
            category: Some("django".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "save".into(),
            category: Some("django".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "delete".into(),
            category: Some("django".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "clean".into(),
            category: Some("django".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "validate_unique".into(),
            category: Some("django".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "get_absolute_url".into(),
            category: Some("django".into()),
            ..Default::default()
        },
        // Flask-style patterns
        WhitelistEntry {
            name: "before_request".into(),
            category: Some("flask".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "after_request".into(),
            category: Some("flask".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "teardown_request".into(),
            category: Some("flask".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "errorhandler".into(),
            category: Some("flask".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "context_processor".into(),
            category: Some("flask".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "url_value_preprocessor".into(),
            category: Some("flask".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "url_defaults".into(),
            category: Some("flask".into()),
            ..Default::default()
        },
        // Pytest fixtures and hooks (already covered by framework detection, but explicit here)
        WhitelistEntry {
            name: "pytest_configure".into(),
            category: Some("pytest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "pytest_unconfigure".into(),
            category: Some("pytest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "pytest_collection_modifyitems".into(),
            category: Some("pytest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "pytest_addoption".into(),
            category: Some("pytest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "pytest_generate_tests".into(),
            category: Some("pytest".into()),
            ..Default::default()
        },
        // Entry points and plugin patterns
        WhitelistEntry {
            name: "main".into(),
            category: Some("entry_point".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "setup".into(),
            category: Some("entry_point".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "teardown".into(),
            category: Some("entry_point".into()),
            ..Default::default()
        },
        // Magic methods that are called dynamically
        WhitelistEntry {
            name: "__call__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__getattr__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__setattr__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__delattr__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__getattribute__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__dir__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__len__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__iter__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__next__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__contains__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__bool__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__str__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__repr__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__hash__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__eq__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__ne__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__lt__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__le__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__gt__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__ge__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__getitem__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__setitem__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__delitem__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
    ]
}

impl Default for WhitelistEntry {
    fn default() -> Self {
        Self {
            name: String::new(),
            pattern: Some(WhitelistPattern::Exact),
            file: None,
            category: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
struct PyProject {
    tool: ToolConfig,
}

#[derive(Debug, Deserialize, Clone)]
struct ToolConfig {
    cytoscnpy: CytoScnPyConfig,
}

impl Config {
    /// Loads configuration from default locations (.cytoscnpy.toml or pyproject.toml in current dir).
    #[must_use]
    pub fn load() -> Self {
        Self::load_from_path(Path::new("."))
    }

    /// Loads configuration starting from a specific path and traversing up.
    #[must_use]
    pub fn load_from_path(path: &Path) -> Self {
        let mut current = path.to_path_buf();
        if current.is_file() {
            current.pop();
        }

        loop {
            // 1. Try CONFIG_FILENAME
            let cytoscnpy_toml = current.join(CONFIG_FILENAME);
            if cytoscnpy_toml.exists() {
                if let Ok(content) = fs::read_to_string(&cytoscnpy_toml) {
                    if let Ok(mut config) = toml::from_str::<Config>(&content) {
                        config.config_file_path = Some(cytoscnpy_toml);
                        mark_deprecated_keys_from_content(&mut config, &content, &["cytoscnpy"]);
                        return config;
                    }
                }
            }

            // 2. Try PYPROJECT_FILENAME
            let pyproject_toml = current.join(PYPROJECT_FILENAME);
            if pyproject_toml.exists() {
                if let Ok(content) = fs::read_to_string(&pyproject_toml) {
                    if let Ok(pyproject) = toml::from_str::<PyProject>(&content) {
                        let mut config = Config {
                            cytoscnpy: pyproject.tool.cytoscnpy,
                            config_file_path: Some(pyproject_toml),
                        };
                        mark_deprecated_keys_from_content(
                            &mut config,
                            &content,
                            &["tool", "cytoscnpy"],
                        );
                        return config;
                    }
                }
            }

            if !current.pop() {
                break;
            }
        }

        Config::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_deprecation_detection_toml() {
        let content = r"
[cytoscnpy]
complexity = 10
";
        let mut config = toml::from_str::<Config>(content).unwrap();
        mark_deprecated_keys_from_content(&mut config, content, &["cytoscnpy"]);
        assert!(config.cytoscnpy.uses_deprecated_keys());
        assert_eq!(config.cytoscnpy.max_complexity, Some(10));
    }

    #[test]
    fn test_deprecation_detection_pyproject() {
        let content = r"
[tool.cytoscnpy]
nesting = 5
";
        let pyproject = toml::from_str::<PyProject>(content).unwrap();
        let mut config = Config {
            cytoscnpy: pyproject.tool.cytoscnpy,
            config_file_path: None,
        };
        mark_deprecated_keys_from_content(&mut config, content, &["tool", "cytoscnpy"]);
        assert!(config.cytoscnpy.uses_deprecated_keys());
        assert_eq!(config.cytoscnpy.max_nesting, Some(5));
    }

    #[test]
    fn test_load_from_path_no_config() {
        // Create an empty temp directory with no config files
        let dir = TempDir::new().unwrap();
        let config = Config::load_from_path(dir.path());
        // Should return default config
        assert!(config.cytoscnpy.confidence.is_none());
        assert!(config.cytoscnpy.max_complexity.is_none());
    }

    #[test]
    fn test_load_from_path_cytoscnpy_toml() {
        let dir = TempDir::new().unwrap();
        let mut file = std::fs::File::create(dir.path().join(".cytoscnpy.toml")).unwrap();
        writeln!(
            file,
            r"[cytoscnpy]
confidence = 80
max_complexity = 15
"
        )
        .unwrap();

        let config = Config::load_from_path(dir.path());
        assert_eq!(config.cytoscnpy.confidence, Some(80));
        assert_eq!(config.cytoscnpy.max_complexity, Some(15));
    }

    #[test]
    fn test_load_from_path_pyproject_toml() {
        let dir = TempDir::new().unwrap();
        let mut file = std::fs::File::create(dir.path().join("pyproject.toml")).unwrap();
        writeln!(
            file,
            r"[tool.cytoscnpy]
max_lines = 200
max_args = 8
"
        )
        .unwrap();

        let config = Config::load_from_path(dir.path());
        assert_eq!(config.cytoscnpy.max_lines, Some(200));
        assert_eq!(config.cytoscnpy.max_args, Some(8));
    }

    #[test]
    fn test_load_from_path_traverses_up() {
        // Create nested directory structure
        let dir = TempDir::new().unwrap();
        let nested = dir.path().join("src").join("lib");
        std::fs::create_dir_all(&nested).unwrap();

        // Put config in root
        let mut file = std::fs::File::create(dir.path().join(".cytoscnpy.toml")).unwrap();
        writeln!(
            file,
            r"[cytoscnpy]
confidence = 90
"
        )
        .unwrap();

        // Load from nested path - should find config in parent
        let config = Config::load_from_path(&nested);
        assert_eq!(config.cytoscnpy.confidence, Some(90));
    }

    #[test]
    fn test_load_from_file_path() {
        let dir = TempDir::new().unwrap();
        let mut file = std::fs::File::create(dir.path().join(".cytoscnpy.toml")).unwrap();
        writeln!(
            file,
            r"[cytoscnpy]
min_mi = 65.0
"
        )
        .unwrap();

        // Create a file in the directory
        let py_file = dir.path().join("test.py");
        std::fs::write(&py_file, "x = 1").unwrap();

        // Load from file path (not directory)
        let config = Config::load_from_path(&py_file);
        assert_eq!(config.cytoscnpy.min_mi, Some(65.0));
    }
}
