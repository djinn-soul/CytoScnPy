//! Shared utilities for command implementations.

use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Finds all Python files under the given root, excluding specified patterns.
/// Respects .gitignore files in addition to hardcoded defaults.
/// Finds all Python files under the given root, excluding specified patterns.
/// Respects .gitignore files in addition to hardcoded defaults.
pub fn find_python_files(root: &Path, exclude: &[String], verbose: bool) -> Vec<PathBuf> {
    crate::utils::collect_python_files_gitignore(root, exclude, &[], false, verbose).0
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
