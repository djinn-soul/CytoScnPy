use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast as ast;
use ruff_text_size::Ranged;

/// Rule for detecting the use of `assert` in production code.
pub struct AssertUsedRule {
    /// Rule metadata.
    pub metadata: RuleMetadata,
}

impl AssertUsedRule {
    /// Creates a new assert rule instance.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}

impl Rule for AssertUsedRule {
    fn name(&self) -> &'static str {
        "AssertUsedRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }

    fn enter_stmt(&mut self, stmt: &ast::Stmt, context: &Context) -> Option<Vec<Finding>> {
        if context.is_test_file {
            return None;
        }
        if matches!(stmt, ast::Stmt::Assert(_)) {
            return Some(vec![super::super::utils::create_finding(
                "Use of assert detected. The enclosed code will be removed when compiling to optimised byte code.",
                self.metadata,
                context,
                stmt.range().start(),
                "LOW",
            )]);
        }
        None
    }
}
