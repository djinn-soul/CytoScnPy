use rustc_hash::FxHashSet;
use std::path::{Path, PathBuf};

use super::declared::{locate_and_parse_declarations, DeclaredDependency};
use super::imports::extract_imports;
use super::mapping::{get_import_names, get_package_name};
use super::stdlib::is_stdlib_module;

/// The result of the dependency analysis.
pub struct DepsResult {
    /// List of dependencies that are declared but not imported in the code.
    pub unused: Vec<DeclaredDependency>,
    /// List of import names that are found in the code but not declared in dependencies.
    pub missing: Vec<String>,
}

/// Configuration options for the dependency analysis.
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
}

fn is_local_package(roots: &[PathBuf], module_name: &str) -> bool {
    for root in roots {
        let dir = root.join(module_name);
        if dir.is_dir() && (dir.join("__init__.py").exists() || dir.join("__init__.pyi").exists()) {
            return true;
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

    let declared = locate_and_parse_declarations(primary_root, options.requirements.as_ref());
    let imported = extract_imports(options.roots, options.exclude, options.verbose);

    let mut unused = Vec::new();
    for dep in &declared {
        if options
            .ignore_unused
            .iter()
            .any(|ig| ig == &dep.package_name || ig == &dep.normalized_name)
        {
            continue;
        }

        let expected_imports = match get_import_names(&dep.package_name)
            .or_else(|| get_import_names(&dep.normalized_name))
        {
            Some(names) => names.to_vec(),
            None => vec![dep.normalized_name.as_str()],
        };

        let mut is_used = false;
        for expected in expected_imports {
            if imported.contains(expected) {
                is_used = true;
                break;
            }
        }

        if !is_used {
            unused.push(dep.clone());
        }
    }

    let mut missing_set = FxHashSet::default();

    for import_name in &imported {
        if options.ignore_missing.iter().any(|ig| ig == import_name) {
            continue;
        }

        if is_stdlib_module(import_name) {
            continue;
        }

        if is_local_package(options.roots, import_name) {
            continue;
        }

        let pkg_name_guess = get_package_name(import_name).unwrap_or(import_name);
        // Sometimes a package with dashes is matched locally with underscores, test both
        let pkg_normalized = pkg_name_guess.replace('-', "_");

        let mut is_declared = false;
        for dep in &declared {
            if dep.package_name == pkg_name_guess
                || dep.normalized_name == pkg_name_guess
                || dep.normalized_name == pkg_normalized
                || dep.normalized_name == *import_name
            {
                is_declared = true;
                break;
            }
        }

        if !is_declared {
            missing_set.insert(import_name.clone());
        }
    }

    let mut missing: Vec<String> = missing_set.into_iter().collect();
    missing.sort();

    DepsResult { unused, missing }
}
