use crate::metrics::cc_rank;
use crate::utils::LineIndex;
use ruff_text_size::Ranged;

use super::block::calculate_complexity;
use super::visitor::ComplexityVisitor;

/// A finding related to Cyclomatic Complexity.
#[derive(Debug, Clone, PartialEq)]
pub struct ComplexityFinding {
    /// Name of the function, class, or method.
    pub name: String,
    /// The calculated cyclomatic complexity score.
    pub complexity: usize,
    /// The complexity rank (A-F).
    pub rank: char,
    /// The type of the block ("function", "method", "class", "module").
    pub type_: String,
    /// The line number where the block starts.
    pub line: usize,
}

/// Analyzes the cyclomatic complexity of code within a file.
#[must_use]
pub fn analyze_complexity(
    code: &str,
    _path: &std::path::Path,
    no_assert: bool,
) -> Vec<ComplexityFinding> {
    let mut findings = Vec::new();
    if let Ok(parsed) = ruff_python_parser::parse_module(code) {
        let module = parsed.into_syntax();
        let line_index = LineIndex::new(code);

        let mut visitor = ComplexityVisitor::new(&line_index, no_assert);
        let module_complexity = calculate_complexity(&module.body, no_assert);
        if module_complexity > 1 {
            let line = module
                .body
                .first()
                .map_or(1, |stmt| line_index.line_index(stmt.start()));
            visitor.findings.push(ComplexityFinding {
                name: "<module>".to_owned(),
                complexity: module_complexity,
                rank: cc_rank(module_complexity),
                type_: "module".to_owned(),
                line,
            });
        }

        visitor.visit_body(&module.body);
        findings = visitor.findings;
    }
    findings
}

/// Calculates the total cyclomatic complexity of a module.
#[must_use]
pub fn calculate_module_complexity(code: &str) -> Option<usize> {
    if let Ok(parsed) = ruff_python_parser::parse_module(code) {
        let module = parsed.into_syntax();
        return Some(calculate_complexity(&module.body, false));
    }
    None
}
