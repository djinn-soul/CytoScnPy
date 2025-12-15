use crate::config::Config;
use crate::utils::LineIndex;
use ruff_python_ast::{Expr, Stmt};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone)]
/// Context passed to rules during analysis.
pub struct Context {
    /// Path to the file being analyzed.
    pub filename: PathBuf,
    /// Line index for accurate line/column mapping.
    pub line_index: LineIndex,
    /// Configuration settings.
    pub config: Config,
}

#[derive(Debug, Clone, Serialize)]
/// A single issue found by a rule.
pub struct Finding {
    /// ID of the rule that triggered the finding.
    pub rule_id: String,
    /// Severity level (e.g., "warning", "error").
    pub severity: String,
    /// Description of the issue.
    pub message: String,
    /// File where the issue was found.
    pub file: PathBuf,
    /// Line number.
    pub line: usize,
    /// Column number.
    pub col: usize,
}

/// Trait defining a linting rule.
pub trait Rule: Send + Sync {
    /// Returns the descriptive name of the rule.
    fn name(&self) -> &'static str;
    /// Returns the unique code/ID of the rule.
    fn code(&self) -> &'static str;
    /// Called when entering a statement.
    fn enter_stmt(&mut self, _stmt: &Stmt, _context: &Context) -> Option<Vec<Finding>> {
        None
    }
    /// Called when leaving a statement.
    fn leave_stmt(&mut self, _stmt: &Stmt, _context: &Context) -> Option<Vec<Finding>> {
        None
    }
    /// Called when visiting an expression.
    fn visit_expr(&mut self, _expr: &Expr, _context: &Context) -> Option<Vec<Finding>> {
        None
    }
}

/// Module containing security/danger rules.
pub mod danger;
/// Module containing code quality rules.
pub mod quality;
/// Module containing secret scanning rules.
pub mod secrets;
