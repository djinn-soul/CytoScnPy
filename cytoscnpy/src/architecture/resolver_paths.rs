//! Path-to-module derivation, package root discovery, and relative path resolution.

use std::path::{Path, PathBuf};

/// Checks whether a file or directory is inside a Python package containing `__init__.py`.
pub fn file_in_package(file: &Path) -> bool {
    let dir = if file.is_file() || file.extension().is_some_and(|e| e == "py") {
        file.parent().unwrap_or(Path::new("."))
    } else {
        file
    };
    dir.join("__init__.py").is_file()
}

/// Walks up the directory tree while `__init__.py` exists to find the package root.
pub fn find_package_root(path: &Path) -> PathBuf {
    let dir = if path.is_file() || path.extension().is_some_and(|e| e == "py") {
        path.parent().unwrap_or(Path::new("."))
    } else {
        path
    };

    let dir = if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    };

    let mut current = dir;
    while current.join("__init__.py").is_file() {
        if let Some(parent) = current.parent() {
            if parent.as_os_str().is_empty() {
                return PathBuf::from(".");
            }
            current = parent;
        } else {
            break;
        }
    }
    current.to_path_buf()
}

/// Derives the canonical relative path for a file with respect to package roots and directory roots.
pub fn find_best_relative_path(file: &Path, roots: &[PathBuf]) -> PathBuf {
    // 1. Check explicitly provided roots first to preserve namespace packages relative to import root
    for root in roots {
        let effective_root = if root.is_file() {
            if file_in_package(root) {
                find_package_root(root)
            } else {
                root.parent().unwrap_or(Path::new(".")).to_path_buf()
            }
        } else if root.is_dir() && root.join("__init__.py").is_file() {
            find_package_root(root)
        } else {
            root.clone()
        };

        if effective_root.as_os_str().is_empty() || effective_root == Path::new(".") {
            let stripped = file.strip_prefix(".").unwrap_or(file);
            if !stripped.as_os_str().is_empty() {
                return stripped.to_path_buf();
            }
        } else if let Ok(rel) = file.strip_prefix(&effective_root) {
            if !rel.as_os_str().is_empty() {
                return rel.to_path_buf();
            }
        }
    }

    // 2. Fallback to package root if file belongs to a package with __init__.py
    let pkg_root = find_package_root(file);
    if file_in_package(file) {
        if let Ok(rel) = file.strip_prefix(&pkg_root) {
            if !rel.as_os_str().is_empty() {
                return rel.to_path_buf();
            }
        }
        if pkg_root.as_os_str().is_empty() || pkg_root == Path::new(".") {
            let stripped = file.strip_prefix(".").unwrap_or(file);
            if !stripped.as_os_str().is_empty() {
                return stripped.to_path_buf();
            }
        }
    }

    // 3. Fallback to parent directory
    if let Some(parent) = file.parent() {
        if !parent.as_os_str().is_empty() {
            if let Ok(rel) = file.strip_prefix(parent) {
                if !rel.as_os_str().is_empty() {
                    return rel.to_path_buf();
                }
            }
        }
    }

    file.strip_prefix(".").unwrap_or(file).to_path_buf()
}

/// Converts a file path to its canonical dotted Python module name.
pub fn path_to_module_name(path: &Path) -> (String, bool) {
    let mut parts: Vec<String> = Vec::new();
    let mut is_init = false;

    for comp in path.components() {
        let s = comp.as_os_str().to_string_lossy();
        if s.ends_with(".py") {
            let stem = s.trim_end_matches(".py");
            if stem == "__init__" {
                is_init = true;
            } else {
                parts.push(stem.to_owned());
            }
        } else {
            parts.push(s.into_owned());
        }
    }

    let name = if parts.is_empty() {
        "__main__".to_owned()
    } else {
        parts.join(".")
    };

    (name, is_init)
}

/// Extracts the top-level package or root group for a module.
pub fn extract_package_group(module_name: &str) -> String {
    let first = module_name.split('.').next().unwrap_or(module_name);
    if first.is_empty() {
        "<root>".to_owned()
    } else {
        first.to_owned()
    }
}

/// Counts lines of code in a source file.
pub fn count_lines(path: &Path) -> usize {
    std::fs::read_to_string(path)
        .map(|s| s.lines().count())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_namespace_package_preserves_namespace_prefix() {
        let temp = tempdir().unwrap();
        let root = temp.path();

        // Structure: root/ns/pkg/a.py where ONLY pkg has __init__.py (ns is a PEP 420 namespace package)
        let ns_dir = root.join("ns");
        let pkg_dir = ns_dir.join("pkg");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(pkg_dir.join("__init__.py"), "").unwrap();
        let a_file = pkg_dir.join("a.py");
        std::fs::write(&a_file, "def fn(): pass\n").unwrap();

        let rel = find_best_relative_path(&a_file, &[root.to_path_buf()]);
        let (name, is_init) = path_to_module_name(&rel);
        assert_eq!(name, "ns.pkg.a");
        assert!(!is_init);
    }
}
