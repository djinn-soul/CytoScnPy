use super::{finding::create_finding, CAT_MAINTAINABILITY};
use crate::rules::ids;
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::Stmt;
use ruff_text_size::{Ranged, TextSize};
use std::collections::HashSet;
const META_ARGUMENT_COUNT: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_ARGUMENT_COUNT,
    category: CAT_MAINTAINABILITY,
};
const META_FUNCTION_LENGTH: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_FUNCTION_LENGTH,
    category: CAT_MAINTAINABILITY,
};
const META_NESTING: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_NESTING,
    category: CAT_MAINTAINABILITY,
};
pub(super) struct ArgumentCountRule {
    max_args: usize,
}
impl ArgumentCountRule {
    pub(super) fn new(max_args: usize) -> Self {
        Self { max_args }
    }
}
impl Rule for ArgumentCountRule {
    fn name(&self) -> &'static str {
        "ArgumentCountRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_ARGUMENT_COUNT
    }
    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        let (parameters, name_start) = match stmt {
            Stmt::FunctionDef(f) => (&f.parameters, f.name.range().start()),
            _ => return None,
        };
        let total_args = parameters.posonlyargs.len()
            + parameters.args.len()
            + parameters.kwonlyargs.len()
            + usize::from(parameters.vararg.is_some())
            + usize::from(parameters.kwarg.is_some());
        if total_args > self.max_args {
            return Some(vec![create_finding(
                &format!("Too many arguments ({total_args} > {})", self.max_args),
                META_ARGUMENT_COUNT,
                context,
                name_start,
                "LOW",
            )]);
        }
        None
    }
}
pub(super) struct FunctionLengthRule {
    max_lines: usize,
}
impl FunctionLengthRule {
    pub(super) fn new(max_lines: usize) -> Self {
        Self { max_lines }
    }
}
impl Rule for FunctionLengthRule {
    fn name(&self) -> &'static str {
        "FunctionLengthRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_FUNCTION_LENGTH
    }
    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        let (name_start, stmt_range_end) = match stmt {
            Stmt::FunctionDef(f) => (f.name.range().start(), stmt.range().end()),
            _ => return None,
        };
        let start_line = context.line_index.line_index(name_start);
        let end_line = context.line_index.line_index(stmt_range_end);
        let length = end_line.saturating_sub(start_line) + 1;
        if length > self.max_lines {
            return Some(vec![create_finding(
                &format!("Function too long ({length} > {} lines)", self.max_lines),
                META_FUNCTION_LENGTH,
                context,
                name_start,
                "LOW",
            )]);
        }
        None
    }
}
pub(super) struct NestingRule {
    current_depth: usize,
    max_depth: usize,
    reported_lines: HashSet<usize>,
}
impl NestingRule {
    pub(super) fn new(max_depth: usize) -> Self {
        Self {
            current_depth: 0,
            max_depth,
            reported_lines: HashSet::new(),
        }
    }
    fn check_depth(&mut self, context: &Context, location: TextSize) -> Option<Finding> {
        if self.current_depth <= self.max_depth {
            return None;
        }
        let line = context.line_index.line_index(location);
        if self.reported_lines.contains(&line) {
            return None;
        }
        self.reported_lines.insert(line);
        Some(create_finding(
            &format!("Deeply nested code (depth {})", self.current_depth),
            META_NESTING,
            context,
            location,
            "LOW",
        ))
    }
    fn should_increase_depth(stmt: &Stmt) -> bool {
        matches!(
            stmt,
            Stmt::If(_)
                | Stmt::For(_)
                | Stmt::While(_)
                | Stmt::Try(_)
                | Stmt::With(_)
                | Stmt::Match(_)
        )
    }
}
impl Rule for NestingRule {
    fn name(&self) -> &'static str {
        "NestingRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_NESTING
    }
    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        if !Self::should_increase_depth(stmt) {
            return None;
        }
        self.current_depth += 1;
        self.check_depth(context, stmt.range().start()).map(|f| vec![f])
    }
    fn leave_stmt(&mut self, stmt: &Stmt, _context: &Context) -> Option<Vec<Finding>> {
        if Self::should_increase_depth(stmt) && self.current_depth > 0 {
            self.current_depth -= 1;
        }
        None
    }
}

