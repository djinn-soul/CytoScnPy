use crate::commands::utils::find_python_files;
use crate::utils::LineIndex;
use rayon::prelude::*;
use ruff_python_ast::{self as ast, Stmt};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;
use rustc_hash::FxHashSet;
use std::path::PathBuf;

/// Concrete source location for a top-level import name.
#[derive(Debug, Clone)]
pub struct ImportOccurrence {
    /// Top-level import name seen in source code.
    pub name: String,
    /// Python file containing the import.
    pub file: PathBuf,
    /// 1-indexed source line.
    pub line: usize,
    /// 1-indexed source column.
    pub column: usize,
    /// Whether the file is classified as production code.
    pub is_production: bool,
}

/// Import names split by all files and production files.
pub struct ImportScan {
    /// Imports found anywhere in scanned Python files.
    pub all: FxHashSet<String>,
    /// Imports found in files classified as production code.
    pub production: FxHashSet<String>,
    /// Source evidence for every import occurrence.
    pub occurrences: Vec<ImportOccurrence>,
}

fn add_import_occurrence(
    imports: &mut FxHashSet<String>,
    occurrences: &mut Vec<ImportOccurrence>,
    name: &str,
    stmt: &Stmt,
    file: &std::path::Path,
    line_index: &LineIndex,
    is_production: bool,
) {
    imports.insert(name.to_owned());
    occurrences.push(ImportOccurrence {
        name: name.to_owned(),
        file: file.to_path_buf(),
        line: line_index.line_index(stmt.range().start()),
        column: line_index.column_index(stmt.range().start()),
        is_production,
    });
}

fn collect_imports(
    stmts: &[Stmt],
    imports: &mut FxHashSet<String>,
    occurrences: &mut Vec<ImportOccurrence>,
    file: &std::path::Path,
    line_index: &LineIndex,
    is_production: bool,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Import(import_stmt) => {
                for alias in &import_stmt.names {
                    if let Some(top_level) = alias.name.split('.').next() {
                        add_import_occurrence(
                            imports,
                            occurrences,
                            top_level,
                            stmt,
                            file,
                            line_index,
                            is_production,
                        );
                    }
                }
            }
            Stmt::ImportFrom(import_from) => {
                if import_from.level > 0 {
                    continue;
                }
                if let Some(module) = &import_from.module {
                    if let Some(top_level) = module.as_ref().split('.').next() {
                        add_import_occurrence(
                            imports,
                            occurrences,
                            top_level,
                            stmt,
                            file,
                            line_index,
                            is_production,
                        );
                    }
                }
            }
            Stmt::FunctionDef(f) => collect_imports(
                &f.body,
                imports,
                occurrences,
                file,
                line_index,
                is_production,
            ),
            Stmt::ClassDef(c) => {
                collect_imports(
                    &c.body,
                    imports,
                    occurrences,
                    file,
                    line_index,
                    is_production,
                );
            }
            Stmt::If(i) => {
                collect_imports(
                    &i.body,
                    imports,
                    occurrences,
                    file,
                    line_index,
                    is_production,
                );
                for clause in &i.elif_else_clauses {
                    collect_imports(
                        &clause.body,
                        imports,
                        occurrences,
                        file,
                        line_index,
                        is_production,
                    );
                }
            }
            Stmt::For(f) => {
                collect_imports(
                    &f.body,
                    imports,
                    occurrences,
                    file,
                    line_index,
                    is_production,
                );
                collect_imports(
                    &f.orelse,
                    imports,
                    occurrences,
                    file,
                    line_index,
                    is_production,
                );
            }
            Stmt::While(w) => {
                collect_imports(
                    &w.body,
                    imports,
                    occurrences,
                    file,
                    line_index,
                    is_production,
                );
                collect_imports(
                    &w.orelse,
                    imports,
                    occurrences,
                    file,
                    line_index,
                    is_production,
                );
            }
            Stmt::With(w) => {
                collect_imports(
                    &w.body,
                    imports,
                    occurrences,
                    file,
                    line_index,
                    is_production,
                );
            }
            Stmt::Try(t) => {
                // Ruff's StmtTry covers both `try` and `try*` (StmtTry { is_star, .. }).
                collect_imports(
                    &t.body,
                    imports,
                    occurrences,
                    file,
                    line_index,
                    is_production,
                );
                for handler in &t.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = handler;
                    collect_imports(
                        &h.body,
                        imports,
                        occurrences,
                        file,
                        line_index,
                        is_production,
                    );
                }
                collect_imports(
                    &t.orelse,
                    imports,
                    occurrences,
                    file,
                    line_index,
                    is_production,
                );
                collect_imports(
                    &t.finalbody,
                    imports,
                    occurrences,
                    file,
                    line_index,
                    is_production,
                );
            }
            Stmt::Match(m) => {
                for case in &m.cases {
                    collect_imports(
                        &case.body,
                        imports,
                        occurrences,
                        file,
                        line_index,
                        is_production,
                    );
                }
            }
            _ => {}
        }
    }
}

fn is_test_or_dev_file(file: &std::path::Path) -> bool {
    crate::utils::is_test_path(&file.to_string_lossy())
}

fn extract_imports_from_file(file: &std::path::Path, is_production: bool) -> ImportScan {
    let mut scan = ImportScan {
        all: FxHashSet::default(),
        production: FxHashSet::default(),
        occurrences: Vec::new(),
    };
    if let Ok(content) = std::fs::read_to_string(file) {
        if let Ok(parsed) = parse_module(&content) {
            let line_index = LineIndex::new(&content);
            collect_imports(
                &parsed.into_syntax().body,
                &mut scan.all,
                &mut scan.occurrences,
                file,
                &line_index,
                is_production,
            );
            if is_production {
                scan.production.extend(scan.all.iter().cloned());
            }
        }
    }
    scan
}

/// Scans Python files and returns import names split by all files and
/// production files. Test/dev files are excluded only from the production set.
pub fn extract_import_scan(roots: &[PathBuf], exclude: &[String], verbose: bool) -> ImportScan {
    let files = find_python_files(roots, exclude, verbose);

    files
        .into_par_iter()
        .map(|file| {
            let is_production = !is_test_or_dev_file(&file);
            extract_imports_from_file(&file, is_production)
        })
        .reduce(
            || ImportScan {
                all: FxHashSet::default(),
                production: FxHashSet::default(),
                occurrences: Vec::new(),
            },
            |mut acc, scan| {
                acc.all.extend(scan.all);
                acc.production.extend(scan.production);
                acc.occurrences.extend(scan.occurrences);
                acc
            },
        )
}

/// Scans Python files within the provided roots and extracts all import names,
/// including imports nested inside functions, classes, and control flow blocks.
pub fn extract_imports(roots: &[PathBuf], exclude: &[String], verbose: bool) -> FxHashSet<String> {
    extract_import_scan(roots, exclude, verbose).all
}

#[cfg(test)]
#[path = "imports_tests.rs"]
mod tests;
