//! Data structures for module architecture and dependency graph analysis.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Identification and metadata for a Python module node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleNode {
    /// Numerical node identifier in the graph.
    pub id: usize,
    /// Canonical dotted module name (e.g., `cytoscnpy.architecture.types`).
    pub name: String,
    /// Absolute or relative file path to the source file.
    pub file_path: PathBuf,
    /// Normalized display path relative to the analysis root.
    pub relative_path: String,
    /// Total lines in the file.
    pub line_count: usize,
    /// Whether this file is a package `__init__.py`.
    pub is_init: bool,
    /// Top-level package group name (e.g., `cytoscnpy`).
    pub package_group: String,
}

/// Single occurrence of an import between two modules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportOccurrence {
    /// 1-indexed source line where the import occurs.
    pub line: usize,
    /// 1-indexed source column where the import occurs.
    pub column: usize,
    /// Whether the import was inside an `if TYPE_CHECKING:` block.
    pub is_type_checking: bool,
    /// Whether the import was at module level (not inside a function or class).
    pub is_top_level: bool,
    /// The exact imported symbol or submodule if `from ... import ...` was used.
    pub imported_symbol: Option<String>,
}

/// A directed edge in the module graph from one module to another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleEdge {
    /// Identifier of the importing module.
    pub from_id: usize,
    /// Identifier of the imported module.
    pub to_id: usize,
    /// Canonical name of the importing module.
    pub from_name: String,
    /// Canonical name of the imported module.
    pub to_name: String,
    /// All import occurrences representing this dependency edge.
    pub occurrences: Vec<ImportOccurrence>,
}

/// Information about a detected circular dependency cycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CycleInfo {
    /// Ordered sequence of canonical module names forming the cycle (e.g. `[A, B, C, A]`).
    pub modules: Vec<String>,
    /// File paths corresponding to each module in the cycle.
    pub file_paths: Vec<PathBuf>,
    /// Whether this cycle contains top-level runtime imports that could fail on startup.
    pub has_runtime_impact: bool,
    /// Human-readable step-by-step trace of the cycle.
    pub steps: Vec<String>,
}

/// Information about an identified God Module (coupling hotspot).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GodModuleInfo {
    /// Dotted module name.
    pub module_name: String,
    /// File path to the module.
    pub file_path: PathBuf,
    /// Number of project modules importing this module.
    pub fan_in: usize,
    /// Percentage of all project modules that import this module.
    pub fan_in_percentage: f64,
    /// Number of project modules this module imports.
    pub fan_out: usize,
    /// Heuristic rationale for flagging this module.
    pub reason: String,
}

/// Information about bidirectional dependency between two high-level module groups/packages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BidirectionalGroupDep {
    /// First package/group name.
    pub group_a: String,
    /// Second package/group name.
    pub group_b: String,
    /// Count of imports from group A into group B.
    pub edges_a_to_b: usize,
    /// Count of imports from group B into group A.
    pub edges_b_to_a: usize,
    /// Sample module in A importing B.
    pub sample_a_to_b: String,
    /// Sample module in B importing A.
    pub sample_b_to_a: String,
}

/// Aggregate architecture metrics for the module graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ArchitectureGraphStats {
    /// Total number of project modules analyzed.
    pub total_modules: usize,
    /// Total number of directed module-to-module dependencies.
    pub total_internal_edges: usize,
    /// Total occurrences of external/third-party package imports.
    pub total_external_imports: usize,
    /// Average fan-in across all modules.
    pub avg_fan_in: f64,
    /// Maximum fan-in of any module.
    pub max_fan_in: usize,
    /// Module with the highest fan-in.
    pub max_fan_in_module: Option<String>,
    /// Average fan-out across all modules.
    pub avg_fan_out: f64,
    /// Maximum fan-out of any module.
    pub max_fan_out: usize,
    /// Module with the highest fan-out.
    pub max_fan_out_module: Option<String>,
    /// Total number of circular dependency components detected.
    pub circular_dependency_count: usize,
    /// Number of modules in the largest circular dependency cycle.
    pub largest_cycle_size: usize,
    /// List of detected cycles.
    pub cycles: Vec<CycleInfo>,
    /// Identified god modules.
    pub god_modules: Vec<GodModuleInfo>,
    /// Identified bidirectional package group dependencies.
    pub bidirectional_group_deps: Vec<BidirectionalGroupDep>,
}

/// Complete result containing graph elements, statistics, and external dependencies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArchitectureGraphResult {
    /// Summary statistics and identified architectural hazards.
    pub stats: ArchitectureGraphStats,
    /// Module nodes in the graph.
    pub nodes: Vec<ModuleNode>,
    /// Directed edges between module nodes.
    pub edges: Vec<ModuleEdge>,
    /// Frequency breakdown of external package imports (e.g. `{"requests": 5, "pydantic": 12}`).
    pub external_imports: HashMap<String, usize>,
    /// Per-module fan-in counts (`module_name` -> `fan_in`).
    pub fan_in_map: HashMap<String, usize>,
    /// Per-module fan-out counts (`module_name` -> `fan_out`).
    pub fan_out_map: HashMap<String, usize>,
}
