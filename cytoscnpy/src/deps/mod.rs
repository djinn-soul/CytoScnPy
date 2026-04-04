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
/// Standard library reference list.
pub mod stdlib;

pub use analysis::{analyze_dependencies, DepsOptions, DepsResult, RemovableBranch};
pub use declared::{DeclaredDependency, DependencySource};
pub use installed::InstalledPackage;
pub use lockfile::LockfileGraph;
