//! Analyzer for unreferenced large functions in isolated files.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use rayon::prelude::*;

use super::extractor::{extract_python_file, ExtractedFile};
use super::heuristics::{contains_as_word, should_skip_function, UnreferencedOptions};
use super::types::{
    IsolatedFileSummary, UnreferencedFunction, UnreferencedResult, UnreferencedStats,
};
use crate::commands::utils::find_python_files;

/// Analyzes unreferenced large functions in isolated files across `roots`.
#[must_use]
pub fn analyze_unreferenced(
    roots: &[PathBuf],
    exclude: &[String],
    options: &UnreferencedOptions,
    verbose: bool,
) -> UnreferencedResult {
    let python_files = find_python_files(roots, exclude, verbose);
    let mut result = analyze_unreferenced_files(&python_files, options);
    result.roots = roots.to_vec();
    result
}

/// Analyzes unreferenced large functions across a provided list of Python files.
#[must_use]
pub fn analyze_unreferenced_files(
    files: &[PathBuf],
    options: &UnreferencedOptions,
) -> UnreferencedResult {
    let valid_files: Vec<PathBuf> = files
        .iter()
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e == "py" || e == "pyi")
                && (options.include_tests || !crate::utils::is_test_path(&p.to_string_lossy()))
        })
        .cloned()
        .collect();

    let extracted_list: Vec<(PathBuf, ExtractedFile)> = valid_files
        .par_iter()
        .map(|path| {
            let content = fs::read_to_string(path).unwrap_or_default();
            let extracted = extract_python_file(&content, path);
            (path.clone(), extracted)
        })
        .collect();

    let total_scanned_lines: usize = extracted_list.iter().map(|(_, e)| e.total_lines).sum();
    let extracted_files: HashMap<PathBuf, ExtractedFile> = extracted_list.into_iter().collect();

    let connected_files = find_connected_files(&valid_files, &extracted_files);

    let mut unreferenced_items = Vec::new();
    let mut isolated_files = Vec::new();
    let mut total_unreferenced_lines = 0usize;

    for path in &valid_files {
        if connected_files.contains(path) {
            continue;
        }

        let Some(extracted) = extracted_files.get(path) else {
            continue;
        };

        let mut file_unref_count = 0usize;
        let mut file_unref_lines = 0usize;

        for func in &extracted.functions {
            if should_skip_function(&func.name, func.line_count, options) {
                continue;
            }

            // Check if function name appears anywhere else in other files
            let is_referenced = valid_files.iter().any(|other_path| {
                if other_path == path {
                    return false;
                }
                let Some(other_file) = extracted_files.get(other_path) else {
                    return false;
                };

                // Check whole-word presence in content
                contains_as_word(&other_file.content, &func.name)
                    || other_file.imports.iter().any(|imp| imp == &func.name)
            });

            if !is_referenced {
                file_unref_count += 1;
                file_unref_lines += func.line_count;
                total_unreferenced_lines += func.line_count;

                unreferenced_items.push(UnreferencedFunction {
                    file: path.clone(),
                    name: func.name.clone(),
                    start_line: func.start_line,
                    end_line: func.end_line,
                    line_count: func.line_count,
                    node_kind: func.node_kind.clone(),
                    class_name: func.class_name.clone(),
                });
            }
        }

        if file_unref_count > 0 {
            isolated_files.push(IsolatedFileSummary {
                file: path.clone(),
                total_lines: extracted.total_lines,
                unreferenced_functions: file_unref_count,
                unreferenced_lines: file_unref_lines,
            });
        }
    }

    // Sort deterministically: by unreferenced lines descending
    unreferenced_items.sort_by_key(|f| std::cmp::Reverse(f.line_count));
    isolated_files.sort_by_key(|f| std::cmp::Reverse(f.unreferenced_lines));

    let stats = UnreferencedStats {
        total_unreferenced_functions: unreferenced_items.len(),
        total_unreferenced_lines,
        isolated_files_count: isolated_files.len(),
        total_scanned_files: valid_files.len(),
        total_scanned_lines,
    };

    UnreferencedResult {
        items: unreferenced_items,
        isolated_files,
        stats,
        roots: Vec::new(),
    }
}

/// Identifies files that have incoming references (via imports or word occurrences) from other files.
fn find_connected_files(
    valid_files: &[PathBuf],
    extracted_files: &HashMap<PathBuf, ExtractedFile>,
) -> HashSet<PathBuf> {
    // Map each file to its stem
    let file_stems: Vec<(&PathBuf, String)> = valid_files
        .iter()
        .filter_map(|p| {
            let stem = p.file_stem()?.to_string_lossy().to_string();
            Some((p, stem))
        })
        .collect();

    file_stems
        .par_iter()
        .filter_map(|(target_path, stem)| {
            // Special case: package __init__.py files are entry points, always connected
            if stem == "__init__" {
                return Some((*target_path).clone());
            }

            let is_incoming = valid_files.iter().any(|source_path| {
                if source_path == *target_path {
                    return false;
                }
                let Some(source_file) = extracted_files.get(source_path) else {
                    return false;
                };

                // Check if source imports mention target stem
                let imported = source_file.imports.iter().any(|imp| {
                    imp == stem
                        || imp.ends_with(&format!(".{stem}"))
                        || imp.starts_with(&format!("{stem}."))
                        || imp.contains(&format!(".{stem}."))
                });
                if imported {
                    return true;
                }

                // If stem is 4+ chars, check whole-word content occurrence
                stem.len() >= 4 && contains_as_word(&source_file.content, stem)
            });

            if is_incoming {
                Some((*target_path).clone())
            } else {
                None
            }
        })
        .collect()
}
