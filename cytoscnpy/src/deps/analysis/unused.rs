use super::{
    resolution::{dist_is_in_namespace, namespace_root_for_dist, ImportResolver},
    DepsOptions,
};
use crate::deps::{DeclaredDependency, DependencySource};
use rustc_hash::FxHashSet;

pub(super) fn find_unused_declared(
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

pub(super) fn should_treat_requirements_dep_as_export_pin(
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

pub(super) fn reachable_lockfile_packages(
    declared: &[DeclaredDependency],
    graph: &crate::deps::lockfile::LockfileGraph,
) -> FxHashSet<String> {
    let mut reachable = FxHashSet::default();
    for dep in declared.iter().filter(|dep| !dep.is_dev) {
        reachable.extend(graph.transitive_deps(&dep.normalized_name));
    }
    reachable
}

pub(super) fn declared_namespace_matches(
    import_name: &str,
    declared_names: &FxHashSet<String>,
) -> bool {
    declared_names
        .iter()
        .any(|name| dist_is_in_namespace(name, import_name))
}
