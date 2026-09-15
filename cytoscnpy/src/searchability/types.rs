//! Data structures and types for codebase searchability analysis.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Overall result of codebase searchability and name collision analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchabilityResult {
    /// High-level aggregate searchability statistics.
    pub stats: SearchabilityStats,
    /// Detailed duplicate filenames (filenames with count > 1).
    pub duplicate_files: Vec<DuplicateFileEntry>,
    /// Detailed colliding functions (functions defined in >= 3 distinct files).
    pub function_collisions: Vec<FunctionCollisionEntry>,
    /// Detailed generic names found in filenames and function identifiers.
    pub generic_names: Vec<GenericNameEntry>,
}

/// Aggregated searchability statistics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchabilityStats {
    /// Total number of unique files scanned.
    pub total_files: usize,
    /// Total number of function definitions analyzed.
    pub total_functions: usize,
    /// Number of filenames that appear in multiple directories.
    pub duplicate_filenames: usize,
    /// The worst duplicate filename and how many times it appears.
    pub worst_duplicate_filename: Option<(String, usize)>,
    /// Number of function names defined in 3 or more distinct files (excluding structural names).
    pub function_name_collisions: usize,
    /// The worst function collision and how many files define it.
    pub worst_function_collision: Option<(String, usize)>,
    /// Total occurrences of generic names in filenames and function names.
    pub generic_name_count: usize,
}

/// A duplicate filename entry and its occurrences across directories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicateFileEntry {
    /// Basename of the file (e.g. `utils.py`).
    pub filename: String,
    /// Number of times this filename occurs.
    pub count: usize,
    /// Paths where this filename occurs.
    pub paths: Vec<PathBuf>,
}

/// A function collision entry and where it is defined.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionCollisionEntry {
    /// Function or method name.
    pub function_name: String,
    /// Number of distinct files defining this function name.
    pub file_count: usize,
    /// Locations where this function is defined: (`file_path`, `line_number`).
    pub locations: Vec<FunctionLocation>,
}

/// Location of a function definition.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FunctionLocation {
    /// Path to the file containing the function.
    pub file: PathBuf,
    /// 1-indexed source line number.
    pub line: usize,
}

/// Category of generic identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GenericCategory {
    /// Filename stem (e.g. `utils.py` -> `utils`).
    Filename,
    /// Function or method name (e.g. `def handle()`).
    Function,
}

/// A generic name occurrence in the codebase.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GenericNameEntry {
    /// Category: Filename or Function.
    pub category: GenericCategory,
    /// The generic identifier (e.g. `utils`, `handler`, `helpers`).
    pub identifier: String,
    /// File where the generic identifier was found.
    pub file: PathBuf,
    /// Source line number if applicable.
    pub line: Option<usize>,
}
