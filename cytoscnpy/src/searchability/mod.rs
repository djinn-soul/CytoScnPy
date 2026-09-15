//! Codebase searchability, name collisions, and generic identifier analysis.
//!
//! Provides static analysis of codebase searchability and navigation friction:
//! - Duplicate filename detection across directories (excluding package `__init__.py`)
//! - Function and method name collision detection (defined in >= 3 distinct files)
//! - Generic and low-information identifier detection (`utils`, `helpers`, `common`, `handler`, etc.)

pub mod filenames;
pub mod functions;
pub mod generic;
pub mod reporter;
pub mod types;

#[cfg(test)]
mod tests;

pub use filenames::find_duplicate_filenames;
pub use functions::{
    extract_functions_from_file, extract_functions_from_source, find_function_collisions,
    FunctionDefinition,
};
pub use generic::{find_generic_names, is_generic_name, GENERIC_NAMES};
pub use reporter::{print_json_report, print_terminal_report};
pub use types::{
    DuplicateFileEntry, FunctionCollisionEntry, FunctionLocation, GenericCategory,
    GenericNameEntry, SearchabilityResult, SearchabilityStats,
};

use crate::commands::utils::find_python_files;
use std::path::PathBuf;

/// Analyzes codebase searchability, filename duplication, and function name collisions across roots.
#[must_use]
pub fn analyze_searchability(
    roots: &[PathBuf],
    exclude: &[String],
    verbose: bool,
) -> SearchabilityResult {
    let files = find_python_files(roots, exclude, verbose);
    analyze_searchability_files(&files)
}

/// Analyzes searchability over an explicit slice of file paths.
#[must_use]
pub fn analyze_searchability_files(files: &[PathBuf]) -> SearchabilityResult {
    let (duplicate_filenames, worst_duplicate_filename, duplicate_files) =
        filenames::find_duplicate_filenames(files);

    let mut all_functions = Vec::new();
    for file in files {
        let mut file_funcs = functions::extract_functions_from_file(file);
        all_functions.append(&mut file_funcs);
    }

    let total_functions = all_functions.len();
    let (function_name_collisions, worst_function_collision, function_collisions) =
        functions::find_function_collisions(&all_functions);

    let (generic_name_count, generic_names) = generic::find_generic_names(files, &all_functions);

    SearchabilityResult {
        stats: SearchabilityStats {
            total_files: files.len(),
            total_functions,
            duplicate_filenames,
            worst_duplicate_filename,
            function_name_collisions,
            worst_function_collision,
            generic_name_count,
        },
        duplicate_files,
        function_collisions,
        generic_names,
    }
}
