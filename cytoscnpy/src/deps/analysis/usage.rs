use super::resolution::{is_local_package, ImportResolver};
use super::unused::declared_namespace_matches;
use super::{
    DependencyImportLocation, DepsOptions, DevDependencyInProduction, MissingDependency,
    TransitiveDependency,
};
use crate::deps::{imports::ImportOccurrence, DeclaredDependency};
use rustc_hash::{FxHashMap, FxHashSet};

pub(super) type OccurrenceIndex<'a> = FxHashMap<&'a str, Vec<&'a ImportOccurrence>>;

pub(super) fn index_occurrences(occurrences: &[ImportOccurrence]) -> OccurrenceIndex<'_> {
    let mut index = OccurrenceIndex::default();
    for occurrence in occurrences {
        index
            .entry(occurrence.name.as_str())
            .or_default()
            .push(occurrence);
    }
    index
}

pub(super) fn locations_for_import(
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

pub(super) fn find_missing_imports(
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

pub(super) fn find_transitive_imports(
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
        // A namespace root (`google`) can resolve to whichever provider the venv
        // recorded, so it may name a reachable-but-undeclared distribution even
        // though a declared one publishes into that same namespace.
        if declared_namespace_matches(&import_name.to_lowercase(), &declared_norm) {
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

pub(super) fn find_stdlib_declarations(
    declared: &[DeclaredDependency],
    stdlib_modules: &FxHashSet<&'static str>,
) -> Vec<DeclaredDependency> {
    declared
        .iter()
        .filter(|dep| dep.marker.is_none() && stdlib_modules.contains(dep.normalized_name.as_str()))
        .cloned()
        .collect()
}

pub(super) fn find_dev_dependencies_in_production(
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
