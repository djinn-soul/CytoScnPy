//! Shared utilities for command implementations.

use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Finds all Python files under the given root, excluding specified patterns.
/// Respects .gitignore files in addition to hardcoded defaults.
pub fn find_python_files(root: &Path, exclude: &[String], verbose: bool) -> Vec<PathBuf> {
    crate::utils::collect_python_files_gitignore(root, exclude, &[], false, verbose).0
}

/// Merges primary excludes with additional ignore patterns into a single list.
/// This is a common pattern used by all subcommands.
pub fn merge_excludes(primary: Vec<String>, additional: Vec<String>) -> Vec<String> {
    let mut merged = primary;
    merged.extend(additional);
    merged
}

/// Trait for items that can be filtered by rank (A-F).
pub trait HasRank {
    fn rank(&self) -> char;
}

/// Filters a list of items by minimum and/or maximum rank.
/// Rank ordering: A < B < C < D < E < F (A is best, F is worst).
pub fn filter_by_rank<T: HasRank>(
    items: Vec<T>,
    min_rank: Option<char>,
    max_rank: Option<char>,
) -> Vec<T> {
    items
        .into_iter()
        .filter(|item| {
            let rank = item.rank();
            let passes_min = min_rank.map_or(true, |min| rank >= min);
            let passes_max = max_rank.map_or(true, |max| rank <= max);
            passes_min && passes_max
        })
        .collect()
}

/// Writes output to either a file or a writer.
pub fn write_output<W: Write>(
    writer: &mut W,
    content: &str,
    output_file: Option<String>,
) -> Result<()> {
    if let Some(path) = output_file {
        let mut file = fs::File::create(path)?;
        writeln!(file, "{content}")?;
    } else {
        writeln!(writer, "{content}")?;
    }
    Ok(())
}
