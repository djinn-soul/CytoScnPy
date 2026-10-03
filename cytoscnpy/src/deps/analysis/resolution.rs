use crate::deps::{DeclaredDependency, InstalledPackage};
use rustc_hash::FxHashMap;
use std::path::PathBuf;

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
pub(super) fn dist_is_in_namespace(dist_normalized: &str, import_name: &str) -> bool {
    KNOWN_NAMESPACE_IMPORTS.contains(&import_name)
        && dist_normalized.starts_with(import_name)
        && dist_normalized.as_bytes().get(import_name.len()) == Some(&b'_')
}

/// The namespace import root a declared distribution would be imported through,
/// if it belongs to a known namespace package.
pub(super) fn namespace_root_for_dist(dist_normalized: &str) -> Option<&'static str> {
    KNOWN_NAMESPACE_IMPORTS
        .iter()
        .copied()
        .find(|root| dist_is_in_namespace(dist_normalized, root))
}

/// Directories a first-party module could live in for a given analysis root.
/// Covers the flat layout (`<root>/pkg`) and the src layout (`<root>/src/pkg`).
pub(super) fn local_search_dirs(roots: &[PathBuf]) -> Vec<PathBuf> {
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

pub(super) fn is_local_package(roots: &[PathBuf], module_name: &str) -> bool {
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
pub(super) struct ImportResolver<'a> {
    pub(super) custom: Option<&'a FxHashMap<String, Vec<String>>>,
    /// Import name → normalized distribution name, derived from `custom`.
    pub(super) custom_reverse: FxHashMap<String, String>,
    pub(super) environment: &'a EnvironmentMapping,
    pub(super) builtin: &'static FxHashMap<&'static str, Vec<&'static str>>,
    pub(super) builtin_reverse: &'static FxHashMap<&'static str, &'static str>,
}

/// Inverts the `package_mapping` config so an import can be traced back to the
/// distribution the user mapped it to. Without this, a configured mapping makes
/// the declared dependency look used but still leaves the import looking
/// undeclared. Entries are visited in sorted order so that two distributions
/// claiming one import name resolve to the same winner on every run.
pub(super) fn custom_reverse_mapping(
    custom: Option<&FxHashMap<String, Vec<String>>>,
) -> FxHashMap<String, String> {
    let mut reverse = FxHashMap::default();
    let Some(custom) = custom else {
        return reverse;
    };
    let mut entries: Vec<(&String, &Vec<String>)> = custom.iter().collect();
    entries.sort_unstable_by_key(|(dist, _)| *dist);
    for (dist, import_names) in entries {
        let normalized = crate::deps::declared::normalize_package_name(dist);
        for import_name in import_names {
            reverse
                .entry(import_name.clone())
                .or_insert_with(|| normalized.clone());
        }
    }
    reverse
}

/// Import names derived from the packages installed in the virtual environment.
#[derive(Default)]
pub(super) struct EnvironmentMapping {
    /// Normalized distribution name → the import names it provides.
    forward: FxHashMap<String, Vec<String>>,
    /// Import name → the normalized distribution name providing it.
    reverse: FxHashMap<String, String>,
}

pub(super) fn environment_mapping(
    installed: &FxHashMap<String, InstalledPackage>,
) -> EnvironmentMapping {
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
    pub(super) fn imports_for<'b>(&'b self, dep: &'b DeclaredDependency) -> Vec<&'b str> {
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
    pub(super) fn distribution_for(&self, import_name: &str) -> String {
        let import_lower = import_name.to_lowercase();
        if let Some(dist) = self
            .custom_reverse
            .get(import_name)
            .or_else(|| self.custom_reverse.get(&import_lower))
        {
            return dist.clone();
        }
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
        crate::deps::declared::normalize_package_name(guess)
    }
}

// ── Step helpers ─────────────────────────────────────────────────────────────
