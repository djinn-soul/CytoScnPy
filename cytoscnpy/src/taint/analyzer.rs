//! Main taint analyzer with plugin architecture.
//!
//! Provides a configurable taint analysis engine that supports:
//! - Built-in sources and sinks
//! - Custom plugin sources and sinks
//! - Configuration via TOML

use super::config::TaintConfig as AnalyzerConfig;
use super::crossfile::CrossFileAnalyzer;
use super::interprocedural;
use super::intraprocedural;
use super::plugins::{
    AzureSourcePlugin as AnalyzerAzureSourcePlugin, BuiltinSinkPlugin as AnalyzerBuiltinSinkPlugin,
    BuiltinSourcePlugin as AnalyzerBuiltinSourcePlugin,
    DjangoSourcePlugin as AnalyzerDjangoSourcePlugin,
    DynamicPatternPlugin as AnalyzerDynamicPatternPlugin,
    FlaskSourcePlugin as AnalyzerFlaskSourcePlugin, PluginRegistry as AnalyzerPluginRegistry,
    TaintSinkPlugin as AnalyzerTaintSinkPlugin, TaintSourcePlugin as AnalyzerTaintSourcePlugin,
};
use super::types::TaintFinding;
use crate::utils::LineIndex;
use ruff_python_ast::Stmt;
use std::path::PathBuf;
use std::sync::Arc;

pub use super::config::{CustomSinkConfig, CustomSourceConfig, TaintConfig};
pub use super::plugins::{
    BuiltinSinkPlugin, BuiltinSourcePlugin, DjangoSourcePlugin, FlaskSourcePlugin, PluginRegistry,
    SanitizerPlugin, TaintSinkPlugin, TaintSourcePlugin,
};

/// Main taint analyzer.
pub struct TaintAnalyzer {
    /// Plugin registry.
    pub plugins: AnalyzerPluginRegistry,
    /// Configuration.
    pub config: AnalyzerConfig,
    /// Cross-file analyzer (if enabled).
    crossfile_analyzer: Option<CrossFileAnalyzer>,
}

impl TaintAnalyzer {
    /// Creates a new taint analyzer with default plugins.
    #[must_use]
    pub fn new(config: AnalyzerConfig) -> Self {
        let mut plugins = AnalyzerPluginRegistry::new();

        plugins.register_source(AnalyzerFlaskSourcePlugin);
        plugins.register_source(AnalyzerDjangoSourcePlugin);
        plugins.register_source(AnalyzerBuiltinSourcePlugin);
        plugins.register_source(AnalyzerAzureSourcePlugin);
        plugins.register_sink(AnalyzerBuiltinSinkPlugin);

        let custom_sources: Vec<String> = config
            .custom_sources
            .iter()
            .map(|source| source.pattern.clone())
            .collect();
        let custom_sinks: Vec<String> = config
            .custom_sinks
            .iter()
            .map(|sink| sink.pattern.clone())
            .collect();

        if !custom_sources.is_empty() || !custom_sinks.is_empty() {
            let dynamic = Arc::new(AnalyzerDynamicPatternPlugin {
                sources: custom_sources,
                sinks: custom_sinks,
            });
            plugins
                .sources
                .push(Arc::clone(&dynamic) as Arc<dyn AnalyzerTaintSourcePlugin>);
            plugins
                .sinks
                .push(dynamic as Arc<dyn AnalyzerTaintSinkPlugin>);
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
    pub fn empty(config: AnalyzerConfig) -> Self {
        Self {
            plugins: AnalyzerPluginRegistry::new(),
            config,
            crossfile_analyzer: None,
        }
    }

    /// Registers a custom source plugin.
    pub fn add_source<T: AnalyzerTaintSourcePlugin + 'static>(&mut self, plugin: T) {
        self.plugins.register_source(plugin);
    }

    /// Registers a custom sink plugin.
    pub fn add_sink<T: AnalyzerTaintSinkPlugin + 'static>(&mut self, plugin: T) {
        self.plugins.register_sink(plugin);
    }

    /// Analyzes a single file.
    #[must_use]
    pub fn analyze_file(&self, source: &str, file_path: &PathBuf) -> Vec<TaintFinding> {
        let mut findings = Vec::new();

        let stmts = match ruff_python_parser::parse_module(source) {
            Ok(parsed) => parsed.into_syntax().body,
            Err(_) => return findings,
        };

        let line_index = LineIndex::new(source);

        if self.config.intraprocedural {
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

            for stmt in &stmts {
                if let Stmt::FunctionDef(func) = stmt {
                    if func.is_async {
                        findings.extend(intraprocedural::analyze_async_function(
                            func,
                            self,
                            file_path,
                            &line_index,
                            None,
                        ));
                    } else {
                        findings.extend(intraprocedural::analyze_function(
                            func,
                            self,
                            file_path,
                            &line_index,
                            None,
                        ));
                    }
                }
            }
        }

        if self.config.interprocedural {
            findings.extend(interprocedural::analyze_module(
                &stmts,
                self,
                file_path,
                &line_index,
            ));
        }

        if self.config.crossfile {
            let mut cross_file = CrossFileAnalyzer::new();
            findings.extend(cross_file.analyze_file(self, file_path, &stmts, &line_index));
        }

        findings.sort_by(|a, b| {
            a.sink_line
                .cmp(&b.sink_line)
                .then(a.source_line.cmp(&b.source_line))
        });
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
        Self::new(AnalyzerConfig::all_levels())
    }
}
