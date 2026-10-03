//! Duplicate filename detection across codebase directories.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::types::DuplicateFileEntry;

/// Filenames that are structural/idiomatic to Python packages and expected to repeat.
const EXCLUDED_FILENAMES: &[&str] = &["__init__.py", "__main__.py", "conftest.py"];

/// Analyzes file paths for duplicate basenames across distinct directories.
#[must_use]
pub fn find_duplicate_filenames(
    files: &[PathBuf],
) -> (usize, Option<(String, usize)>, Vec<DuplicateFileEntry>) {
    let mut name_to_paths: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();

    for file in files {
        let Some(name_os) = file.file_name() else {
            continue;
        };
        let name = name_os.to_string_lossy().to_string();

        if is_excluded_filename(&name) {
            continue;
        }

        name_to_paths.entry(name).or_default().push(file.clone());
    }

    let mut duplicate_entries = Vec::new();
    let mut worst: Option<(String, usize)> = None;

    for (filename, mut paths) in name_to_paths {
        paths.sort();
        paths.dedup();
        if paths.len() > 1 {
            let count = paths.len();
            match &worst {
                Some((_, prev_count)) if count > *prev_count => {
                    worst = Some((filename.clone(), count));
                }
                None => {
                    worst = Some((filename.clone(), count));
                }
                _ => {}
            }
            duplicate_entries.push(DuplicateFileEntry {
                filename,
                count,
                paths,
            });
        }
    }

    duplicate_entries.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.filename.cmp(&b.filename))
    });
    let duplicate_count = duplicate_entries.len();

    (duplicate_count, worst, duplicate_entries)
}

/// Checks if a filename should be excluded from duplicate detection.
#[must_use]
pub fn is_excluded_filename(name: &str) -> bool {
    EXCLUDED_FILENAMES.contains(&name)
}

/// Normalizes a path for display relative to a base directory if possible.
#[must_use]
pub fn display_relative(path: &Path, base: Option<&Path>) -> String {
    if let Some(base) = base {
        if let Ok(rel) = path.strip_prefix(base) {
            return rel.to_string_lossy().to_string();
        }
    }
    path.to_string_lossy().to_string()
}
