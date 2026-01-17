//! Cross-file taint analysis.
//!
//! Tracks taint flow across module boundaries.

use super::analyzer::TaintAnalyzer;
use super::interprocedural;
use super::summaries::{get_builtin_summaries, SummaryDatabase};
use super::types::TaintFinding;
use crate::utils::LineIndex;
use ruff_python_ast::Stmt;
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};

/// Cross-file taint analysis database.
#[derive(Debug, Default)]
pub struct CrossFileAnalyzer {
    /// Summaries per module
    module_summaries: FxHashMap<String, SummaryDatabase>,
    /// Import mappings: (`importing_module`, `alias`) -> (`actual_module`, `actual_name`)
    import_map: FxHashMap<(String, String), (String, String)>,
    /// Cache of taint findings by file
    findings_cache: FxHashMap<PathBuf, Vec<TaintFinding>>,
}

impl CrossFileAnalyzer {
    /// Creates a new cross-file analyzer.
    #[must_use]
    pub fn new() -> Self {
        let mut analyzer = Self::default();

        // Initialize with builtin summaries
        let mut builtin_db = SummaryDatabase::new();
        for (name, summary) in get_builtin_summaries() {
            builtin_db.summaries.insert(name, summary);
        }
        analyzer
            .module_summaries
            .insert("__builtins__".to_owned(), builtin_db);

        analyzer
    }

    /// Registers an import mapping.
    pub fn register_import(
        &mut self,
        importing_module: &str,
        alias: &str,
        actual_module: &str,
        actual_name: &str,
    ) {
        self.import_map.insert(
            (importing_module.to_owned(), alias.to_owned()),
            (actual_module.to_owned(), actual_name.to_owned()),
        );
    }

    /// Resolves an imported name to its actual module and name.
    #[must_use]
    pub fn resolve_import(&self, module: &str, name: &str) -> Option<(&str, &str)> {
        self.import_map
            .get(&(module.to_owned(), name.to_owned()))
            .map(|(m, n)| (m.as_str(), n.as_str()))
    }

    /// Analyzes a file and caches the results.
    pub fn analyze_file(
        &mut self,
        analyzer: &TaintAnalyzer,
        file_path: &Path,
        stmts: &[Stmt],
        line_index: &LineIndex,
    ) -> Vec<TaintFinding> {
        // Check cache
        if let Some(findings) = self.findings_cache.get(Path::new(file_path)) {
            return findings.clone();
        }

        // Extract imports first
        self.extract_imports(file_path, stmts);

        // Perform interprocedural analysis
        let findings = interprocedural::analyze_module(stmts, analyzer, file_path, line_index);

        // Cache results
        self.findings_cache
            .insert(file_path.to_path_buf(), findings.clone());

        findings
    }

    /// Extracts import statements and registers them.
    fn extract_imports(&mut self, file_path: &Path, stmts: &[ruff_python_ast::Stmt]) {
        let module_name = file_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        for stmt in stmts {
            match stmt {
                ruff_python_ast::Stmt::Import(import) => {
                    for alias in &import.names {
                        let actual_name = alias.name.to_string();
                        let imported_as = alias
                            .asname
                            .as_ref()
                            .map_or_else(|| actual_name.clone(), |id| id.as_str().to_owned());

                        self.register_import(
                            &module_name,
                            &imported_as,
                            &actual_name,
                            &actual_name,
                        );
                    }
                }
                ruff_python_ast::Stmt::ImportFrom(import) => {
                    if let Some(module) = &import.module {
                        for alias in &import.names {
                            let actual_name = alias.name.to_string();
                            let imported_as = alias
                                .asname
                                .as_ref()
                                .map_or_else(|| actual_name.clone(), |id| id.as_str().to_owned());

                            self.register_import(
                                &module_name,
                                &imported_as,
                                module.as_ref(),
                                &actual_name,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Gets summaries for a module.
    #[must_use]
    pub fn get_module_summaries(&self, module: &str) -> Option<&SummaryDatabase> {
        self.module_summaries.get(module)
    }

    /// Checks if a function from another module taints its return.
    #[must_use]
    pub fn external_function_taints_return(&self, module: &str, func: &str) -> bool {
        self.module_summaries
            .get(module)
            .and_then(|db| db.get(func))
            .is_some_and(|s| s.returns_tainted)
    }

    /// Merges all findings from analyzed files.
    #[must_use]
    pub fn get_all_findings(&self) -> Vec<TaintFinding> {
        self.findings_cache
            .values()
            .flat_map(|f| f.iter().cloned())
            .collect()
    }

    /// Clears the analysis cache.
    pub fn clear_cache(&mut self) {
        self.findings_cache.clear();
    }
}

/// Analyzes multiple files for cross-file taint flow.
#[must_use]
pub fn analyze_project(
    analyzer_ctx: &TaintAnalyzer,
    files: &[(PathBuf, String)],
) -> Vec<TaintFinding> {
    let mut analyzer = CrossFileAnalyzer::new();

    // First pass: build import maps and summaries
    for (path, source) in files {
        if let Ok(parsed) = ruff_python_parser::parse_module(source) {
            let module = parsed.into_syntax();
            let line_index = LineIndex::new(source);
            analyzer.analyze_file(analyzer_ctx, path, &module.body, &line_index);
        }
    }

    // Collect all findings
    analyzer.get_all_findings()
}
