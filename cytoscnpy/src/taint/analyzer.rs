//! Main taint analyzer with plugin architecture.
//!
//! Provides a configurable taint analysis engine that supports:
//! - Built-in sources and sinks
//! - Custom plugin sources and sinks
//! - Configuration via TOML

use super::crossfile::CrossFileAnalyzer;
use super::interprocedural;
use super::intraprocedural;
use super::sinks::check_sink as check_builtin_sink;
use super::sources::check_taint_source;
use super::types::{Severity, SinkMatch, TaintFinding, TaintInfo, TaintSource, VulnType};
use crate::utils::LineIndex;
use ruff_python_ast::{Expr, Stmt};
use ruff_text_size::Ranged;
use std::path::PathBuf;
use std::sync::Arc;

// ============================================================================
// Plugin Traits
// ============================================================================

/// Trait for custom taint source plugins.
pub trait TaintSourcePlugin: Send + Sync {
    /// Returns the name of this source plugin.
    fn name(&self) -> &str;

    /// Checks if an expression is a taint source.
    /// Returns Some(TaintInfo) if the expression is a source, None otherwise.
    fn check_source(&self, expr: &Expr, line_index: &LineIndex) -> Option<TaintInfo>;

    /// Returns the source patterns this plugin handles (for documentation).
    fn patterns(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Trait for custom taint sink plugins.
pub trait TaintSinkPlugin: Send + Sync {
    /// Returns the name of this sink plugin.
    fn name(&self) -> &str;

    /// Checks if a call expression is a dangerous sink.
    /// Returns Some(SinkMatch) if the call is a sink, None otherwise.
    fn check_sink(&self, call: &ruff_python_ast::ExprCall) -> Option<SinkMatch>;

    /// Returns the sink patterns this plugin handles.
    fn patterns(&self) -> Vec<String> {
        Vec::new()
    }
}

// SinkMatch moved to types.rs

/// Trait for custom sanitizer plugins.
pub trait SanitizerPlugin: Send + Sync {
    /// Returns the name of this sanitizer plugin.
    fn name(&self) -> &str;

    /// Checks if a call sanitizes taint.
    fn is_sanitizer(&self, call: &ruff_python_ast::ExprCall) -> bool;

    /// Returns which vulnerability types this sanitizer addresses.
    fn sanitizes_vuln_types(&self) -> Vec<VulnType> {
        Vec::new()
    }
}

// ============================================================================
// Plugin Registry
// ============================================================================

/// Registry for taint analysis plugins.
#[derive(Default)]
pub struct PluginRegistry {
    /// Registered source plugins
    pub sources: Vec<Arc<dyn TaintSourcePlugin>>,
    /// Registered sink plugins
    pub sinks: Vec<Arc<dyn TaintSinkPlugin>>,
    /// Registered sanitizer plugins
    pub sanitizers: Vec<Arc<dyn SanitizerPlugin>>,
}

impl PluginRegistry {
    /// Creates a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a source plugin.
    pub fn register_source<T: TaintSourcePlugin + 'static>(&mut self, plugin: T) {
        self.sources.push(Arc::new(plugin));
    }

    /// Registers a sink plugin.
    pub fn register_sink<T: TaintSinkPlugin + 'static>(&mut self, plugin: T) {
        self.sinks.push(Arc::new(plugin));
    }

    /// Registers a sanitizer plugin.
    pub fn register_sanitizer<T: SanitizerPlugin + 'static>(&mut self, plugin: T) {
        self.sanitizers.push(Arc::new(plugin));
    }

    /// Checks all source plugins for a match.
    pub fn check_sources(&self, expr: &Expr, line_index: &LineIndex) -> Option<TaintInfo> {
        for plugin in &self.sources {
            if let Some(info) = plugin.check_source(expr, line_index) {
                return Some(info);
            }
        }
        None
    }

    /// Checks all sink plugins for a match.
    pub fn check_sinks(&self, call: &ruff_python_ast::ExprCall) -> Option<SinkMatch> {
        for plugin in &self.sinks {
            if let Some(sink) = plugin.check_sink(call) {
                return Some(sink);
            }
        }
        None
    }

    /// Checks if any sanitizer plugin matches.
    pub fn is_sanitizer(&self, call: &ruff_python_ast::ExprCall) -> bool {
        for plugin in &self.sanitizers {
            if plugin.is_sanitizer(call) {
                return true;
            }
        }
        false
    }
}

