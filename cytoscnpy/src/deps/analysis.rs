use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::declared::{locate_and_parse_declarations, DeclaredDependency};
use super::imports::extract_imports;
use super::installed::{detect_venv, scan_installed, InstalledPackage};
use super::lockfile::load_lockfile_graph;
use super::mapping::{get_package_mapping, get_reverse_mapping};
use super::stdlib::get_stdlib_modules;

/// A branch of transitive packages that would be removable along with an
/// unused declared dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemovableBranch {
    /// The unused declared root package.
    pub root: String,
    /// Transitive packages only used by this root (safe to remove with it).
    pub unique_transitive: Vec<String>,
}

/// The result of the full v3 dependency analysis.
pub struct DepsResult {
    /// Declared but not imported in the codebase.
    pub unused: Vec<DeclaredDependency>,
    /// Imported but not declared in project metadata.
    pub missing: Vec<String>,
    /// Installed in the environment but not declared by the project.
    pub extra_installed: Vec<InstalledPackage>,
    /// Installed, not declared, not imported, and not required by any other installed pkg.
    pub orphan_installed: Vec<InstalledPackage>,
    /// For each unused declared package, what would be removable with it.
    pub removable_branches: Vec<RemovableBranch>,
}

/// Configuration options for the v3 dependency analysis.
#[derive(Clone)]
pub struct DepsOptions<'a> {
    /// Absolute paths to the project roots to analyze.
    pub roots: &'a [PathBuf],
    /// List of paths or patterns to exclude.
    pub exclude: &'a [String],
    /// Optional path to a specific requirements.txt file.
    pub requirements: Option<String>,
    /// List of package names to ignore if unused.
    pub ignore_unused: &'a [String],
    /// List of package or import names to ignore if missing.
    pub ignore_missing: &'a [String],
    /// Whether to print verbose debug output.
    pub verbose: bool,
    /// Whether to output the findings as a JSON string.
    pub json: bool,
    /// Custom package mapping configuration.
    pub package_mapping: Option<&'a FxHashMap<String, Vec<String>>>,
    /// Override path to the virtual environment (default: auto-detect .venv).
    pub venv_path: Option<PathBuf>,
    /// Override path to the lockfile (default: auto-detect uv.lock / poetry.lock).
    pub lockfile_path: Option<PathBuf>,
    /// Whether to include extra-installed packages in the report.
    pub show_extra: bool,
    /// Whether to include orphan packages in the report.
    pub show_orphans: bool,
    /// If set, only report the removal impact for this one package.
    pub impact_package: Option<String>,
}

fn is_local_package(roots: &[PathBuf], module_name: &str) -> bool {
    for root in roots {
        let dir = root.join(module_name);
        if dir.is_dir() {
            // Regular package: explicit init file.
            if dir.join("__init__.py").exists() || dir.join("__init__.pyi").exists() {
                return true;
            }
            // Namespace package (Python 3.3+, PEP 420): a directory without an
            // __init__.py is still a valid package as long as it contains at least
            // one Python source file directly inside it, OR contains a subdirectory
            // that is itself a package (handles `src/` layout where `src/myapp/` is
            // the real package and `from src.myapp import ...` is used).
            if let Ok(entries) = std::fs::read_dir(&dir) {
                let has_py_or_pkg_subdir = entries.filter_map(std::result::Result::ok).any(|e| {
                    let p = e.path();
                    if p.extension().is_some_and(|ext| ext == "py") {
                        return true;
                    }
                    if p.is_dir() {
                        return p.join("__init__.py").exists()
                            || p.join("__init__.pyi").exists()
                            || std::fs::read_dir(&p).is_ok_and(|rd| {
                                rd.filter_map(std::result::Result::ok)
                                    .any(|e2| e2.path().extension().is_some_and(|ext| ext == "py"))
                            });
                    }
                    false
                });
                if has_py_or_pkg_subdir {
                    return true;
                }
            }
        }
        let file = root.join(format!("{module_name}.py"));
        if file.is_file() {
            return true;
        }
    }
    false
}

