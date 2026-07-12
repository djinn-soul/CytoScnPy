use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::declared::{locate_and_parse_declarations, DeclaredDependency, DependencySource};
use super::imports::{extract_import_scan, ImportOccurrence};
use super::installed::{detect_venv, scan_installed, InstalledPackage};
use super::lockfile::{load_lockfile_graph, load_lockfile_graph_at};
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

/// Source location for an import that contributed to a dependency finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyImportLocation {
    /// Python file containing the import.
    pub file: PathBuf,
    /// 1-indexed source line.
    pub line: usize,
    /// 1-indexed source column.
    pub column: usize,
}

impl From<&ImportOccurrence> for DependencyImportLocation {
    fn from(value: &ImportOccurrence) -> Self {
        Self {
            file: value.file.clone(),
            line: value.line,
            column: value.column,
        }
    }
}

/// Imported package that is not declared directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingDependency {
    /// Top-level import name seen in source code.
    pub import_name: String,
    /// Source locations where the import appears.
    #[serde(default)]
    pub locations: Vec<DependencyImportLocation>,
}

/// Imported package that is available only through another dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitiveDependency {
    /// Top-level import name seen in source code.
    pub import_name: String,
    /// Normalized package name found in the lockfile graph.
    pub package_name: String,
    /// Source locations where the import appears.
    #[serde(default)]
    pub locations: Vec<DependencyImportLocation>,
}

/// Development dependency imported from production code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevDependencyInProduction {
    /// Top-level import name seen in production source code.
    pub import_name: String,
    /// Declared development dependency that provides the import.
    pub dependency: DeclaredDependency,
    /// Production source locations where the import appears.
    #[serde(default)]
    pub locations: Vec<DependencyImportLocation>,
}

/// The result of the full v3 dependency analysis.
pub struct DepsResult {
    /// Declared but not imported in the codebase.
    pub unused: Vec<DeclaredDependency>,
    /// Imported but not declared in project metadata.
    pub missing: Vec<String>,
    /// Missing dependency findings with source evidence.
    pub missing_details: Vec<MissingDependency>,
    /// Installed in the environment but not declared by the project.
    pub extra_installed: Vec<InstalledPackage>,
    /// Installed, not declared, not imported, and not required by any other installed pkg.
    pub orphan_installed: Vec<InstalledPackage>,
    /// For each unused declared package, what would be removable with it.
    pub removable_branches: Vec<RemovableBranch>,
    /// Imported packages that are present only as transitive lockfile dependencies.
    pub transitive: Vec<TransitiveDependency>,
    /// Development dependencies imported from production files.
    pub dev_in_production: Vec<DevDependencyInProduction>,
    /// Declared packages that are part of the Python standard library.
    pub stdlib: Vec<DeclaredDependency>,
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
    /// Whether development dependencies should be reported as unused.
    pub include_dev_unused: bool,
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
            // that is itself a package.
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
        if root.join(format!("{module_name}.py")).is_file()
            || root.join(format!("{module_name}.pyi")).is_file()
            || root.join(format!("{module_name}.so")).is_file()
            || root.join(format!("{module_name}.pyd")).is_file()
        {
            return true;
        }
    }
    false
}

// ── Step helpers ─────────────────────────────────────────────────────────────

