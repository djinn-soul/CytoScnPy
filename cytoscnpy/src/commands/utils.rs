//! Shared utilities for command implementations.

use crate::constants::DEFAULT_EXCLUDE_FOLDERS;
use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Finds all Python files under the given root, excluding specified patterns.
pub fn find_python_files(root: &Path, exclude: &[String]) -> Vec<PathBuf> {
    // Merge user excludes with default excludes (.venv, __pycache__, etc.)
    let default_excludes: Vec<String> = DEFAULT_EXCLUDE_FOLDERS()
        .iter()
        .map(|&s| s.to_owned())
        .collect();
    let all_excludes: Vec<String> = exclude.iter().cloned().chain(default_excludes).collect();

    WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            // Prune excluded directories (prevents descent)
            let path = e.path();
            if path.is_dir() {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default();
                return !all_excludes.iter().any(|ex| name.contains(ex));
            }
            true
        })
        .filter_map(std::result::Result::ok)
        .filter(|e| {
            let path = e.path();
            // Only include .py files
            path.is_file() && path.extension().is_some_and(|ext| ext == "py")
        })
        .map(|e| e.path().to_path_buf())
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

/// Loads all Python files from the given paths.
pub fn load_python_files(paths: &[PathBuf], exclude: &[String]) -> Vec<(PathBuf, String)> {
    paths
        .iter()
        .flat_map(|p| find_python_files(p, exclude))
        .filter_map(|p| fs::read_to_string(&p).ok().map(|c| (p, c)))
        .collect()
}
