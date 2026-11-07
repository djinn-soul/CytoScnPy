//! Python source discovery confined to a canonical project boundary.

use rayon::prelude::*;

use super::{effective_excludes, source_walker};

/// Walks `root` for Python sources (and notebooks when `include_ipynb`),
/// returning `(files, directory_count, errors)`. Every source is resolved
/// against `boundary` so symlinks cannot escape it, and duplicate targets are
/// reported once.
pub(crate) fn collect_python_files_within(
    root: &std::path::Path,
    boundary: Option<&std::path::Path>,
    exclude: &[String],
    include: &[String],
    include_ipynb: bool,
    verbose: bool,
) -> (Vec<std::path::PathBuf>, usize, Vec<String>) {
    let all_excludes = effective_excludes(exclude, include);
    let walker = source_walker(root, all_excludes.clone());

    let mut dir_count = 0;
    let boundary = if let Some(boundary) = boundary {
        boundary
    } else if root.is_dir() {
        root
    } else {
        root.parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new("."))
    };
    // Resolve the boundary once; each source is resolved exactly once below,
    // and that resolution also identifies symlinked duplicates.
    let canonical_boundary = boundary
        .canonicalize()
        .map_err(|error| format!("Failed to resolve root {}: {error}", boundary.display()));

    // Walk serially so sources and errors keep the walker's order.
    let mut discovered = Vec::new();
    for result in walker {
        if let Ok(entry) = result {
            let path = entry.path();

            // Count directories (excluded dirs won't appear here due to filter_entry)
            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                if path != root {
                    dir_count += 1;
                }
                continue;
            }

            // Check file extension
            let is_python = path.extension().is_some_and(|ext| ext == "py");
            let is_notebook = include_ipynb && path.extension().is_some_and(|ext| ext == "ipynb");

            if !is_python && !is_notebook {
                continue;
            }

            let rel_path = path.strip_prefix(root).unwrap_or(path);
            if super::super::is_path_ignored(rel_path, &all_excludes) {
                continue;
            }

            if let Err(error) = &canonical_boundary {
                discovered.push(Discovered::Error(error.clone()));
                break;
            }
            discovered.push(Discovered::Source {
                is_symlink: entry.file_type().is_some_and(|kind| kind.is_symlink()),
                path: entry.into_path(),
            });
        } else if let Err(e) = result {
            if verbose {
                eprintln!("Walk error: {e}");
            }
            discovered.push(Discovered::Error(e.to_string()));
        }
    }

    // WARNING: A descendant symlink can escape an otherwise trusted scan
    // root. Validate every source before consumers open it. Resolving each
    // path costs several system calls, so resolve them in parallel.
    let resolutions: Vec<Option<Result<std::path::PathBuf, String>>> = discovered
        .par_iter()
        .map(|item| match (item, &canonical_boundary) {
            (Discovered::Source { path, .. }, Ok(canonical)) => Some(
                super::validation::resolve_within_canonical_root(path, canonical, boundary)
                    .map_err(|error| error.to_string()),
            ),
            _ => None,
        })
        .collect();

    let mut files = Vec::new();
    let mut errors = Vec::new();
    let mut seen_sources = rustc_hash::FxHashSet::default();
    for (item, resolution) in discovered.into_iter().zip(resolutions) {
        let (path, is_symlink) = match item {
            Discovered::Error(error) => {
                errors.push(error);
                continue;
            }
            Discovered::Source { path, is_symlink } => (path, is_symlink),
        };
        let (Some(resolution), Ok(canonical_boundary)) = (resolution, &canonical_boundary) else {
            continue;
        };
        match resolution {
            Ok(resolved) => {
                if !seen_sources.insert(resolved.clone()) {
                    continue;
                }
                // Read the resolved target of symlinks so replacing the
                // link after discovery cannot redirect the source read.
                // Spell it under the caller's root so reports and module
                // names stay project-relative instead of absolute.
                if is_symlink {
                    files.push(match resolved.strip_prefix(canonical_boundary) {
                        Ok(relative) => boundary.join(relative),
                        Err(_) => resolved,
                    });
                } else {
                    files.push(path);
                }
            }
            Err(error) => errors.push(error),
        }
    }

    (files, dir_count, errors)
}

/// A walked source awaiting validation, or a walk error, in walk order.
enum Discovered {
    Source {
        path: std::path::PathBuf,
        is_symlink: bool,
    },
    Error(String),
}
