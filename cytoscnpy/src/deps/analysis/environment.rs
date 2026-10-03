use super::resolution::ImportResolver;
use super::{DepsOptions, RemovableBranch};
use crate::deps::{
    installed::{detect_venv, scan_installed},
    lockfile::{load_lockfile_graph, load_lockfile_graph_at},
    DeclaredDependency, InstalledPackage,
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::path::Path;

/// Loads the packages installed in the project's virtual environment. The result
/// drives the extra/orphan reports and, more importantly, the import-name
/// mapping, so it is loaded whenever a venv is present.
pub(super) fn load_installed(
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

pub(super) fn scan_environment(
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

pub(super) fn build_removable_branches(
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
        let norm = crate::deps::declared::normalize_package_name(pkg);
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
