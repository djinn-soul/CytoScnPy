use crate::deps::declared::DeclaredDependency;
use crate::deps::imports::ImportOccurrence;
use crate::deps::InstalledPackage;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    /// Declaration, source, or discovery errors that prevented a complete scan.
    pub scan_errors: Vec<crate::analyzer::types::ParseError>,
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