/// Analyzes dependencies across the project given the provided options.
pub fn analyze_dependencies(options: &DepsOptions<'_>) -> DepsResult {
    let primary_root = options
        .roots
        .first()
        .map(std::path::PathBuf::as_path)
        .unwrap_or_else(|| Path::new("."));

    // ── Step 1: declared + imported (v1 logic) ────────────────────────────────
    let declared = locate_and_parse_declarations(primary_root, options.requirements.as_ref());
    let imported = extract_imports(options.roots, options.exclude, options.verbose);

    let pkg_mapping = get_package_mapping();
    let mut unused = Vec::new();

    for dep in &declared {
        if options
            .ignore_unused
            .iter()
            .any(|ig| ig == &dep.package_name || ig == &dep.normalized_name)
        {
            continue;
        }

        let custom_expected = options.package_mapping.and_then(|m| {
            m.get(dep.package_name.as_str())
                .or_else(|| m.get(dep.normalized_name.as_str()))
        });

        let expected_imports: Vec<&str> = match custom_expected {
            Some(names) => names.iter().map(std::string::String::as_str).collect(),
            None => match pkg_mapping
                .get(dep.package_name.as_str())
                .or_else(|| pkg_mapping.get(dep.normalized_name.as_str()))
            {
                Some(names) => names.clone(),
                None => vec![dep.normalized_name.as_str()],
            },
        };

        let is_used = expected_imports.iter().any(|e| imported.contains(*e));
        if !is_used {
            unused.push(dep.clone());
        }
    }

    let stdlib_modules = get_stdlib_modules();
    let reverse_mapping = get_reverse_mapping();
    let mut missing_set = FxHashSet::default();

    for import_name in &imported {
        if options.ignore_missing.iter().any(|ig| ig == import_name) {
            continue;
        }
        if stdlib_modules.contains(import_name.as_str()) {
            continue;
        }
        if is_local_package(options.roots, import_name) {
            continue;
        }

        let import_lower = import_name.to_lowercase();
        // Try the original casing first (handles entries like "PIL"), then lowercase.
        let pkg_name_guess = reverse_mapping
            .get(import_name.as_str())
            .or_else(|| reverse_mapping.get(import_lower.as_str()))
            .copied()
            .unwrap_or(import_lower.as_str());
        // Normalize separators and case so we can compare against dep.normalized_name.
        let pkg_normalized = pkg_name_guess.to_lowercase().replace('-', "_");

        let is_declared = declared.iter().any(|dep| {
            dep.package_name.to_lowercase() == pkg_name_guess
                || dep.normalized_name == pkg_name_guess
                || dep.normalized_name == pkg_normalized
                || dep.normalized_name == import_lower
        });

        if !is_declared {
            missing_set.insert(import_name.clone());
        }
    }

    let mut missing: Vec<String> = missing_set.into_iter().collect();
    missing.sort();

    // ── Step 2: environment scan (v3) ─────────────────────────────────────────
    let mut extra_installed: Vec<InstalledPackage> = Vec::new();
    let mut orphan_installed: Vec<InstalledPackage> = Vec::new();

    if options.show_extra || options.show_orphans {
        let venv_root = options
            .venv_path
            .clone()
            .or_else(|| detect_venv(primary_root));

        if let Some(venv) = venv_root {
            let installed = scan_installed(&venv);

            // Declared normalized names for fast lookup
            let declared_norm: FxHashSet<String> =
                declared.iter().map(|d| d.normalized_name.clone()).collect();

            // Imported normalized names for orphan detection
            let imported_norm: FxHashSet<String> = imported
                .iter()
                .map(|i| {
                    let i_lower = i.to_lowercase();
                    // Try original casing first (handles "PIL"), then lowercase.
                    reverse_mapping
                        .get(i.as_str())
                        .or_else(|| reverse_mapping.get(i_lower.as_str()))
                        .map(|s| s.to_lowercase().replace('-', "_"))
                        .unwrap_or_else(|| i_lower.replace('-', "_"))
                })
                .collect();

            for (norm_name, pkg) in &installed {
                // Skip packages that are declared
                if declared_norm.contains(norm_name) {
                    continue;
                }
                // Skip stdlib artefacts that sometimes appear in dist-info
                if stdlib_modules.contains(norm_name.as_str()) {
                    continue;
                }

                if options.show_extra {
                    extra_installed.push(pkg.clone());
                }

                if options.show_orphans {
                    // Orphan = not imported, not required by any other installed pkg
                    let is_imported = imported_norm.contains(norm_name);
                    let is_required_by_other = installed.values().any(|other| {
                        other.normalized_name != *norm_name && other.requires.contains(norm_name)
                    });

                    if !is_imported && !is_required_by_other {
                        orphan_installed.push(pkg.clone());
                    }
                }
            }

            extra_installed.sort_by(|a, b| a.normalized_name.cmp(&b.normalized_name));
            orphan_installed.sort_by(|a, b| a.normalized_name.cmp(&b.normalized_name));
        }
    }

    // ── Step 3: removable branch analysis (v3) ────────────────────────────────
    let mut removable_branches: Vec<RemovableBranch> = Vec::new();

    let lockfile_root = options
        .lockfile_path
        .as_deref()
        .and_then(Path::parent)
        .unwrap_or(primary_root);

    if let Some(graph) = load_lockfile_graph(lockfile_root) {
        // Declared normalized names (all, not just unused)
        let all_declared_norm: FxHashSet<String> =
            declared.iter().map(|d| d.normalized_name.clone()).collect();

        let target_unused: Vec<&DeclaredDependency> = if let Some(ref pkg) = options.impact_package
        {
            let norm = pkg.to_lowercase().replace('-', "_");
            declared
                .iter()
                .filter(|d| d.normalized_name == norm)
                .collect()
        } else {
            unused.iter().collect()
        };

        for dep in target_unused {
            let transitive = graph.transitive_deps(&dep.normalized_name);

            // Keep only packages not depended upon by any other declared root
            let unique: Vec<String> = transitive
                .into_iter()
                .filter(|t| {
                    // Check reverse: is this transitive package required by any other declared dep?
                    let required_by_others = graph
                        .reverse
                        .get(t.as_str())
                        .map(|parents| {
                            parents.iter().any(|parent| {
                                *parent != dep.normalized_name && all_declared_norm.contains(parent)
                            })
                        })
                        .unwrap_or(false);
                    !required_by_others
                })
                .collect();

            removable_branches.push(RemovableBranch {
                root: dep.package_name.clone(),
                unique_transitive: unique,
            });
        }
    }

    DepsResult {
        unused,
        missing,
        extra_installed,
        orphan_installed,
        removable_branches,
    }
}