// ============================================================================
// Built-in Plugins
// ============================================================================

/// Built-in Flask source plugin.
pub struct FlaskSourcePlugin;

impl TaintSourcePlugin for FlaskSourcePlugin {
    fn name(&self) -> &'static str {
        "Flask"
    }

    fn check_source(&self, expr: &Expr, line_index: &LineIndex) -> Option<TaintInfo> {
        check_taint_source(expr, line_index)
            .filter(|info| matches!(info.source, TaintSource::FlaskRequest(_)))
    }

    fn patterns(&self) -> Vec<String> {
        vec![
            "request.args".to_owned(),
            "request.form".to_owned(),
            "request.data".to_owned(),
            "request.json".to_owned(),
            "request.cookies".to_owned(),
            "request.files".to_owned(),
        ]
    }
}

/// Built-in Django source plugin.
pub struct DjangoSourcePlugin;

impl TaintSourcePlugin for DjangoSourcePlugin {
    fn name(&self) -> &'static str {
        "Django"
    }

    fn check_source(&self, expr: &Expr, line_index: &LineIndex) -> Option<TaintInfo> {
        check_taint_source(expr, line_index)
            .filter(|info| matches!(info.source, TaintSource::DjangoRequest(_)))
    }

    fn patterns(&self) -> Vec<String> {
        vec![
            "request.GET".to_owned(),
            "request.POST".to_owned(),
            "request.body".to_owned(),
            "request.COOKIES".to_owned(),
        ]
    }
}

/// Built-in input/environment source plugin.
pub struct BuiltinSourcePlugin;

impl TaintSourcePlugin for BuiltinSourcePlugin {
    fn name(&self) -> &'static str {
        "Builtin"
    }

    fn check_source(&self, expr: &Expr, line_index: &LineIndex) -> Option<TaintInfo> {
        check_taint_source(expr, line_index).filter(|info| {
            matches!(
                info.source,
                TaintSource::Input
                    | TaintSource::Environment
                    | TaintSource::CommandLine
                    | TaintSource::FileRead
                    | TaintSource::ExternalData
            )
        })
    }

    fn patterns(&self) -> Vec<String> {
        vec![
            "input()".to_owned(),
            "sys.argv".to_owned(),
            "os.environ".to_owned(),
        ]
    }
}

/// Azure Functions source plugin.
pub struct AzureSourcePlugin;

impl TaintSourcePlugin for AzureSourcePlugin {
    fn name(&self) -> &'static str {
        "AzureFunctions"
    }

    fn check_source(&self, expr: &Expr, line_index: &LineIndex) -> Option<TaintInfo> {
        check_taint_source(expr, line_index)
            .filter(|info| matches!(info.source, TaintSource::AzureFunctionsRequest(_)))
    }

    fn patterns(&self) -> Vec<String> {
        vec![
            "req.params".to_owned(),
            "req.route_params".to_owned(),
            "req.headers".to_owned(),
            "req.form".to_owned(),
            "req.get_json".to_owned(),
            "req.get_body".to_owned(),
        ]
    }
}

/// Built-in sink plugin.
pub struct BuiltinSinkPlugin;

impl TaintSinkPlugin for BuiltinSinkPlugin {
    fn name(&self) -> &'static str {
        "Builtin"
    }

    fn check_sink(&self, call: &ruff_python_ast::ExprCall) -> Option<SinkMatch> {
        check_builtin_sink(call).map(|info| SinkMatch {
            name: info.name,
            vuln_type: info.vuln_type,
            severity: info.severity,
            dangerous_args: info.dangerous_args,
            dangerous_keywords: info.dangerous_keywords,
            remediation: info.remediation,
        })
    }

    fn patterns(&self) -> Vec<String> {
        super::sinks::SINK_PATTERNS
            .iter()
            .map(|s| (*s).to_owned())
            .collect()
    }
}

/// Plugin for dynamic patterns from configuration.
pub struct DynamicPatternPlugin {
    /// List of custom source patterns to match.
    pub sources: Vec<String>,
    /// List of custom sink patterns to match.
    pub sinks: Vec<String>,
}