fn find_unused_declared(
    declared: &[DeclaredDependency],
    imported: &FxHashSet<String>,
    options: &DepsOptions<'_>,
    pkg_mapping: &FxHashMap<&'static str, Vec<&'static str>>,
    stdlib_modules: &FxHashSet<&'static str>,
    lockfile_reachable: Option<&FxHashSet<String>>,
) -> Vec<DeclaredDependency> {
    let mut unused = Vec::new();
    let pyproject_runtime_deps: FxHashSet<&str> = declared
        .iter()
        .filter(|dep| matches!(dep.source, DependencySource::Pyproject))
        .filter(|dep| !dep.is_dev && !dep.is_optional)
        .map(|dep| dep.normalized_name.as_str())
        .collect();
    for dep in declared {
        if options
            .ignore_unused
            .iter()
            .any(|ig| ig == &dep.package_name || ig == &dep.normalized_name)
        {
            continue;
        }
        if dep.is_dev && !options.include_dev_unused {
            continue;
        }
        if stdlib_modules.contains(dep.normalized_name.as_str()) {
            continue;
        }
        if should_treat_requirements_dep_as_export_pin(
            dep,
            options,
            &pyproject_runtime_deps,
            lockfile_reachable,
        ) {
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

        if !expected_imports.iter().any(|e| imported.contains(*e)) {
            unused.push(dep.clone());
        }
    }
    unused
}

fn should_treat_requirements_dep_as_export_pin(
    dep: &DeclaredDependency,
    options: &DepsOptions<'_>,
    pyproject_runtime_deps: &FxHashSet<&str>,
    lockfile_reachable: Option<&FxHashSet<String>>,
) -> bool {
    options.requirements.is_none()
        && !pyproject_runtime_deps.is_empty()
        && !pyproject_runtime_deps.contains(dep.normalized_name.as_str())
        && lockfile_reachable.is_some_and(|reachable| reachable.contains(&dep.normalized_name))
        && matches!(
            &dep.source,
            DependencySource::Requirements(filename) if filename == "requirements.txt"
        )
}

fn package_name_for_import(
    import_name: &str,
    reverse_mapping: &FxHashMap<&'static str, &'static str>,
) -> String {
    let import_lower = import_name.to_lowercase();
    let pkg_name_guess = reverse_mapping
        .get(import_name)
        .or_else(|| reverse_mapping.get(import_lower.as_str()))
        .copied()
        .unwrap_or(import_lower.as_str());
    super::declared::normalize_package_name(pkg_name_guess)
}

fn reachable_lockfile_packages(
    declared: &[DeclaredDependency],
    graph: &super::lockfile::LockfileGraph,
) -> FxHashSet<String> {
    let mut reachable = FxHashSet::default();
    for dep in declared.iter().filter(|dep| !dep.is_dev) {
        reachable.extend(graph.transitive_deps(&dep.normalized_name));
    }
    reachable
}

fn declared_namespace_matches(import_name: &str, declared_names: &FxHashSet<String>) -> bool {
    const KNOWN_NAMESPACE_IMPORTS: &[&str] = &["azure", "google", "zope"];
    KNOWN_NAMESPACE_IMPORTS.contains(&import_name)
        && declared_names.iter().any(|name| {
            name.starts_with(import_name) && name.as_bytes().get(import_name.len()) == Some(&b'_')
        })
}

type OccurrenceIndex<'a> = FxHashMap<&'a str, Vec<&'a ImportOccurrence>>;

fn index_occurrences(occurrences: &[ImportOccurrence]) -> OccurrenceIndex<'_> {
    let mut index = OccurrenceIndex::default();
    for occurrence in occurrences {
        index
            .entry(occurrence.name.as_str())
            .or_default()
            .push(occurrence);
    }
    index
}

fn locations_for_import(
    occurrences: &OccurrenceIndex<'_>,
    import_name: &str,
    production_only: bool,
) -> Vec<DependencyImportLocation> {
    occurrences
        .get(import_name)
        .into_iter()
        .flatten()
        .filter(|occurrence| !production_only || occurrence.is_production)
        .map(|occurrence| DependencyImportLocation::from(*occurrence))
        .collect()
}

fn find_missing_imports(
    imported: &FxHashSet<String>,
    occurrences: &OccurrenceIndex<'_>,
    declared: &[DeclaredDependency],
    options: &DepsOptions<'_>,
    stdlib_modules: &FxHashSet<&'static str>,
    reverse_mapping: &FxHashMap<&'static str, &'static str>,
    lockfile_reachable: Option<&FxHashSet<String>>,
) -> Vec<MissingDependency> {
    // Pre-build a set of all declared names (original and normalized) for O(1) lookup.
    let declared_names: FxHashSet<String> = declared
        .iter()
        .flat_map(|dep| [dep.package_name.to_lowercase(), dep.normalized_name.clone()])
        .collect();

    let mut missing = Vec::new();
    for import_name in imported {
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
        let pkg_normalized = package_name_for_import(import_name, reverse_mapping);
        let is_transitive = lockfile_reachable.is_some_and(|reachable| {
            reachable.contains(&pkg_normalized) && !declared_names.contains(&pkg_normalized)
        });
        if is_transitive {
            continue;
        }

        let is_declared = declared_names.contains(&pkg_normalized)
            || declared_names.contains(&import_lower)
            || declared_namespace_matches(&import_lower, &declared_names);

        if !is_declared {
            missing.push(MissingDependency {
                import_name: import_name.clone(),
                locations: locations_for_import(occurrences, import_name, false),
            });
        }
    }

    missing.sort_by(|a, b| a.import_name.cmp(&b.import_name));
    missing
}

