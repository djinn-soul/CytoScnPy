use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Default, Clone)]
/// Top-level configuration struct.
pub struct Config {
    #[serde(default)]
    /// The main configuration section for CytoScnPy.
    pub cytoscnpy: CytoScnPyConfig,
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
    /// Whether to scan for secrets.
    pub secrets: Option<bool>,
    /// Whether to scan for dangerous code patterns.
    pub danger: Option<bool>,
    /// Whether to scan for code quality issues.
    pub quality: Option<bool>,
    // New fields for rule configuration
    /// Maximum allowed lines for a function.
    pub max_lines: Option<usize>,
    /// Maximum allowed arguments for a function.
    pub max_args: Option<usize>,
    /// Maximum allowed cyclomatic complexity.
    pub complexity: Option<usize>,
    /// Maximum allowed indentation depth.
    pub nesting: Option<usize>,
    /// Minimum allowed Maintainability Index.
    pub min_mi: Option<f64>,
    /// List of rule codes to ignore.
    pub ignore: Option<Vec<String>>,
    /// Fail threshold percentage (0.0-100.0).
    pub fail_threshold: Option<f64>,
    /// Advanced secrets scanning configuration.
    #[serde(default)]
    pub secrets_config: SecretsConfig,
}

/// Configuration for advanced secrets scanning (Secret Scanning 2.0).
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
    /// Custom secret patterns defined by user.
    #[serde(default)]
    pub patterns: Vec<CustomSecretPattern>,
}

fn default_entropy_threshold() -> f64 {
    4.0
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

impl Default for SecretsConfig {
    fn default() -> Self {
        Self {
            entropy_threshold: default_entropy_threshold(),
            min_length: default_min_length(),
            entropy_enabled: default_entropy_enabled(),
            scan_comments: default_scan_comments(),
            patterns: Vec::new(),
        }
    }
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
    pub fn load() -> Self {
        Self::load_from_path(Path::new("."))
    }

    /// Loads configuration starting from a specific path and traversing up.
    pub fn load_from_path(path: &Path) -> Self {
        let mut current = path.to_path_buf();
        if current.is_file() {
            current.pop();
        }

        loop {
            // 1. Try .cytoscnpy.toml
            let cytoscnpy_toml = current.join(".cytoscnpy.toml");
            if cytoscnpy_toml.exists() {
                if let Ok(content) = fs::read_to_string(&cytoscnpy_toml) {
                    if let Ok(config) = toml::from_str::<Config>(&content) {
                        return config;
                    }
                }
            }

            // 2. Try pyproject.toml
            let pyproject_toml = current.join("pyproject.toml");
            if pyproject_toml.exists() {
                if let Ok(content) = fs::read_to_string(&pyproject_toml) {
                    if let Ok(pyproject) = toml::from_str::<PyProject>(&content) {
                        return Config {
                            cytoscnpy: pyproject.tool.cytoscnpy,
                        };
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
