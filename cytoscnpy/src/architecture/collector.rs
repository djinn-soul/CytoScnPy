//! Extracts raw import declarations and occurrences from Python AST.

use crate::utils::LineIndex;
use ruff_python_ast::{self as ast, Stmt};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;
use std::path::Path;

/// Raw import occurrence extracted from AST before project-level module resolution.
#[derive(Debug, Clone)]
pub struct RawImport {
    /// Specified module path in import statement (e.g., `os.path`, `subpackage`).
    pub module: Option<String>,
    /// Specific symbol or submodule imported (e.g. `bar` in `from foo import bar`).
    pub imported_symbol: Option<String>,
    /// Relative import dot count (0 for absolute, 1 for `.`, 2 for `..`, etc.).
    pub level: u32,
    /// 1-indexed source line.
    pub line: usize,
    /// 1-indexed source column.
    pub column: usize,
    /// Whether this import is enclosed within a `TYPE_CHECKING` guard.
    pub is_type_checking: bool,
    /// Whether this import is at module scope (outside function or class bodies).
    pub is_top_level: bool,
}

use super::collector_type_checking::TypeCheckingState;

/// Parses a Python source file and extracts all import statements.
pub fn collect_raw_imports(file: &Path) -> Vec<RawImport> {
    let Ok(content) = std::fs::read_to_string(file) else {
        return Vec::new();
    };
    let Ok(parsed) = parse_module(&content) else {
        return Vec::new();
    };

    let line_index = LineIndex::new(&content);
    let mut imports = Vec::new();
    let mut tc_state = TypeCheckingState::default();

    traverse_stmts(
        &parsed.into_syntax().body,
        &mut imports,
        &mut tc_state,
        &line_index,
        true,
        false,
    );

    imports
}

fn traverse_stmts(
    stmts: &[Stmt],
    imports: &mut Vec<RawImport>,
    tc: &mut TypeCheckingState,
    line_index: &LineIndex,
    is_top_level: bool,
    in_type_checking: bool,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Import(imp) => {
                tc.record_import(imp);
                for alias in &imp.names {
                    let line = line_index.line_index(stmt.range().start());
                    let column = line_index.column_index(stmt.range().start());
                    imports.push(RawImport {
                        module: Some(alias.name.to_string()),
                        imported_symbol: None,
                        level: 0,
                        line,
                        column,
                        is_type_checking: in_type_checking,
                        is_top_level,
                    });
                }
            }
            Stmt::ImportFrom(imp) => {
                tc.record_import_from(imp);
                let line = line_index.line_index(stmt.range().start());
                let column = line_index.column_index(stmt.range().start());
                let mod_name = imp.module.as_ref().map(ToString::to_string);

                if imp.names.is_empty() {
                    imports.push(RawImport {
                        module: mod_name,
                        imported_symbol: None,
                        level: imp.level,
                        line,
                        column,
                        is_type_checking: in_type_checking,
                        is_top_level,
                    });
                } else {
                    for alias in &imp.names {
                        let sym = if alias.name.as_str() == "*" {
                            None
                        } else {
                            Some(alias.name.to_string())
                        };
                        imports.push(RawImport {
                            module: mod_name.clone(),
                            imported_symbol: sym,
                            level: imp.level,
                            line,
                            column,
                            is_type_checking: in_type_checking,
                            is_top_level,
                        });
                    }
                }
            }
            Stmt::FunctionDef(f) => {
                traverse_stmts(&f.body, imports, tc, line_index, false, in_type_checking);
            }
            Stmt::ClassDef(c) => {
                traverse_stmts(&c.body, imports, tc, line_index, false, in_type_checking);
            }
            Stmt::If(i) => {
                let guarded = tc.is_type_checking_test(&i.test);
                traverse_stmts(
                    &i.body,
                    imports,
                    tc,
                    line_index,
                    is_top_level,
                    in_type_checking || guarded,
                );
                for clause in &i.elif_else_clauses {
                    let clause_guarded = clause
                        .test
                        .as_ref()
                        .is_some_and(|t| tc.is_type_checking_test(t));
                    traverse_stmts(
                        &clause.body,
                        imports,
                        tc,
                        line_index,
                        is_top_level,
                        in_type_checking || clause_guarded,
                    );
                }
            }
            Stmt::Try(t) => {
                traverse_stmts(
                    &t.body,
                    imports,
                    tc,
                    line_index,
                    is_top_level,
                    in_type_checking,
                );
                for h in &t.handlers {
                    let ast::ExceptHandler::ExceptHandler(handler) = h;
                    traverse_stmts(
                        &handler.body,
                        imports,
                        tc,
                        line_index,
                        is_top_level,
                        in_type_checking,
                    );
                }
                traverse_stmts(
                    &t.orelse,
                    imports,
                    tc,
                    line_index,
                    is_top_level,
                    in_type_checking,
                );
                traverse_stmts(
                    &t.finalbody,
                    imports,
                    tc,
                    line_index,
                    is_top_level,
                    in_type_checking,
                );
            }
            Stmt::For(f) => {
                traverse_stmts(
                    &f.body,
                    imports,
                    tc,
                    line_index,
                    is_top_level,
                    in_type_checking,
                );
                traverse_stmts(
                    &f.orelse,
                    imports,
                    tc,
                    line_index,
                    is_top_level,
                    in_type_checking,
                );
            }
            Stmt::While(w) => {
                traverse_stmts(
                    &w.body,
                    imports,
                    tc,
                    line_index,
                    is_top_level,
                    in_type_checking,
                );
                traverse_stmts(
                    &w.orelse,
                    imports,
                    tc,
                    line_index,
                    is_top_level,
                    in_type_checking,
                );
            }
            Stmt::With(w) => {
                traverse_stmts(
                    &w.body,
                    imports,
                    tc,
                    line_index,
                    is_top_level,
                    in_type_checking,
                );
            }
            Stmt::Match(m) => {
                for case in &m.cases {
                    traverse_stmts(
                        &case.body,
                        imports,
                        tc,
                        line_index,
                        is_top_level,
                        in_type_checking,
                    );
                }
            }
            _ => {}
        }
    }
}
