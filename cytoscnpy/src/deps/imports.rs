use crate::commands::utils::find_python_files;
use rayon::prelude::*;
use ruff_python_ast::{self as ast, Stmt};
use ruff_python_parser::parse_module;
use rustc_hash::FxHashSet;
use std::path::PathBuf;

fn collect_imports(stmts: &[Stmt], imports: &mut FxHashSet<String>) {
    for stmt in stmts {
        match stmt {
            Stmt::Import(import_stmt) => {
                for alias in &import_stmt.names {
                    if let Some(top_level) = alias.name.split('.').next() {
                        imports.insert(top_level.to_owned());
                    }
                }
            }
            Stmt::ImportFrom(import_from) => {
                if import_from.level > 0 {
                    continue;
                }
                if let Some(module) = &import_from.module {
                    if let Some(top_level) = module.as_ref().split('.').next() {
                        imports.insert(top_level.to_owned());
                    }
                }
            }
            Stmt::FunctionDef(f) => collect_imports(&f.body, imports),
            Stmt::ClassDef(c) => collect_imports(&c.body, imports),
            Stmt::If(i) => {
                collect_imports(&i.body, imports);
                for clause in &i.elif_else_clauses {
                    collect_imports(&clause.body, imports);
                }
            }
            Stmt::For(f) => {
                collect_imports(&f.body, imports);
                collect_imports(&f.orelse, imports);
            }
            Stmt::While(w) => {
                collect_imports(&w.body, imports);
                collect_imports(&w.orelse, imports);
            }
            Stmt::With(w) => collect_imports(&w.body, imports),
            Stmt::Try(t) => {
                collect_imports(&t.body, imports);
                for handler in &t.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = handler;
                    collect_imports(&h.body, imports);
                }
                collect_imports(&t.orelse, imports);
                collect_imports(&t.finalbody, imports);
            }
            Stmt::Match(m) => {
                for case in &m.cases {
                    collect_imports(&case.body, imports);
                }
            }
            _ => {}
        }
    }
}

/// Scans Python files within the provided roots and extracts all import names,
/// including imports nested inside functions, classes, and control flow blocks.
pub fn extract_imports(roots: &[PathBuf], exclude: &[String], verbose: bool) -> FxHashSet<String> {
    let files = find_python_files(roots, exclude, verbose);

    files
        .into_par_iter()
        .map(|file| {
            if let Ok(content) = std::fs::read_to_string(&file) {
                if let Ok(parsed) = parse_module(&content) {
                    let mut imports = FxHashSet::default();
                    collect_imports(&parsed.into_syntax().body, &mut imports);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_extract_imports_simple() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.py");
        fs::write(
            &file_path,
            "import os\nfrom sys import path\nimport requests.sessions\n",
        )?;

        let imports = extract_imports(&[dir.path().to_path_buf()], &[], false);
        assert!(imports.contains("os"));
        assert!(imports.contains("sys"));
        assert!(imports.contains("requests"));
        assert_eq!(imports.len(), 3);
        Ok(())
    }

    #[test]
    fn test_extract_imports_skips_relative() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.py");
        fs::write(
            &file_path,
            "from . import local\nfrom ..parent import other\n",
        )?;

        let imports = extract_imports(&[dir.path().to_path_buf()], &[], false);
        assert!(imports.is_empty());
        Ok(())
    }

    #[test]
    fn test_extract_imports_nested_in_function() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.py");
        fs::write(
            &file_path,
            "def foo():\n    import json\n    from pathlib import Path\n",
        )?;

        let imports = extract_imports(&[dir.path().to_path_buf()], &[], false);
        assert!(imports.contains("json"));
        assert!(imports.contains("pathlib"));
        Ok(())
    }

    #[test]
    fn test_extract_imports_nested_in_try_except() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.py");
        fs::write(
            &file_path,
            "try:\n    import ujson as json\nexcept ImportError:\n    import json\n",
        )?;

        let imports = extract_imports(&[dir.path().to_path_buf()], &[], false);
        assert!(imports.contains("ujson"));
        assert!(imports.contains("json"));
        Ok(())
    }

    #[test]
    fn test_extract_imports_nested_in_if() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.py");
        fs::write(
            &file_path,
            "import sys\nif sys.platform == 'win32':\n    import winreg\nelse:\n    import fcntl\n",
        )?;

        let imports = extract_imports(&[dir.path().to_path_buf()], &[], false);
        assert!(imports.contains("sys"));
        assert!(imports.contains("winreg"));
        assert!(imports.contains("fcntl"));
        Ok(())
    }

    #[test]
    fn test_extract_imports_nested_in_class() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.py");
        fs::write(
            &file_path,
            "class Foo:\n    import dataclasses\n    def method(self):\n        import typing\n",
        )?;

        let imports = extract_imports(&[dir.path().to_path_buf()], &[], false);
        assert!(imports.contains("dataclasses"));
        assert!(imports.contains("typing"));
        Ok(())
    }
}
