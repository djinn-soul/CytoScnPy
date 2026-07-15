/// Core dependency analysis logic.
pub mod analysis;
/// Parsers for pyproject.toml and requirements.txt files.
pub mod declared;
/// AST extraction for Python import statements.
pub mod imports;
/// Installed environment package scanner.
pub mod installed;
/// Lockfile parser (uv.lock / poetry.lock) for dependency graph.
pub mod lockfile;
/// Package-to-import mapping definitions.
pub mod mapping;
/// Parsers for the setuptools declaration files (setup.py / setup.cfg).
pub mod setup;
/// Standard library reference list.
pub mod stdlib;

#[cfg(test)]
#[path = "analysis_tests.rs"]
mod analysis_tests;

pub use analysis::{
    analyze_dependencies, DependencyImportLocation, DepsOptions, DepsResult,
    DevDependencyInProduction, MissingDependency, RemovableBranch, TransitiveDependency,
};
pub use declared::{DeclaredDependency, DependencySource};
pub use installed::InstalledPackage;
pub use lockfile::LockfileGraph;
