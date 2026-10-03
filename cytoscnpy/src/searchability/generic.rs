//! Generic identifier detection across filenames and function names.

use std::path::PathBuf;

use super::functions::FunctionDefinition;
use super::types::{GenericCategory, GenericNameEntry};

/// Generic, low-information terms that reduce codebase searchability.
pub const GENERIC_NAMES: &[&str] = &[
    "utils",
    "helpers",
    "common",
    "misc",
    "data",
    "handler",
    "process",
    "base",
    "core",
    "manager",
    "service",
    "index",
    "main",
    "lib",
    "mod",
    "init",
    "config",
    "constants",
    "types",
];

/// Checks whether an identifier is an exact match for a generic term (case-insensitive).
#[must_use]
pub fn is_generic_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    GENERIC_NAMES.contains(&lower.as_str())
}

/// Identifies generic filenames and function names.
#[must_use]
pub fn find_generic_names(
    files: &[PathBuf],
    functions: &[FunctionDefinition],
) -> (usize, Vec<GenericNameEntry>) {
    let mut entries = Vec::new();

    for file in files {
        let Some(stem) = file.file_stem() else {
            continue;
        };
        let stem_str = stem.to_string_lossy();
        if is_generic_name(&stem_str) {
            entries.push(GenericNameEntry {
                category: GenericCategory::Filename,
                identifier: stem_str.to_string(),
                file: file.clone(),
                line: None,
            });
        }
    }

    for func in functions {
        if is_generic_name(&func.name) {
            entries.push(GenericNameEntry {
                category: GenericCategory::Function,
                identifier: func.name.clone(),
                file: func.file.clone(),
                line: Some(func.line),
            });
        }
    }

    entries.sort();
    let count = entries.len();
    (count, entries)
}
