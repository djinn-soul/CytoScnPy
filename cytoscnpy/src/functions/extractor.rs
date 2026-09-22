//! Python AST function and method metric extractor.

use crate::complexity::calculate_complexity;
use crate::utils::LineIndex;
use ruff_python_ast::{ModModule, Stmt};
use ruff_text_size::Ranged;
use std::path::Path;

use super::nesting::calculate_max_nesting;
use super::types::FunctionInfo;

/// Extracts all function and method records from Python source text.
#[must_use]
pub fn extract_functions(code: &str, file_path: &Path) -> Vec<FunctionInfo> {
    let Ok(parsed) = ruff_python_parser::parse_module(code) else {
        return Vec::new();
    };
    let line_index = LineIndex::new(code);
    extract_functions_from_ast(&parsed.into_syntax(), &line_index, file_path)
}

/// Extracts all function and method records from a parsed Python module AST.
#[must_use]
pub fn extract_functions_from_ast(
    module: &ModModule,
    line_index: &LineIndex,
    file_path: &Path,
) -> Vec<FunctionInfo> {
    let mut extractor = FunctionExtractor {
        file_path,
        line_index,
        class_stack: Vec::new(),
        functions: Vec::new(),
    };
    extractor.visit_body(&module.body);
    extractor.functions
}

struct FunctionExtractor<'a> {
    file_path: &'a Path,
    line_index: &'a LineIndex,
    class_stack: Vec<String>,
    functions: Vec<FunctionInfo>,
}

impl FunctionExtractor<'_> {
    fn visit_body(&mut self, body: &[Stmt]) {
        for stmt in body {
            self.visit_stmt(stmt);
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::FunctionDef(node) => {
                self.record_function(node.name.as_str(), node.range(), &node.body, node.is_async);
            }
            Stmt::ClassDef(node) => {
                self.class_stack.push(node.name.to_string());
                self.visit_body(&node.body);
                self.class_stack.pop();
            }
            Stmt::If(node) => {
                self.visit_body(&node.body);
                for clause in &node.elif_else_clauses {
                    self.visit_body(&clause.body);
                }
            }
            Stmt::For(node) => {
                self.visit_body(&node.body);
                self.visit_body(&node.orelse);
            }
            Stmt::While(node) => {
                self.visit_body(&node.body);
                self.visit_body(&node.orelse);
            }
            Stmt::With(node) => {
                self.visit_body(&node.body);
            }
            Stmt::Try(node) => {
                self.visit_body(&node.body);
                for handler in &node.handlers {
                    let ruff_python_ast::ExceptHandler::ExceptHandler(h) = handler;
                    self.visit_body(&h.body);
                }
                self.visit_body(&node.orelse);
                self.visit_body(&node.finalbody);
            }
            Stmt::Match(node) => {
                for case in &node.cases {
                    self.visit_body(&case.body);
                }
            }
            _ => {}
        }
    }

    fn record_function(
        &mut self,
        name: &str,
        range: ruff_text_size::TextRange,
        body: &[Stmt],
        is_async: bool,
    ) {
        let start_line = self.line_index.line_index(range.start());
        let end_line = self.line_index.line_index(range.end());
        let line_count = end_line.saturating_sub(start_line) + 1;
        let cyclomatic_complexity = calculate_complexity(body, false);
        let max_nesting = calculate_max_nesting(body);

        let is_method = !self.class_stack.is_empty();
        let class_name = if is_method {
            Some(self.class_stack.join("."))
        } else {
            None
        };

        let qualified_name = if let Some(ref cls) = class_name {
            format!("{cls}.{name}")
        } else {
            name.to_owned()
        };

        self.functions.push(FunctionInfo {
            name: qualified_name,
            simple_name: name.to_owned(),
            file: self.file_path.to_path_buf(),
            start_line,
            end_line,
            line_count,
            cyclomatic_complexity,
            max_nesting,
            is_async,
            is_method,
            class_name,
        });

        // Continue visiting the body in case inner functions or classes exist
        self.visit_body(body);
    }
}
