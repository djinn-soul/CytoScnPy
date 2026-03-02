use crate::taint::types::{Severity, VulnType};

/// Configuration for taint analysis.
#[derive(Debug, Clone, Default)]
pub struct TaintConfig {
    /// Enable intraprocedural analysis.
    pub intraprocedural: bool,
    /// Enable interprocedural analysis.
    pub interprocedural: bool,
    /// Enable cross-file analysis.
    pub crossfile: bool,
    /// Custom source patterns from config.
    pub custom_sources: Vec<CustomSourceConfig>,
    /// Custom sink patterns from config.
    pub custom_sinks: Vec<CustomSinkConfig>,
}

/// Custom source configuration (from TOML).
#[derive(Debug, Clone)]
pub struct CustomSourceConfig {
    /// Name of the source.
    pub name: String,
    /// Pattern to match (e.g., `mylib.get_input`).
    pub pattern: String,
    /// Severity level.
    pub severity: Severity,
}

/// Custom sink configuration (from TOML).
#[derive(Debug, Clone)]
pub struct CustomSinkConfig {
    /// Name of the sink.
    pub name: String,
    /// Pattern to match (e.g., `mylib.dangerous_func`).
    pub pattern: String,
    /// Vulnerability type.
    pub vuln_type: VulnType,
    /// Severity level.
    pub severity: Severity,
    /// Remediation advice.
    pub remediation: String,
}

impl TaintConfig {
    /// Creates a default config with all analysis levels enabled.
    #[must_use]
    pub fn all_levels() -> Self {
        Self {
            intraprocedural: true,
            interprocedural: true,
            crossfile: true,
            custom_sources: Vec::new(),
            custom_sinks: Vec::new(),
        }
    }

    /// Creates a config with all analysis levels and custom patterns.
    #[must_use]
    pub fn with_custom(sources: Vec<String>, sinks: Vec<String>) -> Self {
        let mut config = Self::all_levels();

        for pattern in sources {
            config.custom_sources.push(CustomSourceConfig {
                name: format!("Custom: {pattern}"),
                pattern,
                severity: Severity::High,
            });
        }

        for pattern in sinks {
            config.custom_sinks.push(CustomSinkConfig {
                name: format!("Custom: {pattern}"),
                pattern,
                vuln_type: VulnType::CodeInjection,
                severity: Severity::High,
                remediation: "Review data flow from custom source to this sink.".to_owned(),
            });
        }

        config
    }

    /// Creates a config with only intraprocedural analysis.
    #[must_use]
    pub fn intraprocedural_only() -> Self {
        Self {
            intraprocedural: true,
            interprocedural: false,
            crossfile: false,
            custom_sources: Vec::new(),
            custom_sinks: Vec::new(),
        }
    }
}
