//! Deduplicate physical source files and overlapping directory inputs.

use std::collections::HashSet;
use std::path::PathBuf;

fn identity(path: &PathBuf) -> PathBuf {
    // Discovery/read paths report resolution failures separately. Retain the
    // original spelling here so failed inputs are still diagnosed.
    path.canonicalize().unwrap_or_else(|_| path.clone())
}

/// An analysis input after removing repeated spellings of the same path.
pub(crate) struct Input<'a> {
    pub(crate) path: &'a PathBuf,
    /// The outermost other directory input containing this directory. Nested
    /// inputs are still walked, so explicit inputs bypass the parent's
    /// exclude/ignore filters like explicit files do, but their directories
    /// were already counted and their symlinks may resolve anywhere inside it.
    pub(crate) enclosing: Option<&'a PathBuf>,
}

pub(crate) fn unique_inputs(paths: &[PathBuf]) -> Vec<Input<'_>> {
    let mut seen = HashSet::new();
    let unique: Vec<_> = paths
        .iter()
        .filter_map(|path| {
            let key = identity(path);
            seen.insert(key.clone()).then_some((path, key))
        })
        .collect();
    unique
        .iter()
        .map(|(path, key)| Input {
            path,
            enclosing: unique
                .iter()
                .filter(|(other, other_key)| {
                    path.is_dir()
                        && other.is_dir()
                        && key != other_key
                        && key.starts_with(other_key)
                })
                .min_by_key(|(_, other_key)| other_key.components().count())
                .map(|(other, _)| *other),
        })
        .collect()
}

pub(crate) fn deduplicate_files(files: &mut Vec<PathBuf>) {
    let mut seen = HashSet::new();
    files.retain(|path| seen.insert(identity(path)));
}
