use std::collections::HashSet;
use std::hash::BuildHasher;

use rustc_hash::FxHashSet;

use super::declared::DeclaredDependency;
use super::mapping::{get_package_mapping, get_reverse_mapping};
use super::stdlib::get_stdlib_modules;

/// The outcome of mapping raw import names to distribution/package names.
pub struct UsageMap {
    /// Normalized package names that could be reliably resolved from imports.
    pub resolved: FxHashSet<String>,
    /// Raw import names that could not be mapped to a known distribution.
    pub unresolved: FxHashSet<String>,
}

/// Maps a set of raw import names to distribution names using known mappings,
/// the declared dependency list, and stdlib filtering.
///
/// - Stdlib imports are silently dropped.
/// - Local package names (caller-provided list) are silently dropped.
/// - Known aliases (e.g. `PIL` → `pillow`) are resolved via `mapping.rs`.
/// - Everything else goes into `unresolved`.
pub fn build_usage_map<S: BuildHasher>(
    imported: &HashSet<String, S>,
    declared: &[DeclaredDependency],
    local_packages: &HashSet<String, S>,
) -> UsageMap {
    let stdlib = get_stdlib_modules();
    let reverse = get_reverse_mapping();
    let pkg_map = get_package_mapping();

    // Build a quick lookup: normalized_name → package_name
    let declared_names: FxHashSet<String> =
        declared.iter().map(|d| d.normalized_name.clone()).collect();

    // Also index by all known import aliases from mapping.rs
    let mut alias_to_pkg: rustc_hash::FxHashMap<String, String> = rustc_hash::FxHashMap::default();
    for (pkg, imports) in pkg_map {
        for imp in imports {
            alias_to_pkg.insert((*imp).to_owned(), (*pkg).to_owned());
        }
    }

    let mut resolved = FxHashSet::default();
    let mut unresolved = FxHashSet::default();

    for import_name in imported {
        // Drop stdlib
        if stdlib.contains(import_name.as_str()) {
            continue;
        }
        // Drop local packages
        if local_packages.contains(import_name) {
            continue;
        }

        // Try reverse mapping (e.g. PIL → pillow)
        let pkg_name: String = reverse
            .get(import_name.as_str())
            .map(|s| s.replace('-', "_"))
            .or_else(|| alias_to_pkg.get(import_name).cloned())
            .unwrap_or_else(|| import_name.clone());

        if declared_names.contains(pkg_name.as_str())
            || declared_names.contains(import_name.as_str())
        {
            resolved.insert(pkg_name);
        } else {
            unresolved.insert(import_name.clone());
        }
    }

    UsageMap {
        resolved,
        unresolved,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deps::declared::{DeclaredDependency, DependencySource};

    fn make_dep(name: &str) -> DeclaredDependency {
        DeclaredDependency {
            package_name: name.to_owned(),
            normalized_name: name.to_lowercase().replace('-', "_"),
            is_dev: false,
            source: DependencySource::Pyproject,
        }
    }

    #[test]
    fn test_resolves_known_alias() {
        let imported: FxHashSet<String> = ["PIL".to_owned()].into_iter().collect();
        let declared = vec![make_dep("pillow")];
        let local = FxHashSet::default();
        let map = build_usage_map(&imported, &declared, &local);
        assert!(map.resolved.contains("pillow"));
        assert!(map.unresolved.is_empty());
    }

    #[test]
    fn test_drops_stdlib() {
        let imported: FxHashSet<String> = ["os".to_owned(), "sys".to_owned()].into_iter().collect();
        let map = build_usage_map(&imported, &[], &FxHashSet::default());
        assert!(map.resolved.is_empty());
        assert!(map.unresolved.is_empty());
    }

    #[test]
    fn test_unresolved_unknown_import() {
        let imported: FxHashSet<String> = ["some_mystery_lib".to_owned()].into_iter().collect();
        let map = build_usage_map(&imported, &[], &FxHashSet::default());
        assert!(map.unresolved.contains("some_mystery_lib"));
    }
}
