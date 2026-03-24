use crate::commands::utils::find_python_files;
use rayon::prelude::*;
use ruff_python_ast::Stmt;
use ruff_python_parser::parse_module;
use rustc_hash::FxHashSet;
use std::path::PathBuf;

/// Scans Python files within the provided roots and extracts all top-level import names.
pub fn extract_imports(roots: &[PathBuf], exclude: &[String], verbose: bool) -> FxHashSet<String> {
    let files = find_python_files(roots, exclude, verbose);

    files
        .into_par_iter()
        .map(|file| {
            if let Ok(content) = std::fs::read_to_string(&file) {
                if let Ok(parsed) = parse_module(&content) {
                    let mut imports = FxHashSet::default();
                    for stmt in parsed.into_syntax().body {
                        match stmt {
                            Stmt::Import(import_stmt) => {
                                for alias in &import_stmt.names {
                                    if let Some(top_level) = alias.name.split('.').next() {
                                        imports.insert(top_level.to_owned());
                                    }
                                }
                            }
                            Stmt::ImportFrom(import_from) => {
                                // Skip relative imports
                                if import_from.level > 0 {
                                    continue;
                                }
                                if let Some(module) = &import_from.module {
                                    if let Some(top_level) = module.as_ref().split('.').next() {
                                        imports.insert(top_level.to_owned());
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    return imports;
                }
            }
            FxHashSet::default()
        })
        .reduce(FxHashSet::default, |mut acc: FxHashSet<String>, set| {
            acc.extend(set);
            acc
        })
}
