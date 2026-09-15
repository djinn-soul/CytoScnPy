//! Data types for mutable global state analysis.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Category of detected mutable global state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GlobalKind {
    /// Python module-level mutable collection (list, dict, set, etc.).
    ModuleCollection,
    /// Python class-level mutable attribute without explicit type declaration.
    ClassVariable,
    /// Python function modifying a module variable via `global`.
    GlobalMutation,
    /// Rust `static mut` declaration.
    RustStaticMut,
    /// JavaScript/TypeScript top-level mutable `var` or `let` collection.
    JsTopLevelMutable,
}

impl GlobalKind {
    /// Short human-readable code or label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ModuleCollection => "module_collection",
            Self::ClassVariable => "class_variable",
            Self::GlobalMutation => "global_mutation",
            Self::RustStaticMut => "static_mut",
            Self::JsTopLevelMutable => "js_toplevel_var",
        }
    }

    /// Descriptive explanation of the anti-pattern.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::ModuleCollection => "Module-level mutable collection (list, dict, set)",
            Self::ClassVariable => "Class-level mutable attribute (shared across instances)",
            Self::GlobalMutation => "Function-level mutation of module global via `global`",
            Self::RustStaticMut => "Rust `static mut` declaration",
            Self::JsTopLevelMutable => "JavaScript top-level mutable variable",
        }
    }
}

impl std::fmt::Display for GlobalKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A single detected mutable global instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalMatch {
    /// Relative or canonical path to the file containing the global.
    pub file: PathBuf,
    /// Line number (1-indexed).
    pub line: usize,
    /// Column number (1-indexed).
    pub column: usize,
    /// Variable or identifier name.
    pub name: String,
    /// The specific category of mutable global.
    pub kind: GlobalKind,
    /// Code snippet or line content.
    pub snippet: String,
}

/// Aggregate summary statistics for mutable global state.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalStats {
    /// Total number of mutable global instances detected.
    pub total_globals: usize,
    /// Module-level mutable collections (Python).
    pub module_collection_count: usize,
    /// Class-level mutable attributes (Python).
    pub class_variable_count: usize,
    /// Global statement mutations (Python).
    pub global_mutation_count: usize,
    /// Rust `static mut` declarations.
    pub rust_static_mut_count: usize,
    /// JavaScript/TypeScript top-level mutable variables.
    pub js_toplevel_count: usize,
    /// Number of distinct files containing at least one mutable global.
    pub affected_files: usize,
}

/// Result of scanning a codebase for mutable global state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalsResult {
    /// The target root directory that was scanned.
    pub target: PathBuf,
    /// Total number of source files scanned.
    pub files_scanned: usize,
    /// All detected mutable global instances.
    pub matches: Vec<GlobalMatch>,
    /// Aggregate statistics.
    pub stats: GlobalStats,
}

impl GlobalsResult {
    /// Creates a new, empty result for the given target path.
    #[must_use]
    pub fn new(target: PathBuf) -> Self {
        Self {
            target,
            files_scanned: 0,
            matches: Vec::new(),
            stats: GlobalStats::default(),
        }
    }

    /// Returns `true` if no mutable global patterns were detected.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.matches.is_empty()
    }
}
