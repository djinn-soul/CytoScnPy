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

/// Distribution names that publish into a shared namespace package: the import
/// root is the first `_`-separated segment of the distribution name
/// (`google-cloud-storage` → `google`, `ruamel.yaml` → `ruamel`).
const KNOWN_NAMESPACE_IMPORTS: &[&str] = &[
    "azure",
    "backports",
    "google",
    "jaraco",
    "paste",
    "repoze",
    "ruamel",
    "sphinxcontrib",
    "zc",
    "zope",
];

/// True if `dist_normalized` is a distribution published under the namespace
/// package `import_name` (e.g. `google_cloud_storage` under `google`).
fn dist_is_in_namespace(dist_normalized: &str, import_name: &str) -> bool {
    KNOWN_NAMESPACE_IMPORTS.contains(&import_name)
        && dist_normalized.starts_with(import_name)
        && dist_normalized.as_bytes().get(import_name.len()) == Some(&b'_')
}

/// The namespace import root a declared distribution would be imported through,
/// if it belongs to a known namespace package.
fn namespace_root_for_dist(dist_normalized: &str) -> Option<&'static str> {
    KNOWN_NAMESPACE_IMPORTS
        .iter()
        .copied()
        .find(|root| dist_is_in_namespace(dist_normalized, root))
}

/// Directories a first-party module could live in for a given analysis root.
/// Covers the flat layout (`<root>/pkg`) and the src layout (`<root>/src/pkg`).
fn local_search_dirs(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut dirs = Vec::with_capacity(roots.len() * 2);
    for root in roots {
        dirs.push(root.clone());
        let src = root.join("src");
        if src.is_dir() {
            dirs.push(src);
        }
    }
    dirs
}

fn is_local_package(roots: &[PathBuf], module_name: &str) -> bool {
    for root in &local_search_dirs(roots) {
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

// ── Import name resolution ───────────────────────────────────────────────────

/// Maps between distribution names and the import names they provide.
///
/// A distribution's import name frequently differs from its name on `PyPI`
/// (`pyyaml` → `yaml`), and no static table can cover every distribution. So
/// evidence is preferred over guesswork, in order: the `package_mapping` config, then
/// the `top_level.txt` the installer recorded in the virtual environment, then
/// the built-in table of common mismatches, and only then the distribution name
/// itself.
struct ImportResolver<'a> {
    custom: Option<&'a FxHashMap<String, Vec<String>>>,
    environment: &'a EnvironmentMapping,
    builtin: &'static FxHashMap<&'static str, Vec<&'static str>>,
    builtin_reverse: &'static FxHashMap<&'static str, &'static str>,
}

/// Import names derived from the packages installed in the virtual environment.
#[derive(Default)]
struct EnvironmentMapping {
    /// Normalized distribution name → the import names it provides.
    forward: FxHashMap<String, Vec<String>>,
    /// Import name → the normalized distribution name providing it.
    reverse: FxHashMap<String, String>,
}

fn environment_mapping(installed: &FxHashMap<String, InstalledPackage>) -> EnvironmentMapping {
    let mut mapping = EnvironmentMapping::default();
    // Several distributions can share one import name (namespace packages like
    // `google`), and the reverse map keeps the first writer. Iterate in sorted
    // order so the winner does not depend on hash iteration order.
    let mut packages: Vec<(&String, &InstalledPackage)> = installed.iter().collect();
    packages.sort_unstable_by_key(|(normalized, _)| *normalized);
    for (normalized, pkg) in packages {
        if pkg.top_level.is_empty() {
            continue;
        }
        for import_name in &pkg.top_level {
            mapping
                .reverse
                .entry(import_name.clone())
                .or_insert_with(|| normalized.clone());
        }
        mapping
            .forward
            .insert(normalized.clone(), pkg.top_level.clone());
    }
    mapping
}

impl ImportResolver<'_> {
    /// The import names a declared dependency is expected to provide.
    fn imports_for<'b>(&'b self, dep: &'b DeclaredDependency) -> Vec<&'b str> {
        if let Some(names) = self.custom.and_then(|custom| {
            custom
                .get(dep.package_name.as_str())
                .or_else(|| custom.get(dep.normalized_name.as_str()))
        }) {
            return names.iter().map(String::as_str).collect();
        }
        if let Some(names) = self.environment.forward.get(&dep.normalized_name) {
            return names.iter().map(String::as_str).collect();
        }
        if let Some(names) = self
            .builtin
            .get(dep.package_name.as_str())
            .or_else(|| self.builtin.get(dep.normalized_name.as_str()))
        {
            return names.clone();
        }
        vec![dep.normalized_name.as_str()]
    }

    /// The normalized distribution name an import most likely comes from.
    fn distribution_for(&self, import_name: &str) -> String {
        let import_lower = import_name.to_lowercase();
        if let Some(dist) = self
            .environment
            .reverse
            .get(import_name)
            .or_else(|| self.environment.reverse.get(&import_lower))
        {
            return dist.clone();
        }
        let guess = self
            .builtin_reverse
            .get(import_name)
            .or_else(|| self.builtin_reverse.get(import_lower.as_str()))
            .copied()
            .unwrap_or(import_lower.as_str());
        super::declared::normalize_package_name(guess)
    }
}

// ── Step helpers ─────────────────────────────────────────────────────────────

