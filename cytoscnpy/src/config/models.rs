use rustc_hash::FxHashMap;
use serde::Deserialize;

use super::security::{DangerConfig, SecretsConfig};
use super::whitelist::WhitelistEntry;

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

#[derive(Debug, Deserialize, Clone)]
pub(super) struct PyProject {
    pub(super) tool: ToolConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct ToolConfig {
    pub(super) cytoscnpy: CytoScnPyConfig,
}