impl TaintSourcePlugin for DynamicPatternPlugin {
    fn name(&self) -> &'static str {
        "DynamicConfig"
    }

    fn check_source(&self, expr: &Expr, line_index: &LineIndex) -> Option<TaintInfo> {
        use crate::rules::danger::utils::get_call_name;
        let target = if let Expr::Call(call) = expr {
            &call.func
        } else {
            expr
        };
        if let Some(call_name) = get_call_name(target) {
            for pattern in &self.sources {
                if &call_name == pattern {
                    return Some(TaintInfo::new(
                        TaintSource::Custom(pattern.clone()),
                        line_index.line_index(expr.range().start()),
                    ));
                }
            }
        }
        None
    }

    fn patterns(&self) -> Vec<String> {
        self.sources.clone()
    }
}

impl TaintSinkPlugin for DynamicPatternPlugin {
    fn name(&self) -> &'static str {
        "DynamicConfig"
    }

    fn check_sink(&self, call: &ruff_python_ast::ExprCall) -> Option<SinkMatch> {
        use crate::rules::danger::utils::get_call_name;
        if let Some(call_name) = get_call_name(&call.func) {
            for pattern in &self.sinks {
                if &call_name == pattern {
                    return Some(SinkMatch {
                        name: pattern.clone(),
                        vuln_type: VulnType::CodeInjection,
                        severity: Severity::High,
                        dangerous_args: vec![0], // Assume first arg is dangerous by default for custom sinks
                        dangerous_keywords: Vec::new(),
                        remediation: "Review data flow to this custom sink.".to_owned(),
                    });
                }
            }
        }
        None
    }

    fn patterns(&self) -> Vec<String> {
        self.sinks.clone()
    }
}

// ============================================================================
// Main Analyzer
// ============================================================================

/// Configuration for taint analysis.
#[derive(Debug, Clone, Default)]
pub struct TaintConfig {
    /// Enable intraprocedural analysis
    pub intraprocedural: bool,
    /// Enable interprocedural analysis
    pub interprocedural: bool,
    /// Enable cross-file analysis
    pub crossfile: bool,
    /// Custom source patterns from config
    pub custom_sources: Vec<CustomSourceConfig>,
    /// Custom sink patterns from config
    pub custom_sinks: Vec<CustomSinkConfig>,
}

/// Custom source configuration (from TOML).
#[derive(Debug, Clone)]
pub struct CustomSourceConfig {
    /// Name of the source
    pub name: String,
    /// Pattern to match (e.g., "`mylib.get_input`")
    pub pattern: String,
    /// Severity level
    pub severity: Severity,
}

/// Custom sink configuration (from TOML).
#[derive(Debug, Clone)]
pub struct CustomSinkConfig {
    /// Name of the sink
    pub name: String,
    /// Pattern to match (e.g., "`mylib.dangerous_func`")
    pub pattern: String,
    /// Vulnerability type
    pub vuln_type: VulnType,
    /// Severity level
    pub severity: Severity,
    /// Remediation advice
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
                vuln_type: VulnType::CodeInjection, // Default to code injection for custom patterns
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

/// Main taint analyzer.
pub struct TaintAnalyzer {
    /// Plugin registry
    pub plugins: PluginRegistry,
    /// Configuration
    pub config: TaintConfig,
    /// Cross-file analyzer (if enabled)
    crossfile_analyzer: Option<CrossFileAnalyzer>,
}

impl TaintAnalyzer {
    /// Creates a new taint analyzer with default plugins.
    #[must_use]
    pub fn new(config: TaintConfig) -> Self {
        let mut plugins = PluginRegistry::new();

        // Register built-in plugins
        plugins.register_source(FlaskSourcePlugin);
        plugins.register_source(DjangoSourcePlugin);
        plugins.register_source(BuiltinSourcePlugin);
        plugins.register_source(AzureSourcePlugin);
        plugins.register_sink(BuiltinSinkPlugin);

        // Register custom patterns from config
        let custom_sources: Vec<String> = config
            .custom_sources
            .iter()
            .map(|s| s.pattern.clone())
            .collect();
        let custom_sinks: Vec<String> = config
            .custom_sinks
            .iter()
            .map(|s| s.pattern.clone())
            .collect();

        if !custom_sources.is_empty() || !custom_sinks.is_empty() {
            let dynamic = Arc::new(DynamicPatternPlugin {
                sources: custom_sources,
                sinks: custom_sinks,
            });
            plugins
                .sources
                .push(Arc::clone(&dynamic) as Arc<dyn TaintSourcePlugin>);
            plugins.sinks.push(dynamic);
        }

        let crossfile_analyzer = if config.crossfile {
            Some(CrossFileAnalyzer::new())
        } else {
            None
        };

        Self {
            plugins,
            config,
            crossfile_analyzer,
        }
    }