fn find_transitive_imports(
    imported: &FxHashSet<String>,
    occurrences: &OccurrenceIndex<'_>,
    declared: &[DeclaredDependency],
    options: &DepsOptions<'_>,
    stdlib_modules: &FxHashSet<&'static str>,
    reverse_mapping: &FxHashMap<&'static str, &'static str>,
    lockfile_reachable: Option<&FxHashSet<String>>,
) -> Vec<TransitiveDependency> {
    let Some(reachable) = lockfile_reachable else {
        return Vec::new();
    };
    let declared_norm: FxHashSet<String> = declared
        .iter()
        .map(|dep| dep.normalized_name.clone())
        .collect();
    let mut transitive = Vec::new();
    for import_name in imported {
        if options.ignore_missing.iter().any(|ig| ig == import_name) {
            continue;
        }
        if stdlib_modules.contains(import_name.as_str())
            || is_local_package(options.roots, import_name)
        {
            continue;
        }
        let package_name = package_name_for_import(import_name, reverse_mapping);
        if reachable.contains(&package_name) && !declared_norm.contains(&package_name) {
            transitive.push(TransitiveDependency {
                import_name: import_name.clone(),
                package_name,
                locations: locations_for_import(occurrences, import_name, false),
            });
        }
    }
    transitive.sort_by(|a, b| a.import_name.cmp(&b.import_name));
    transitive
}

fn find_stdlib_declarations(
    declared: &[DeclaredDependency],
    stdlib_modules: &FxHashSet<&'static str>,
) -> Vec<DeclaredDependency> {
    declared
        .iter()
        .filter(|dep| dep.marker.is_none() && stdlib_modules.contains(dep.normalized_name.as_str()))
        .cloned()
        .collect()
}

fn find_dev_dependencies_in_production(
    declared: &[DeclaredDependency],
    production_imports: &FxHashSet<String>,
    occurrences: &OccurrenceIndex<'_>,
    options: &DepsOptions<'_>,
    pkg_mapping: &FxHashMap<&'static str, Vec<&'static str>>,
) -> Vec<DevDependencyInProduction> {
    let production_declared: FxHashSet<&str> = declared
        .iter()
        .filter(|dep| !dep.is_dev)
        .map(|dep| dep.normalized_name.as_str())
        .collect();
    let mut findings = Vec::new();
    for dep in declared.iter().filter(|dep| dep.is_dev && !dep.is_optional) {
        if production_declared.contains(dep.normalized_name.as_str()) {
            continue;
        }
        if options
            .ignore_missing
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

        for import_name in expected_imports {
            if production_imports.contains(import_name) {
                findings.push(DevDependencyInProduction {
                    import_name: import_name.to_owned(),
                    dependency: dep.clone(),
                    locations: locations_for_import(occurrences, import_name, true),
                });
            }
        }
    }
    findings.sort_by(|a, b| {
        a.dependency
            .normalized_name
            .cmp(&b.dependency.normalized_name)
            .then_with(|| a.import_name.cmp(&b.import_name))
    });
    findings
}

