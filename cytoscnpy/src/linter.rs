use crate::config::Config;
use crate::rules::{Context, Finding, Rule};
use crate::utils::LineIndex;
use std::path::PathBuf;

mod expression_traversal;
mod statement_traversal;

/// Visitor for traversing the AST and applying linter rules.
pub struct LinterVisitor {
    rules: Vec<Box<dyn Rule>>,
    context: Context,
    /// List of findings collected during the traversal.
    pub findings: Vec<Finding>,
}

impl LinterVisitor {
    /// Creates a new `LinterVisitor` with the given rules and context.
    #[must_use]
    pub fn new(
        rules: Vec<Box<dyn Rule>>,
        filename: PathBuf,
        line_index: LineIndex,
        config: Config,
        is_test_file: bool,
    ) -> Self {
        Self {
            rules,
            context: Context {
                filename,
                line_index,
                config,
                is_test_file,
            },
            findings: Vec::new(),
        }
    }
}