    /// Creates an analyzer with no built-in plugins (for custom setups).
    #[must_use]
    pub fn empty(config: TaintConfig) -> Self {
        Self {
            plugins: PluginRegistry::new(),
            config,
            crossfile_analyzer: None,
        }
    }

    /// Registers a custom source plugin.
    pub fn add_source<T: TaintSourcePlugin + 'static>(&mut self, plugin: T) {
        self.plugins.register_source(plugin);
    }

    /// Registers a custom sink plugin.
    pub fn add_sink<T: TaintSinkPlugin + 'static>(&mut self, plugin: T) {
        self.plugins.register_sink(plugin);
    }

    /// Analyzes a single file.
    #[must_use]
    pub fn analyze_file(&self, source: &str, file_path: &PathBuf) -> Vec<TaintFinding> {
        let mut findings = Vec::new();

        // Parse the source
        let stmts = match ruff_python_parser::parse_module(source) {
            Ok(parsed) => parsed.into_syntax().body,
            Err(_) => return findings,
        };

        let line_index = LineIndex::new(source);

        // Level 1: Intraprocedural
        if self.config.intraprocedural {
            // Analyze module-level statements (not inside functions)
            let mut module_state = super::propagation::TaintState::new();
            for stmt in &stmts {
                intraprocedural::analyze_stmt_public(
                    stmt,
                    self,
                    &mut module_state,
                    &mut findings,
                    file_path,
                    &line_index,
                );
            }

            // Analyze functions
            for stmt in &stmts {
                if let Stmt::FunctionDef(func) = stmt {
                    if func.is_async {
                        let func_findings = intraprocedural::analyze_async_function(
                            func,
                            self,
                            file_path,
                            &line_index,
                            None,
                        );
                        findings.extend(func_findings);
                    } else {
                        let func_findings = intraprocedural::analyze_function(
                            func,
                            self,
                            file_path,
                            &line_index,
                            None,
                        );
                        findings.extend(func_findings);
                    }
                }
            }
        }

        // Level 2: Interprocedural
        // Level 2: Interprocedural
        if self.config.interprocedural {
            let interprocedural_findings =
                interprocedural::analyze_module(&stmts, self, file_path, &line_index);
            findings.extend(interprocedural_findings);
        }

        // Level 3: Cross-file
        if self.config.crossfile {
            let mut cross_file = CrossFileAnalyzer::new();
            let cross_file_findings = cross_file.analyze_file(self, file_path, &stmts, &line_index);
            findings.extend(cross_file_findings);
        }

        // Deduplicate findings
        findings.dedup_by(|a, b| a.source_line == b.source_line && a.sink_line == b.sink_line);

        findings
    }

    /// Analyzes multiple files with cross-file tracking.
    pub fn analyze_project(&mut self, files: &[(PathBuf, String)]) -> Vec<TaintFinding> {
        if self.config.crossfile {
            if let Some(mut analyzer) = self.crossfile_analyzer.take() {
                for (path, source) in files {
                    if let Ok(parsed) = ruff_python_parser::parse_module(source) {
                        let module = parsed.into_syntax();
                        let line_index = LineIndex::new(source);
                        analyzer.analyze_file(self, path, &module.body, &line_index);
                    }
                }
                let findings = analyzer.get_all_findings();
                self.crossfile_analyzer = Some(analyzer);
                return findings;
            }
        }

        // Fall back to per-file analysis
        files
            .iter()
            .flat_map(|(path, source)| self.analyze_file(source, path))
            .collect()
    }

    /// Clears analysis caches.
    pub fn clear_cache(&mut self) {
        if let Some(ref mut analyzer) = self.crossfile_analyzer {
            analyzer.clear_cache();
        }
    }
}

impl Default for TaintAnalyzer {
    fn default() -> Self {
        Self::new(TaintConfig::all_levels())
    }
}