fn find_unused_declared(
    declared: &[DeclaredDependency],
    imported: &FxHashSet<String>,
    options: &DepsOptions<'_>,
    resolver: &ImportResolver<'_>,
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

        let expected_imports = resolver.imports_for(dep);

        // A distribution in a namespace package is imported through its namespace
        // root (`from google.cloud import storage` uses `google-cloud-storage`),
        // so the root counts as usage of every declared dist under it.
        let namespace_used = namespace_root_for_dist(&dep.normalized_name)
            .is_some_and(|root| imported.contains(root));

        if !namespace_used && !expected_imports.iter().any(|e| imported.contains(*e)) {
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
    declared_names
        .iter()
        .any(|name| dist_is_in_namespace(name, import_name))
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
    resolver: &ImportResolver<'_>,
    stdlib_modules: &FxHashSet<&'static str>,
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
        let pkg_normalized = resolver.distribution_for(import_name);
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
    resolver: &ImportResolver<'_>,
    stdlib_modules: &FxHashSet<&'static str>,
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
        let package_name = resolver.distribution_for(import_name);
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
    resolver: &ImportResolver<'_>,
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
        // One dependency can map to several import names (`attrs` → `attr`, `attrs`).
        // Report it once, with the evidence from every import name that was used.
        let used: Vec<&str> = resolver
            .imports_for(dep)
            .into_iter()
            .filter(|name| production_imports.contains(*name))
            .collect();
        let Some(primary) = used.first() else {
            continue;
        };

        findings.push(DevDependencyInProduction {
            import_name: (*primary).to_owned(),
            dependency: dep.clone(),
            locations: used
                .iter()
                .flat_map(|name| locations_for_import(occurrences, name, true))
                .collect(),
        });
    }
    findings.sort_by(|a, b| {
        a.dependency
            .normalized_name
            .cmp(&b.dependency.normalized_name)
            .then_with(|| a.import_name.cmp(&b.import_name))
    });
    findings
}

/// Loads the packages installed in the project's virtual environment. The result
/// drives the extra/orphan reports and, more importantly, the import-name
/// mapping, so it is loaded whenever a venv is present.
fn load_installed(
    options: &DepsOptions<'_>,
    primary_root: &Path,
) -> FxHashMap<String, InstalledPackage> {
    options
        .venv_path
        .clone()
        .or_else(|| detect_venv(primary_root))
        .map(|venv| scan_installed(&venv))
        .unwrap_or_default()
}

fn scan_environment(
    options: &DepsOptions<'_>,
    installed: &FxHashMap<String, InstalledPackage>,
    declared: &[DeclaredDependency],
    imported: &FxHashSet<String>,
    stdlib_modules: &FxHashSet<&'static str>,
    resolver: &ImportResolver<'_>,
) -> (Vec<InstalledPackage>, Vec<InstalledPackage>) {
    let mut extra_installed = Vec::new();
    let mut orphan_installed = Vec::new();

    if !options.show_extra && !options.show_orphans {
        return (extra_installed, orphan_installed);
    }

    // Declared normalized names for fast lookup
    let declared_norm: FxHashSet<String> =
        declared.iter().map(|d| d.normalized_name.clone()).collect();

    // Imported names resolved back to the distributions that provide them.
    let imported_norm: FxHashSet<String> = imported
        .iter()
        .map(|import_name| resolver.distribution_for(import_name))
        .collect();

    for (norm_name, pkg) in installed {
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
    let analysis_root = options
        .roots
        .first()
        .map(std::path::PathBuf::as_path)
        .unwrap_or_else(|| Path::new("."));
    // Manifests, lockfiles and the venv live at the project root, which is not
    // necessarily the directory being analyzed (`cytoscnpy deps src/`).
    let project_root = super::declared::find_project_root(analysis_root);
    let primary_root = project_root.as_path();

    let declared = locate_and_parse_declarations(primary_root, options.requirements.as_ref());
    let import_scan = extract_import_scan(options.roots, options.exclude, options.verbose);
    let imported = &import_scan.all;
    // Imports under `if TYPE_CHECKING:` are not runtime imports, so they never make
    // a dependency "missing", but they do keep a declared dependency from looking unused.
    let used_at_all: FxHashSet<String> = imported
        .iter()
        .chain(import_scan.type_checking.iter())
        .cloned()
        .collect();
    let occurrence_index = index_occurrences(&import_scan.occurrences);

    let stdlib_modules = get_stdlib_modules();
    let installed = load_installed(options, primary_root);
    let environment = environment_mapping(&installed);
    let resolver = ImportResolver {
        custom: options.package_mapping,
        environment: &environment,
        builtin: get_package_mapping(),
        builtin_reverse: get_reverse_mapping(),
    };
    let lockfile_graph = match options.lockfile_path.as_deref() {
        Some(path) => load_lockfile_graph_at(path),
        None => load_lockfile_graph(primary_root),
    };
    let lockfile_reachable = lockfile_graph
        .as_ref()
        .map(|graph| reachable_lockfile_packages(&declared, graph));

    let unused = find_unused_declared(
        &declared,
        &used_at_all,
        options,
        &resolver,
        stdlib_modules,
        lockfile_reachable.as_ref(),
    );
    let missing = find_missing_imports(
        imported,
        &occurrence_index,
        &declared,
        options,
        &resolver,
        stdlib_modules,
        lockfile_reachable.as_ref(),
    );
    let transitive = find_transitive_imports(
        imported,
        &occurrence_index,
        &declared,
        options,
        &resolver,
        stdlib_modules,
        lockfile_reachable.as_ref(),
    );
    let dev_in_production = find_dev_dependencies_in_production(
        &declared,
        &import_scan.production,
        &occurrence_index,
        options,
        &resolver,
    );
    let stdlib = find_stdlib_declarations(&declared, stdlib_modules);
    let (extra_installed, orphan_installed) = scan_environment(
        options,
        &installed,
        &declared,
        imported,
        stdlib_modules,
        &resolver,
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
