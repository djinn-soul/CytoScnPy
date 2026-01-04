//! Stats and files commands.

use super::utils::find_python_files;
use crate::analyzer::CytoScnPy;
use crate::config::Config;
use crate::raw_metrics::analyze_raw;

use anyhow::Result;
use colored::Colorize;
use comfy_table::Table;
use rayon::prelude::*;
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone)]
struct FileMetrics {
    file: String,
    code_lines: usize,
    comment_lines: usize,
    empty_lines: usize,
    total_lines: usize,
    size_kb: f64,
}

#[derive(Serialize)]
struct StatsReport {
    total_files: usize,
    total_directories: usize,
    total_size_kb: f64,
    total_lines: usize,
    code_lines: usize,
    comment_lines: usize,
    empty_lines: usize,
    total_functions: usize,
    total_classes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    files: Option<Vec<FileMetrics>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    secrets: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    danger: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quality: Option<Vec<String>>,
}

fn count_functions_and_classes(code: &str, _file_path: &Path) -> (usize, usize) {
    use ruff_python_ast::Stmt;
    if let Ok(parsed) = ruff_python_parser::parse_module(code) {
        let m = parsed.into_syntax();
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
        (functions, classes)
    } else {
        (0, 0)
    }
}

/// Executes the stats command - generates comprehensive project report.
///
/// # Errors
///
/// Returns an error if file I/O fails or JSON serialization fails.
#[allow(
    clippy::fn_params_excessive_bools,
    clippy::too_many_lines,
    clippy::cast_precision_loss
)]
pub fn run_stats_v2<W: Write>(
    root: &Path,
    roots: &[PathBuf],
    all: bool,
    secrets: bool,
    danger: bool,
    quality: bool,
    json: bool,
    output: Option<String>,
    exclude: &[String],
    include_tests: bool,
    include_folders: &[String],
    verbose: bool,
    config: Config,
    mut writer: W,
) -> Result<usize> {
    let output = if let Some(out) = output {
        Some(crate::utils::validate_output_path(
            Path::new(&out),
            Some(root),
        )?)
    } else {
        None
    };

    // Use collect_python_files_gitignore to get both files and directory count in one pass.
    // This is more efficient and ensures consistent exclusion logic.
    let mut files = Vec::new();
    let mut num_directories = 0;
    for path in roots {
        let (f, d) = crate::utils::collect_python_files_gitignore(
            path,
            exclude,
            include_folders,
            false, // include_ipynb: stats command defaults to py files only for now
            verbose,
        );
        files.extend(f);
        num_directories += d;
    }

    // Filter out test files if include_tests is false
    let files: Vec<PathBuf> = if !include_tests {
        files
            .into_iter()
            .filter(|p| !crate::utils::is_test_path(&p.to_string_lossy()))
            .collect()
    } else {
        files
    };

    let file_metrics: Vec<FileMetrics> = files
        .par_iter()
        .filter(|p| p.is_file())
        .map(|file_path| {
            let code = fs::read_to_string(file_path).unwrap_or_default();
            let metrics = analyze_raw(&code);
            let size_bytes = fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);
            FileMetrics {
                file: file_path.to_string_lossy().to_string(),
                code_lines: metrics.sloc,
                comment_lines: metrics.comments,
                empty_lines: metrics.blank,
                total_lines: metrics.loc,
                size_kb: size_bytes as f64 / 1024.0,
            }
        })
        .collect();

    let (total_functions, total_classes): (usize, usize) = files
        .par_iter()
        .filter(|p| p.is_file())
        .map(|file_path| {
            let code = fs::read_to_string(file_path).unwrap_or_default();
            count_functions_and_classes(&code, file_path)
        })
        .reduce(|| (0, 0), |(f1, c1), (f2, c2)| (f1 + f2, c1 + c2));

    let total_files = file_metrics.len();
    let total_size_kb: f64 = file_metrics.iter().map(|f| f.size_kb).sum();
    let total_lines: usize = file_metrics.iter().map(|f| f.total_lines).sum();
    let code_lines: usize = file_metrics.iter().map(|f| f.code_lines).sum();
    let comment_lines: usize = file_metrics.iter().map(|f| f.comment_lines).sum();
    let empty_lines: usize = file_metrics.iter().map(|f| f.empty_lines).sum();

    let include_files = all;
    let include_secrets = all || secrets;
    let include_danger = all || danger;
    let include_quality = all || quality;

    let analysis_result = if include_secrets || include_danger || include_quality {
        let mut analyzer = CytoScnPy::default()
            .with_tests(include_tests)
            .with_includes(include_folders.to_vec())
            .with_secrets(include_secrets)
            .with_danger(include_danger)
            .with_quality(include_quality)
            .with_excludes(exclude.to_vec())
            .with_config(config);
        Some(analyzer.analyze_paths(roots))
    } else {
        None
    };

    let report = StatsReport {
        total_files,
        total_directories: num_directories,
        total_size_kb,
        total_lines,
        code_lines,
        comment_lines,
        empty_lines,
        total_functions,
        total_classes,
        files: if include_files {
            Some(file_metrics.clone())
        } else {
            None
        },
        secrets: if include_secrets {
            analysis_result.as_ref().map(|r| {
                r.secrets
                    .iter()
                    .map(|s| format!("{}:{}: {}", s.file.display(), s.line, s.message))
                    .collect()
            })
        } else {
            None
        },
        danger: if include_danger {
            analysis_result.as_ref().map(|r| {
                r.danger
                    .iter()
                    .map(|d| format!("{}:{}: {}", d.file.display(), d.line, d.message))
                    .collect()
            })
        } else {
            None
        },
        quality: if include_quality {
            analysis_result.as_ref().map(|r| {
                r.quality
                    .iter()
                    .map(|q| format!("{}:{}: {}", q.file.display(), q.line, q.message))
                    .collect()
            })
        } else {
            None
        },
    };

    if json {
        let json_output = serde_json::to_string_pretty(&report)?;
        if let Some(ref file_path) = output {
            fs::write(file_path, &json_output)?;
            writeln!(writer, "Report written to: {}", file_path.display())?;
        } else {
            writeln!(writer, "{json_output}")?;
        }
    } else {
        let mut md = String::new();
        md.push_str("# CytoScnPy Project Statistics Report\n\n");
        md.push_str("## Overview\n\n");
        md.push_str("| Metric              |        Value |\n");
        md.push_str("|---------------------|-------------:|\n");
        md.push_str(&format!("| Total Files         | {total_files:>12} |\n"));
        md.push_str(&format!(
            "| Total Directories   | {num_directories:>12} |\n"
        ));
        md.push_str(&format!(
            "| Total Size          | {total_size_kb:>9.2} KB |\n"
        ));
        md.push_str(&format!("| Total Lines         | {total_lines:>12} |\n"));
        md.push_str(&format!("| Code Lines          | {code_lines:>12} |\n"));
        md.push_str(&format!("| Comment Lines       | {comment_lines:>12} |\n"));
        md.push_str(&format!("| Empty Lines         | {empty_lines:>12} |\n"));
        md.push_str(&format!(
            "| Functions           | {total_functions:>12} |\n"
        ));
        md.push_str(&format!("| Classes             | {total_classes:>12} |\n"));

        if include_files {
            md.push_str("\n## Per-File Metrics\n\n");
            md.push_str("| File | Code | Comments | Empty | Total | Size (KB) |\n");
            md.push_str("|------|------|----------|-------|-------|----------|\n");
            for f in &file_metrics {
                let short_name = Path::new(&f.file)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| f.file.clone());
                md.push_str(&format!(
                    "| {} | {} | {} | {} | {} | {:.2} |\n",
                    short_name,
                    f.code_lines,
                    f.comment_lines,
                    f.empty_lines,
                    f.total_lines,
                    f.size_kb
                ));
            }
        }

        if include_secrets {
            md.push_str("\n## Secrets Scan\n\n");
            if let Some(ref result) = analysis_result {
                if result.secrets.is_empty() {
                    md.push_str("No secrets detected.\n");
                } else {
                    md.push_str("| File | Line | Issue |\n");
                    md.push_str("|------|------|-------|\n");
                    for s in &result.secrets {
                        let short_file = s
                            .file
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| s.file.display().to_string());
                        md.push_str(&format!(
                            "| {} | {} | {} |\n",
                            short_file, s.line, s.message
                        ));
                    }
                }
            }
        }

        if include_danger {
            md.push_str("\n## Dangerous Code\n\n");
            if let Some(ref result) = analysis_result {
                if result.danger.is_empty() {
                    md.push_str("No dangerous code patterns detected.\n");
                } else {
                    md.push_str("| File | Line | Issue |\n");
                    md.push_str("|------|------|-------|\n");
                    for d in &result.danger {
                        let short_file = d
                            .file
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| d.file.display().to_string());
                        md.push_str(&format!(
                            "| {} | {} | {} |\n",
                            short_file, d.line, d.message
                        ));
                    }
                }
            }
        }

        if include_quality {
            md.push_str("\n## Quality Issues\n\n");
            if let Some(ref result) = analysis_result {
                if result.quality.is_empty() {
                    md.push_str("No quality issues detected.\n");
                } else {
                    md.push_str("| File | Line | Issue |\n");
                    md.push_str("|------|------|-------|\n");
                    for q in &result.quality {
                        let short_file = q
                            .file
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| q.file.display().to_string());
                        md.push_str(&format!(
                            "| {} | {} | {} |\n",
                            short_file, q.line, q.message
                        ));
                    }
                }
            }
        }

        if let Some(output_path) = output {
            fs::write(&output_path, &md)?;
            writeln!(writer, "{}", "Report generated successfully!".green())?;
            writeln!(
                writer,
                "Output: {}",
                output_path.display().to_string().cyan()
            )?;
        } else {
            writeln!(writer, "{md}")?;
        }
    }

    // Return quality issue count for fail-on-quality gate
    let quality_count = analysis_result
        .as_ref()
        .map(|r| r.quality.len())
        .unwrap_or(0);

    Ok(quality_count)
}

