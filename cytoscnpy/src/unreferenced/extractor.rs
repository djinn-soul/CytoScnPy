//! AST extraction of functions and import references from Python source files.

use ruff_python_ast::{Stmt, StmtClassDef, StmtFunctionDef};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;
use std::path::Path;

use crate::utils::LineIndex;

/// A raw function definition extracted from a source file before heuristic filtering.
#[derive(Debug, Clone)]
pub struct RawFunction {
    /// Function or method name.
    pub name: String,
    /// 1-indexed starting line.
    pub start_line: usize,
    /// 1-indexed ending line.
    pub end_line: usize,
    /// Total physical lines spanned.
    pub line_count: usize,
    /// Node kind: "function" or "method".
    pub node_kind: String,
    /// Enclosing class name if a method.
    pub class_name: Option<String>,
}

/// Extracted file metadata: functions, imported module stems/symbols, and line count.
#[derive(Debug, Clone)]
pub struct ExtractedFile {
    /// Extracted function definitions.
    pub functions: Vec<RawFunction>,
    /// Imported module strings and imported symbol names.
    pub imports: Vec<String>,
    /// Total lines in file.
    pub total_lines: usize,
    /// Raw source text.
    pub content: String,
}

/// Parses a Python source file and extracts its functions, imports, and total lines.
#[must_use]
pub fn extract_python_file(content: &str, _path: &Path) -> ExtractedFile {
    let total_lines = content.lines().count();
    let Ok(parsed) = parse_module(content) else {
        return ExtractedFile {
            functions: Vec::new(),
            imports: Vec::new(),
            total_lines,
            content: content.to_owned(),
        };
    };

    let line_index = LineIndex::new(content);
    let mut functions = Vec::new();
    let mut imports = Vec::new();

    visit_statements(
        parsed.suite(),
        None,
        &line_index,
        &mut functions,
        &mut imports,
    );

    ExtractedFile {
        functions,
        imports,
        total_lines,
        content: content.to_owned(),
    }
}

fn visit_statements(
    stmts: &[Stmt],
    class_context: Option<&str>,
    line_index: &LineIndex,
    functions: &mut Vec<RawFunction>,
    imports: &mut Vec<String>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::FunctionDef(func) => {
                record_function(func, class_context, line_index, functions);
                // Also scan inner statements (e.g. inner classes or methods)
                visit_statements(&func.body, None, line_index, functions, imports);
            }
            Stmt::ClassDef(class_def) => {
                visit_class(class_def, line_index, functions, imports);
            }
            Stmt::Import(imp) => {
                for alias in &imp.names {
                    imports.push(alias.name.to_string());
                }
            }
            Stmt::ImportFrom(imp_from) => {
                if let Some(module) = &imp_from.module {
                    imports.push(module.to_string());
                }
                for alias in &imp_from.names {
                    imports.push(alias.name.to_string());
                }
            }
            Stmt::If(if_stmt) => {
                visit_statements(&if_stmt.body, class_context, line_index, functions, imports);
                for clause in &if_stmt.elif_else_clauses {
                    visit_statements(&clause.body, class_context, line_index, functions, imports);
                }
            }
            Stmt::For(for_stmt) => {
                visit_statements(
                    &for_stmt.body,
                    class_context,
                    line_index,
                    functions,
                    imports,
                );
                visit_statements(
                    &for_stmt.orelse,
                    class_context,
                    line_index,
                    functions,
                    imports,
                );
            }
            Stmt::While(while_stmt) => {
                visit_statements(
                    &while_stmt.body,
                    class_context,
                    line_index,
                    functions,
                    imports,
                );
                visit_statements(
                    &while_stmt.orelse,
                    class_context,
                    line_index,
                    functions,
                    imports,
                );
            }
            Stmt::Try(try_stmt) => {
                visit_statements(
                    &try_stmt.body,
                    class_context,
                    line_index,
                    functions,
                    imports,
                );
                for handler in &try_stmt.handlers {
                    let ruff_python_ast::ExceptHandler::ExceptHandler(h) = handler;
                    visit_statements(&h.body, class_context, line_index, functions, imports);
                }
                visit_statements(
                    &try_stmt.orelse,
                    class_context,
                    line_index,
                    functions,
                    imports,
                );
                visit_statements(
                    &try_stmt.finalbody,
                    class_context,
                    line_index,
                    functions,
                    imports,
                );
            }
            Stmt::With(with_stmt) => {
                visit_statements(
                    &with_stmt.body,
                    class_context,
                    line_index,
                    functions,
                    imports,
                );
            }
            _ => {}
        }
    }
}

fn visit_class(
    class_def: &StmtClassDef,
    line_index: &LineIndex,
    functions: &mut Vec<RawFunction>,
    imports: &mut Vec<String>,
) {
    let class_name = class_def.name.as_str();
    for stmt in &class_def.body {
        if let Stmt::FunctionDef(func) = stmt {
            record_function(func, Some(class_name), line_index, functions);
            visit_statements(&func.body, None, line_index, functions, imports);
        } else {
            visit_statements(
                std::slice::from_ref(stmt),
                Some(class_name),
                line_index,
                functions,
                imports,
            );
        }
    }
}

fn record_function(
    func: &StmtFunctionDef,
    class_context: Option<&str>,
    line_index: &LineIndex,
    functions: &mut Vec<RawFunction>,
) {
    let start_line = line_index.line_index(func.start());
    let end_line = line_index.line_index(func.end());
    let line_count = end_line.saturating_sub(start_line) + 1;

    let node_kind = if class_context.is_some() {
        "method".to_owned()
    } else {
        "function".to_owned()
    };

    functions.push(RawFunction {
        name: func.name.to_string(),
        start_line,
        end_line,
        line_count,
        node_kind,
        class_name: class_context.map(str::to_owned),
    });
}
