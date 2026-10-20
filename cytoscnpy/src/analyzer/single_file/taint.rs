//! Per-file taint analysis on the already parsed module.

use crate::analyzer::CytoScnPy;
use crate::taint::analyzer::{TaintAnalyzer, TaintConfig};
use crate::taint::types::TaintFinding;
use crate::utils::{LineIndex, Suppression};
use ruff_python_ast::Stmt;
use rustc_hash::FxHashMap;
use std::path::Path;

impl CytoScnPy {
    /// Returns the taint analyzer for the current `config`, building it on
    /// first use, or `None` when taint analysis is disabled.
    fn taint_analyzer(&self) -> Option<&TaintAnalyzer> {
        self.taint_analyzer
            .get_or_init(|| {
                let danger_config = &self.config.cytoscnpy.danger_config;
                let enabled = self.enable_danger
                    && danger_config
                        .enable_taint
                        .unwrap_or(crate::constants::TAINT_ENABLED_DEFAULT);
                enabled.then(|| {
                    let sanitizers = crate::taint::sanitizers::SanitizerConfig::from_danger_config(
                        danger_config,
                    );
                    TaintAnalyzer::new(TaintConfig::with_custom(
                        danger_config.custom_sources.clone().unwrap_or_default(),
                        danger_config.custom_sinks.clone().unwrap_or_default(),
                        sanitizers,
                    ))
                })
            })
            .as_ref()
    }

    /// Taint findings for one parsed file, without suppressed findings.
    pub(super) fn file_taint_findings(
        &self,
        stmts: &[Stmt],
        file_path: &Path,
        line_index: &LineIndex,
        ignored_lines: &FxHashMap<usize, Suppression>,
    ) -> Vec<TaintFinding> {
        let Some(analyzer) = self.taint_analyzer() else {
            return Vec::new();
        };
        let mut findings = analyzer.analyze_module(stmts, &file_path.to_path_buf(), line_index);
        findings.retain(|finding| {
            !crate::utils::is_line_suppressed(ignored_lines, finding.sink_line, &finding.rule_id)
                && !self.is_rule_ignored_for_path(&finding.file, &finding.rule_id)
        });
        findings
    }
}
