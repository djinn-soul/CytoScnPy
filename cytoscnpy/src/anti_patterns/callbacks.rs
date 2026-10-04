//! Detect nested closures and callback registrations using Python syntax.
use super::types::{AntiPatternKind, AntiPatternMatch};
use crate::utils::LineIndex;
use ruff_python_ast::{
    visitor::{self, Visitor},
    Expr, Stmt,
};
use ruff_text_size::Ranged;
use std::path::Path;

/// Detect nested closures or deeply nested callback registrations, excluding ordinary control flow.
#[must_use]
pub fn detect_nested_callbacks(source: &str, file: &Path) -> Vec<AntiPatternMatch> {
    let Ok(parsed) = ruff_python_parser::parse_module(source) else {
        return Vec::new();
    };
    detect_nested_callbacks_ast(source, file, parsed.suite())
}

pub(crate) fn detect_nested_callbacks_ast(
    source: &str,
    file: &Path,
    body: &[Stmt],
) -> Vec<AntiPatternMatch> {
    let mut detector = CallbackVisitor {
        source,
        file,
        lines: LineIndex::new(source),
        closures: 0,
        blocks: 0,
        matches: Vec::new(),
    };
    detector.visit_body(body);
    detector.matches
}

struct CallbackVisitor<'a> {
    source: &'a str,
    file: &'a Path,
    lines: LineIndex,
    closures: usize,
    blocks: usize,
    matches: Vec<AntiPatternMatch>,
}

impl CallbackVisitor<'_> {
    fn record(&mut self, node: &impl Ranged) {
        let line = self.lines.line_index(node.range().start());
        if self.matches.iter().any(|finding| finding.line == line) {
            return;
        }
        self.matches.push(AntiPatternMatch {
            file: self.file.to_path_buf(),
            line,
            column: self.lines.column_index(node.range().start()),
            kind: AntiPatternKind::DeeplyNestedCallback,
            pattern_name: AntiPatternKind::DeeplyNestedCallback.as_str().to_owned(),
            description: AntiPatternKind::DeeplyNestedCallback
                .description()
                .to_owned(),
            snippet: self
                .source
                .lines()
                .nth(line - 1)
                .unwrap_or("")
                .trim()
                .chars()
                .take(120)
                .collect(),
        });
    }
}

impl<'a> Visitor<'a> for CallbackVisitor<'_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        let closure = matches!(stmt, Stmt::FunctionDef(_));
        let block = matches!(
            stmt,
            Stmt::If(_)
                | Stmt::For(_)
                | Stmt::While(_)
                | Stmt::With(_)
                | Stmt::Try(_)
                | Stmt::Match(_)
        );
        self.closures += usize::from(closure);
        self.blocks += usize::from(block);
        if closure && self.closures >= 3 {
            self.record(stmt);
        }
        visitor::walk_stmt(self, stmt);
        self.closures -= usize::from(closure);
        self.blocks -= usize::from(block);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        let closure = matches!(expr, Expr::Lambda(_));
        self.closures += usize::from(closure);
        let callback = matches!(expr, Expr::Call(call) if matches!(call.func.as_ref(), Expr::Attribute(attr)
            if matches!(attr.attr.as_str(), "then" | "catch" | "add_done_callback")));
        if (closure && self.closures >= 3) || (callback && self.blocks >= 3) {
            self.record(expr);
        }
        visitor::walk_expr(self, expr);
        self.closures -= usize::from(closure);
    }
}
