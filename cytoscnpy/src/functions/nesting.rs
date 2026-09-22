//! Control flow nesting depth calculation for Python statement blocks.

use ruff_python_ast::Stmt;

/// Calculate the maximum control flow nesting depth within a function body.
///
/// Depth starts at 0 for statements directly at the top level of the function body.
/// Each compound control-flow statement (if, for, while, with, try, match) increases
/// nesting depth by 1 for statements contained in its body or clauses.
/// Nested function and class definitions are not descended into because they define
/// their own independent scope and function metrics.
#[must_use]
pub fn calculate_max_nesting(body: &[Stmt]) -> usize {
    let mut max_depth = 0;
    walk_statements(body, 0, &mut max_depth);
    max_depth
}

fn walk_statements(stmts: &[Stmt], current_depth: usize, max_depth: &mut usize) {
    for stmt in stmts {
        walk_stmt(stmt, current_depth, max_depth);
    }
}

fn walk_stmt(stmt: &Stmt, current_depth: usize, max_depth: &mut usize) {
    match stmt {
        Stmt::If(node) => {
            let next_depth = current_depth + 1;
            *max_depth = (*max_depth).max(next_depth);
            walk_statements(&node.body, next_depth, max_depth);
            for clause in &node.elif_else_clauses {
                walk_statements(&clause.body, next_depth, max_depth);
            }
        }
        Stmt::For(node) => {
            let next_depth = current_depth + 1;
            *max_depth = (*max_depth).max(next_depth);
            walk_statements(&node.body, next_depth, max_depth);
            walk_statements(&node.orelse, next_depth, max_depth);
        }
        Stmt::While(node) => {
            let next_depth = current_depth + 1;
            *max_depth = (*max_depth).max(next_depth);
            walk_statements(&node.body, next_depth, max_depth);
            walk_statements(&node.orelse, next_depth, max_depth);
        }
        Stmt::With(node) => {
            let next_depth = current_depth + 1;
            *max_depth = (*max_depth).max(next_depth);
            walk_statements(&node.body, next_depth, max_depth);
        }
        Stmt::Try(node) => {
            let next_depth = current_depth + 1;
            *max_depth = (*max_depth).max(next_depth);
            walk_statements(&node.body, next_depth, max_depth);
            for handler in &node.handlers {
                let ruff_python_ast::ExceptHandler::ExceptHandler(h) = handler;
                walk_statements(&h.body, next_depth, max_depth);
            }
            walk_statements(&node.orelse, next_depth, max_depth);
            walk_statements(&node.finalbody, next_depth, max_depth);
        }
        Stmt::Match(node) => {
            let next_depth = current_depth + 1;
            *max_depth = (*max_depth).max(next_depth);
            for case in &node.cases {
                walk_statements(&case.body, next_depth, max_depth);
            }
        }
        // Do not descend into nested function or class definitions, or unhandled statements
        _ => {}
    }
}
