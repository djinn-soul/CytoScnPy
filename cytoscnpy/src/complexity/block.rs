use ruff_python_ast::{self as ast, Stmt};

mod expression_traversal;
mod statement_traversal;

pub(super) fn calculate_complexity(body: &[Stmt], no_assert: bool) -> usize {
    calculate_complexity_with_nested(body, no_assert, false)
}

pub(super) fn calculate_total_complexity(body: &[Stmt], no_assert: bool) -> usize {
    calculate_complexity_with_nested(body, no_assert, true)
}

fn calculate_complexity_with_nested(
    body: &[Stmt],
    no_assert: bool,
    descend_definitions: bool,
) -> usize {
    let mut visitor = BlockComplexityVisitor {
        complexity: 1,
        no_assert,
        descend_definitions,
    };
    visitor.visit_body(body);
    visitor.complexity
}

struct BlockComplexityVisitor {
    complexity: usize,
    no_assert: bool,
    descend_definitions: bool,
}

fn is_wildcard_case(pattern: &ast::Pattern) -> bool {
    match pattern {
        ast::Pattern::MatchAs(node) => node.pattern.is_none() && node.name.is_none(),
        _ => false,
    }
}

impl BlockComplexityVisitor {
    fn visit_body(&mut self, body: &[Stmt]) {
        for stmt in body {
            self.visit_stmt(stmt);
        }
    }
}
