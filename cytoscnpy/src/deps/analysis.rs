//! Dependency analysis orchestrates declaration, import, and environment evidence.

mod environment;
mod resolution;
mod types;
mod unused;
mod usage;

use super::declared::scan_declarations;
use super::imports::extract_import_scan;
use super::lockfile::{load_lockfile_graph, load_lockfile_graph_at};
use super::mapping::{get_package_mapping, get_reverse_mapping};
use super::stdlib::get_stdlib_modules;
use environment::{build_removable_branches, load_installed, scan_environment};
use resolution::{custom_reverse_mapping, environment_mapping, ImportResolver};
use rustc_hash::FxHashSet;
use std::path::Path;
pub use types::{
    DependencyImportLocation, DepsOptions, DepsResult, DevDependencyInProduction,
    MissingDependency, RemovableBranch, TransitiveDependency,
};
use unused::{find_unused_declared, reachable_lockfile_packages};
use usage::{
    find_dev_dependencies_in_production, find_missing_imports, find_stdlib_declarations,
    find_transitive_imports, index_occurrences,
};

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

    let declarations = scan_declarations(primary_root, options.requirements.as_ref());
    let declarations_complete = declarations.scan_errors.is_empty();
    let declared = declarations.dependencies;
    let mut scan_errors = declarations.scan_errors;
    let import_scan = extract_import_scan(options.roots, options.exclude, options.verbose);
    scan_errors.extend(import_scan.scan_errors);
    let complete = scan_errors.is_empty();
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
        custom_reverse: custom_reverse_mapping(options.package_mapping),
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

    let mut unused = find_unused_declared(
        &declared,
        &used_at_all,
        options,
        &resolver,
        stdlib_modules,
        lockfile_reachable.as_ref(),
    );
    let mut missing = find_missing_imports(
        imported,
        &occurrence_index,
        &declared,
        options,
        &resolver,
        stdlib_modules,
        lockfile_reachable.as_ref(),
    );
    let mut transitive = find_transitive_imports(
        imported,
        &occurrence_index,
        &declared,
        options,
        &resolver,
        stdlib_modules,
        lockfile_reachable.as_ref(),
    );
    let mut dev_in_production = find_dev_dependencies_in_production(
        &declared,
        &import_scan.production,
        &occurrence_index,
        options,
        &resolver,
    );
    let stdlib = find_stdlib_declarations(&declared, stdlib_modules);
    let (mut extra_installed, mut orphan_installed) = scan_environment(
        options,
        &installed,
        &declared,
        imported,
        stdlib_modules,
        &resolver,
    );
    if !complete {
        // WARNING: Missing source evidence cannot justify removing a dependency.
        unused.clear();
        orphan_installed.clear();
    }
    if !declarations_complete {
        // An unread manifest cannot prove that a package is undeclared or dev-only.
        missing.clear();
        transitive.clear();
        dev_in_production.clear();
        extra_installed.clear();
    }
    let removable_branches = if complete {
        build_removable_branches(options, primary_root, &declared, &unused)
    } else {
        Vec::new()
    };
    scan_errors.sort_by(|a, b| a.file.cmp(&b.file).then_with(|| a.error.cmp(&b.error)));

    DepsResult {
        scan_errors,
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
