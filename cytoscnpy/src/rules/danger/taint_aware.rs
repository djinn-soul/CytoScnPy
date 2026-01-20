//! Taint-Aware Danger Rules
//!
//! This module provides integration between taint analysis and danger rules
//! to reduce false positives by only flagging issues when tainted data flows
//! to dangerous sinks.

use crate::rules::Finding;
use crate::taint::{TaintAnalyzer, TaintInfo, TaintSource};
use std::collections::HashMap;
use std::path::PathBuf;

/// Extended context that includes taint information for more accurate detection.
#[derive(Debug, Clone, Default)]
pub struct TaintContext {
    /// Map of variable names to their taint information
    pub tainted_vars: HashMap<String, TaintInfo>,
    /// Map of line numbers to taint sources that affect them
    pub tainted_lines: HashMap<usize, Vec<String>>,
}

impl TaintContext {
    /// Creates a new empty taint context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks if a variable is tainted.
    #[must_use]
    pub fn is_tainted(&self, var_name: &str) -> bool {
        self.tainted_vars.contains_key(var_name)
    }

    /// Gets taint info for a variable if it exists.
    #[must_use]
    pub fn get_taint_info(&self, var_name: &str) -> Option<&TaintInfo> {
        self.tainted_vars.get(var_name)
    }

    /// Checks if a line has any tainted data flowing to it.
    #[must_use]
    pub fn is_line_tainted(&self, line: usize) -> bool {
        self.tainted_lines.contains_key(&line)
    }

    /// Adds a tainted variable to the context.
    pub fn add_tainted_var(&mut self, var_name: String, info: TaintInfo) {
        self.tainted_vars.insert(var_name, info);
    }

    /// Marks a line as having tainted data.
    pub fn mark_line_tainted(&mut self, line: usize, source: String) {
        self.tainted_lines.entry(line).or_default().push(source);
    }
}

/// Wrapper that combines danger rules with taint analysis for enhanced accuracy.
pub struct TaintAwareDangerAnalyzer {
    /// The taint analyzer instance
    taint_analyzer: TaintAnalyzer,
}

impl TaintAwareDangerAnalyzer {
    /// Creates a new taint-aware danger analyzer.
    #[must_use]
    pub fn new(taint_analyzer: TaintAnalyzer) -> Self {
        Self { taint_analyzer }
    }

    /// Creates a taint-aware analyzer with custom patterns.
    #[must_use]
    pub fn with_custom(sources: Vec<String>, sinks: Vec<String>) -> Self {
        let config = crate::taint::analyzer::TaintConfig::with_custom(sources, sinks);
        Self {
            taint_analyzer: TaintAnalyzer::new(config),
        }
    }

    /// Creates a taint-aware analyzer with default taint configuration.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self {
            taint_analyzer: TaintAnalyzer::default(),
        }
    }

    /// Analyzes a file and builds a taint context for use by danger rules.
    ///
    /// Returns a `TaintContext` that can be used to enhance danger rule detection
    /// by checking if flagged patterns involve tainted (user-controlled) data.
    #[must_use]
    pub fn build_taint_context(&self, source: &str, file_path: &PathBuf) -> TaintContext {
        let findings = self.taint_analyzer.analyze_file(source, file_path);
        let mut context = TaintContext::new();

        for finding in findings {
            // Extract variable names from flow path if available
            if let Some(first_var) = finding.flow_path.first() {
                let info = TaintInfo::new(TaintSource::Input, finding.source_line);
                context.add_tainted_var(first_var.clone(), info);
            }

            // Mark the sink line as tainted
            context.mark_line_tainted(finding.sink_line, finding.source.clone());
        }

        context
    }

    /// Filters danger findings based on taint analysis to reduce false positives.
    ///
    /// For rules that check for injection vulnerabilities (SQL, command, path traversal, SSRF),
    /// this method filters out findings where the input is not tainted (i.e., not from user input).
    #[must_use]
    pub fn filter_findings_with_taint(
        findings: Vec<Finding>,
        taint_context: &TaintContext,
    ) -> Vec<Finding> {
        findings
            .into_iter()
            .filter(|finding| {
                // These rules should only flag when data is tainted
                if crate::constants::get_taint_sensitive_rules().contains(&finding.rule_id.as_str())
                {
                    // Only keep finding if the line has tainted data
                    taint_context.is_line_tainted(finding.line)
                } else {
                    // Keep all other findings
                    true
                }
            })
            .collect()
    }

    /// Enhances finding severity based on taint analysis.
    ///
    /// If a finding involves tainted data, its severity may be increased
    /// to reflect the higher risk.
    pub fn enhance_severity_with_taint(findings: &mut [Finding], taint_context: &TaintContext) {
        let taint_sensitive_rules = crate::constants::get_taint_sensitive_rules();
        let inherently_dangerous = ["CSP-D001", "CSP-D002", "CSP-D003"];

        for finding in findings.iter_mut() {
            if (taint_sensitive_rules.contains(&finding.rule_id.as_str())
                || inherently_dangerous.contains(&finding.rule_id.as_str()))
                && taint_context.is_line_tainted(finding.line)
            {
                // Upgrade severity for tainted injection findings
                if finding.severity == "HIGH" {
                    "CRITICAL".clone_into(&mut finding.severity);
                } else if finding.severity == "MEDIUM" {
                    "HIGH".clone_into(&mut finding.severity);
                }
            }
        }
    }
}

impl Default for TaintAwareDangerAnalyzer {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_taint_context_new() {
        let ctx = TaintContext::new();
        assert!(ctx.tainted_vars.is_empty());
        assert!(ctx.tainted_lines.is_empty());
    }

    #[test]
    fn test_taint_context_is_tainted() {
        let mut ctx = TaintContext::new();
        assert!(!ctx.is_tainted("user_input"));

        ctx.add_tainted_var(
            "user_input".to_owned(),
            TaintInfo::new(TaintSource::Input, 10),
        );
        assert!(ctx.is_tainted("user_input"));
    }

    #[test]
    fn test_taint_context_line_tainting() {
        let mut ctx = TaintContext::new();
        assert!(!ctx.is_line_tainted(10));

        ctx.mark_line_tainted(10, "from user input".to_owned());
        assert!(ctx.is_line_tainted(10));
    }
}
