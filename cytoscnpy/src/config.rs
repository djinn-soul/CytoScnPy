use serde::Deserialize;
use std::fs;
use std::path::Path;

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
    /// Fail threshold percentage (0.0-100.0).
    pub fail_threshold: Option<f64>,
    /// Track if deprecated keys were used in the configuration.
    #[serde(skip)]
    _uses_deprecated_keys: bool,
    /// Advanced secrets scanning configuration.
    #[serde(default)]
    pub secrets_config: SecretsConfig,
}

impl CytoScnPyConfig {
    /// Returns whether deprecated keys were used in the configuration.
    #[must_use]
    pub fn uses_deprecated_keys(&self) -> bool {
        self._uses_deprecated_keys
    }

    /// Sets whether deprecated keys were used (internal use).
    pub(crate) fn set_uses_deprecated_keys(&mut self, value: bool) {
        self._uses_deprecated_keys = value;
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
                        config.config_file_path = Some(cytoscnpy_toml.clone());
                        // Check for deprecated keys using Value for robustness
                        if let Ok(value) = toml::from_str::<toml::Value>(&content) {
                            if let Some(cytoscnpy) = value.get("cytoscnpy") {
                                if cytoscnpy.get("complexity").is_some()
                                    || cytoscnpy.get("nesting").is_some()
                                {
                                    config.cytoscnpy.set_uses_deprecated_keys(true);
                                }
                            }
                        }
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
                            config_file_path: Some(pyproject_toml.clone()),
                        };
                        // Check for deprecated keys in the tool section using Value
                        if let Ok(value) = toml::from_str::<toml::Value>(&content) {
                            if let Some(tool) = value.get("tool") {
                                if let Some(cytoscnpy) = tool.get("cytoscnpy") {
                                    if cytoscnpy.get("complexity").is_some()
                                        || cytoscnpy.get("nesting").is_some()
                                    {
                                        config.cytoscnpy.set_uses_deprecated_keys(true);
                                    }
                                }
                            }
                        }
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
        if let Ok(value) = toml::from_str::<toml::Value>(content) {
            if let Some(cytoscnpy) = value.get("cytoscnpy") {
                if cytoscnpy.get("complexity").is_some() || cytoscnpy.get("nesting").is_some() {
                    config.cytoscnpy.set_uses_deprecated_keys(true);
                }
            }
        }
        assert!(config.cytoscnpy.uses_deprecated_keys());
        assert_eq!(config.cytoscnpy.max_complexity, Some(10));
    }

    #[test]
    fn test_deprecation_detection_pyproject() {
        let content = r#"
[tool.cytoscnpy]
nesting = 5
"#;
        let pyproject = toml::from_str::<PyProject>(content).unwrap();
        let mut config = Config {
            cytoscnpy: pyproject.tool.cytoscnpy,
            config_file_path: None,
        };
        if let Ok(value) = toml::from_str::<toml::Value>(content) {
            if let Some(tool) = value.get("tool") {
                if let Some(cytoscnpy) = tool.get("cytoscnpy") {
                    if cytoscnpy.get("complexity").is_some() || cytoscnpy.get("nesting").is_some() {
                        config.cytoscnpy.set_uses_deprecated_keys(true);
                    }
                }
            }
        }
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