/// Executes the files command - shows per-file metrics table.
///
/// # Errors
///
/// Returns an error if file I/O fails or JSON serialization fails.
#[allow(clippy::cast_precision_loss)]
pub fn run_files<W: Write>(
    roots: &[PathBuf],
    json: bool,
    exclude: &[String],
    verbose: bool,
    mut writer: W,
) -> Result<()> {
    let files = find_python_files(roots, exclude, verbose);

    let file_metrics: Vec<FileMetrics> = files
        .par_iter()
        .filter(|p| p.is_file())
        .map(|file_path| {
            let code = fs::read_to_string(file_path).unwrap_or_default();
            let metrics = analyze_raw(&code);
            let size_bytes = fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);
            FileMetrics {
                file: file_path.to_string_lossy().to_string(),
                code_lines: metrics.sloc,
                comment_lines: metrics.comments,
                empty_lines: metrics.blank,
                total_lines: metrics.loc,
                size_kb: size_bytes as f64 / 1024.0,
            }
        })
        .collect();

    if json {
        writeln!(writer, "{}", serde_json::to_string_pretty(&file_metrics)?)?;
    } else {
        let mut table = Table::new();
        table.set_header(vec![
            "File",
            "Code",
            "Comments",
            "Empty",
            "Total",
            "Size (KB)",
        ]);

        for f in file_metrics {
            let short_name = Path::new(&f.file)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| f.file.clone());
            table.add_row(vec![
                short_name,
                f.code_lines.to_string(),
                f.comment_lines.to_string(),
                f.empty_lines.to_string(),
                f.total_lines.to_string(),
                format!("{:.2}", f.size_kb),
            ]);
        }

        writeln!(writer, "{table}")?;
    }

    Ok(())
}

/// Executes the stats command (original signature for backward compatibility).
///
/// # Errors
///
/// Returns an error if file I/O fails or JSON serialization fails.
#[deprecated(since = "1.2.2", note = "use run_stats_v2 instead")]
pub fn run_stats<W: Write>(
    root: &Path,
    roots: &[PathBuf],
    all: bool,
    secrets: bool,
    danger: bool,
    quality: bool,
    json: bool,
    output: Option<String>,
    exclude: &[String],
    verbose: bool,
    writer: W,
) -> Result<usize> {
    run_stats_v2(
        root,
        roots,
        all,
        secrets,
        danger,
        quality,
        json,
        output,
        exclude,
        false, // include_tests defaults to false
        &[],   // include_folders defaults to empty
        verbose,
        Config::default(),
        writer,
    )
}
