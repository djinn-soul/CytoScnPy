/// Core dependency analysis logic.
pub mod analysis;
/// Parsers for pyproject.toml and requirements.txt files.
pub mod declared;
/// AST extraction for Python import statements.
pub mod imports;
/// Package-to-import mapping definitions.
pub mod mapping;
/// Standard library reference list.
pub mod stdlib;

pub use analysis::{analyze_dependencies, DepsOptions, DepsResult};
pub use declared::{DeclaredDependency, DependencySource};
