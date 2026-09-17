//! Python AST analysis for detecting magic numbers in logic and comparisons.

use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;
use std::collections::HashSet;
use std::path::Path;

use super::types::{AntiPatternKind, AntiPatternMatch};
use crate::utils::LineIndex;

const MAX_SNIPPET: usize = 120;
const MIN_MAGIC_DIGITS: usize = 3;

struct MagicVisitor<'a> {
    source: &'a str,
    file: &'a Path,
    line_index: &'a LineIndex,
    magic_lines: HashSet<usize>,
    matches: Vec<AntiPatternMatch>,
}

/// Analyzes Python AST for magic numbers in conditionals and comparisons.
#[must_use]
pub fn detect_magic_numbers(
    source: &str,
    file: &Path,
    line_index: &LineIndex,
) -> Vec<AntiPatternMatch> {
    let mut visitor = MagicVisitor {
        source,
        file,
        line_index,
        magic_lines: HashSet::new(),
        matches: Vec::new(),
    };

    if let Ok(parsed) = parse_module(source) {
        visitor.visit_statements(parsed.suite());
    }

    visitor.matches
}

fn is_magic_number(num: &ast::Number) -> bool {
    match num {
        ast::Number::Int(int) => {
            let s = int.to_string();
            s.trim_start_matches('-').len() >= MIN_MAGIC_DIGITS
        }
        ast::Number::Float(f) => {
            let s = f.to_string();
            let int_part = s.split('.').next().unwrap_or("").trim_start_matches('-');
            int_part.len() >= MIN_MAGIC_DIGITS
        }
        ast::Number::Complex { .. } => false,
    }
}

impl MagicVisitor<'_> {
    fn check_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::NumberLiteral(num) if is_magic_number(&num.value) => {
                let line = self.line_index.line_index(expr.range().start());
                if self.magic_lines.insert(line) {
                    let column = self.line_index.column_index(expr.range().start());
                    self.matches.push(AntiPatternMatch {
                        file: self.file.to_path_buf(),
                        line,
                        column,
                        kind: AntiPatternKind::MagicNumber,
                        pattern_name: AntiPatternKind::MagicNumber.as_str().to_owned(),
                        description: AntiPatternKind::MagicNumber.description().to_owned(),
                        snippet: get_line_snippet(self.source, line),
                    });
                }
            }
            Expr::Compare(c) => {
                self.check_expr(&c.left);
                for comparator in &c.comparators {
                    self.check_expr(comparator);
                }
            }
            Expr::UnaryOp(u) => self.check_expr(&u.operand),
            Expr::BinOp(b) => {
                self.check_expr(&b.left);
                self.check_expr(&b.right);
            }
            Expr::BoolOp(b) => {
                for v in &b.values {
                    self.check_expr(v);
                }
            }
            Expr::If(i) => self.check_expr(&i.test),
            _ => {}
        }
    }

    fn visit_statements(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            match stmt {
                Stmt::If(s) => {
                    self.check_expr(&s.test);
                    for clause in &s.elif_else_clauses {
                        if let Some(test) = &clause.test {
                            self.check_expr(test);
                        }
                        self.visit_statements(&clause.body);
                    }
                    self.visit_statements(&s.body);
                }
                Stmt::While(s) => {
                    self.check_expr(&s.test);
                    self.visit_statements(&s.body);
                    self.visit_statements(&s.orelse);
                }
                Stmt::For(s) => {
                    self.visit_statements(&s.body);
                    self.visit_statements(&s.orelse);
                }
                Stmt::FunctionDef(s) => self.visit_statements(&s.body),
                Stmt::ClassDef(s) => self.visit_statements(&s.body),
                Stmt::Try(s) => {
                    self.visit_statements(&s.body);
                    for h in &s.handlers {
                        let ast::ExceptHandler::ExceptHandler(handler) = h;
                        self.visit_statements(&handler.body);
                    }
                    self.visit_statements(&s.orelse);
                    self.visit_statements(&s.finalbody);
                }
                Stmt::Expr(s) => {
                    if let Expr::Compare(_) = &*s.value {
                        self.check_expr(&s.value);
                    }
                }
                _ => {}
            }
        }
    }
}

fn get_line_snippet(source: &str, line_1based: usize) -> String {
    source
        .lines()
        .nth(line_1based.saturating_sub(1))
        .map(|l| l.trim().chars().take(MAX_SNIPPET).collect())
        .unwrap_or_default()
}
