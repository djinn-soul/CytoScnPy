use super::model::{FileMetrics, ProjectStats};
use crate::commands::metric_source::{parse_source, read_source};
use crate::raw_metrics::analyze_raw;
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

fn count_functions_and_classes(code: &str, file_path: &Path) -> Result<(usize, usize)> {
    use ruff_python_ast::Stmt;
    let m = parse_source(code, file_path)?;
    {
        let mut functions = 0;
        let mut classes = 0;
        for stmt in &m.body {
            match stmt {
                Stmt::FunctionDef(_) => functions += 1,
                Stmt::ClassDef(c) => {
                    classes += 1;
                    for item in &c.body {
                        if matches!(item, Stmt::FunctionDef(_)) {
                            functions += 1;
                        }
                    }
                }
                _ => {}
            }
        }
        Ok((functions, classes))
    }
}

#[allow(clippy::cast_precision_loss)]
pub(super) fn collect_project_stats(
    roots: &[PathBuf],
    exclude: &[String],
    include_folders: &[String],
    include_tests: bool,
    verbose: bool,
) -> Result<ProjectStats> {
    let mut files = Vec::new();
    let mut num_directories = 0;
    for path in roots {
        let (mut root_files, d, errors) = crate::utils::collect_python_files_gitignore_with_errors(
            path,
            exclude,
            include_folders,
            false,
            verbose,
        );
        anyhow::ensure!(
            errors.is_empty(),
            "Stats file discovery failed for {}: {}",
            path.display(),
            errors.join("; ")
        );
        if !include_tests {
            root_files.retain(|file| !crate::utils::is_test_path_relative_to(file, path));
        }
        files.extend(root_files);
        num_directories += d;
    }

    let collected: Vec<(FileMetrics, usize, usize)> = files
        .par_iter()
        .map(|file_path| {
            let code = read_source(file_path)?;
            let metrics = analyze_raw(&code);
            let size_bytes = fs::metadata(file_path)
                .with_context(|| format!("Failed to inspect {}", file_path.display()))?
                .len();
            let (functions, classes) = count_functions_and_classes(&code, file_path)?;
            Ok((
                FileMetrics {
                    file: file_path.to_string_lossy().to_string(),
                    code_lines: metrics.sloc,
                    comment_lines: metrics.comments,
                    empty_lines: metrics.blank,
                    total_lines: metrics.loc,
                    size_kb: size_bytes as f64 / 1024.0,
                },
                functions,
                classes,
            ))
        })
        .collect::<Result<Vec<_>>>()?;

    let total_functions = collected.iter().map(|(_, functions, _)| functions).sum();
    let total_classes = collected.iter().map(|(_, _, classes)| classes).sum();
    let file_metrics: Vec<_> = collected
        .into_iter()
        .map(|(metrics, _, _)| metrics)
        .collect();

    let total_files = file_metrics.len();
    let total_size_kb: f64 = file_metrics.iter().map(|f| f.size_kb).sum();
    let total_lines: usize = file_metrics.iter().map(|f| f.total_lines).sum();
    let code_lines: usize = file_metrics.iter().map(|f| f.code_lines).sum();
    let comment_lines: usize = file_metrics.iter().map(|f| f.comment_lines).sum();
    let empty_lines: usize = file_metrics.iter().map(|f| f.empty_lines).sum();

    Ok(ProjectStats {
        total_files,
        total_directories: num_directories,
        total_size_kb,
        total_lines,
        code_lines,
        comment_lines,
        empty_lines,
        total_functions,
        total_classes,
        file_metrics,
    })
}
