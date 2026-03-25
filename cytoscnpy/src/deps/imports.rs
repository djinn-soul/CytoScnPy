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
}
