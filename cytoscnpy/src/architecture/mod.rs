//! Module architecture, dependency graph, and circular import analysis.
//!
//! Provides static analysis of Python project structure:
//! - Dotted module identity resolution
//! - Internal and external import graph construction
//! - Fan-in and Fan-out coupling metrics
//! - Circular dependency detection (SCC via Tarjan's algorithm)
//! - God module and bidirectional package dependency detection

pub mod builder;
pub mod collector;
pub mod collector_type_checking;
pub mod cycles;
pub mod layering;
pub mod reporter;
pub mod resolver;
pub mod types;

#[cfg(test)]
mod tests;

pub use builder::build_architecture_graph;
pub use reporter::{print_json_report, print_terminal_report};
pub use types::{
    ArchitectureGraphResult, ArchitectureGraphStats, BidirectionalGroupDep, CycleInfo,
    GodModuleInfo, ImportOccurrence, ModuleEdge, ModuleNode,
};

use crate::commands::utils::find_python_files;
use std::path::PathBuf;

/// Analyzes Python codebase architecture, building the module graph and detecting cycles.
#[must_use]
pub fn analyze_architecture(
    roots: &[PathBuf],
    exclude: &[String],
    verbose: bool,
) -> ArchitectureGraphResult {
    let files = find_python_files(roots, exclude, verbose);
    build_architecture_graph(&files, roots)
}