fn scan_environment(
    options: &DepsOptions<'_>,
    primary_root: &Path,
    declared: &[DeclaredDependency],
    imported: &FxHashSet<String>,
    stdlib_modules: &FxHashSet<&'static str>,
    reverse_mapping: &FxHashMap<&'static str, &'static str>,
) -> (Vec<InstalledPackage>, Vec<InstalledPackage>) {
    let mut extra_installed = Vec::new();
    let mut orphan_installed = Vec::new();

    if !options.show_extra && !options.show_orphans {
        return (extra_installed, orphan_installed);
    }

    let venv_root = options
        .venv_path
        .clone()
        .or_else(|| detect_venv(primary_root));

    let Some(venv) = venv_root else {
        return (extra_installed, orphan_installed);
    };

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
                .map(|s| super::declared::normalize_package_name(s))
                .unwrap_or_else(|| super::declared::normalize_package_name(&i_lower))
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
    (extra_installed, orphan_installed)
}

fn build_removable_branches(
    options: &DepsOptions<'_>,
    primary_root: &Path,
    declared: &[DeclaredDependency],
    unused: &[DeclaredDependency],
) -> Vec<RemovableBranch> {
    let graph = match options.lockfile_path.as_deref() {
        Some(path) => load_lockfile_graph_at(path),
        None => load_lockfile_graph(primary_root),
    };
    let Some(graph) = graph else {
        return Vec::new();
    };

    // Declared normalized names (all, not just unused)
    let all_declared_norm: FxHashSet<String> =
        declared.iter().map(|d| d.normalized_name.clone()).collect();

    let target_unused: Vec<&DeclaredDependency> = if let Some(ref pkg) = options.impact_package {
        let norm = super::declared::normalize_package_name(pkg);
        declared
            .iter()
            .filter(|d| d.normalized_name == norm)
            .collect()
    } else {
        unused.iter().collect()
    };

    let mut branches = Vec::new();
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

        branches.push(RemovableBranch {
            root: dep.package_name.clone(),
            unique_transitive: unique,
        });
    }
    branches
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Analyzes dependencies across the project given the provided options.
pub fn analyze_dependencies(options: &DepsOptions<'_>) -> DepsResult {
    let primary_root = options
        .roots
        .first()
        .map(std::path::PathBuf::as_path)
        .unwrap_or_else(|| Path::new("."));

    let declared = locate_and_parse_declarations(primary_root, options.requirements.as_ref());
    let import_scan = extract_import_scan(options.roots, options.exclude, options.verbose);
    let imported = &import_scan.all;
    let occurrence_index = index_occurrences(&import_scan.occurrences);

    let pkg_mapping = get_package_mapping();
    let stdlib_modules = get_stdlib_modules();
    let reverse_mapping = get_reverse_mapping();
    let lockfile_graph = match options.lockfile_path.as_deref() {
        Some(path) => load_lockfile_graph_at(path),
        None => load_lockfile_graph(primary_root),
    };
    let lockfile_reachable = lockfile_graph
        .as_ref()
        .map(|graph| reachable_lockfile_packages(&declared, graph));

    let unused = find_unused_declared(
        &declared,
        imported,
        options,
        pkg_mapping,
        stdlib_modules,
        lockfile_reachable.as_ref(),
    );
    let missing = find_missing_imports(
        imported,
        &occurrence_index,
        &declared,
        options,
        stdlib_modules,
        reverse_mapping,
        lockfile_reachable.as_ref(),
    );
    let transitive = find_transitive_imports(
        imported,
        &occurrence_index,
        &declared,
        options,
        stdlib_modules,
        reverse_mapping,
        lockfile_reachable.as_ref(),
    );
    let dev_in_production = find_dev_dependencies_in_production(
        &declared,
        &import_scan.production,
        &occurrence_index,
        options,
        pkg_mapping,
    );
    let stdlib = find_stdlib_declarations(&declared, stdlib_modules);
    let (extra_installed, orphan_installed) = scan_environment(
        options,
        primary_root,
        &declared,
        imported,
        stdlib_modules,
        reverse_mapping,
    );
    let removable_branches = build_removable_branches(options, primary_root, &declared, &unused);

    DepsResult {
        unused,
        missing: missing
            .iter()
            .map(|finding| finding.import_name.clone())
            .collect(),
        missing_details: missing,
        extra_installed,
        orphan_installed,
        removable_branches,
        transitive,
        dev_in_production,
        stdlib,
    }
}
