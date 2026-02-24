//! Stats and files commands.

mod collect;
mod markdown;
mod model;

pub use model::{Inspections, ScanOptions};

use super::utils::find_python_files;
use crate::analyzer::CytoScnPy;
use crate::config::Config;
use crate::raw_metrics::analyze_raw;

use anyhow::Result;
use colored::Colorize;
use comfy_table::Table;
use rayon::prelude::*;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use collect::collect_project_stats;
use markdown::generate_markdown_report_v2;
use model::{FileMetrics, ProjectStats, StatsReport};

/// Executes the stats command - generates comprehensive project report.
///
/// # Errors
///
/// Returns an error if file I/O fails or JSON serialization fails.
#[allow(clippy::cast_precision_loss)]
pub fn run_stats_v2<W: Write>(
    root: &Path,
    roots: &[PathBuf],
    options: ScanOptions,
    output: Option<String>,
    exclude: &[String],
    include_tests: bool,
    include_folders: &[String],
    verbose: bool,
    config: Config,
    writer: W,
) -> Result<usize> {
    let output = if let Some(out) = output {
        Some(crate::utils::validate_output_path(
            Path::new(&out),
            Some(root),
        )?)
    } else {
        None
    };

    let stats = collect_project_stats(roots, exclude, include_folders, include_tests, verbose);
    let (analysis_result, report) = perform_stats_analysis(
        &stats,
        roots,
        exclude,
        include_folders,
        include_tests,
        options,
        config,
    );

    generate_stats_output(
        &report,
        analysis_result.as_ref(),
        &stats.file_metrics,
        output,
        options,
        writer,
    )?;

    Ok(analysis_result.as_ref().map_or(0, |r| r.quality.len()))
}

fn perform_stats_analysis(
    stats: &ProjectStats,
    roots: &[PathBuf],
    exclude: &[String],
    include_folders: &[String],
    include_tests: bool,
    options: ScanOptions,
    config: Config,
) -> (Option<crate::analyzer::AnalysisResult>, StatsReport) {
    let include_secrets = options.include_secrets();
    let include_danger = options.include_danger();
    let include_quality = options.include_quality();

    let analysis_result = if options.is_any_enabled() {
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

    let report = create_stats_report(stats, analysis_result.as_ref(), options);
    (analysis_result, report)
}

fn create_stats_report(
    stats: &ProjectStats,
    analysis_result: Option<&crate::analyzer::AnalysisResult>,
    options: ScanOptions,
) -> StatsReport {
    let include_secrets = options.include_secrets();
    let include_danger = options.include_danger();
    let include_quality = options.include_quality();

    StatsReport {
        total_files: stats.total_files,
        total_directories: stats.total_directories,
        total_size_kb: stats.total_size_kb,
        total_lines: stats.total_lines,
        code_lines: stats.code_lines,
        comment_lines: stats.comment_lines,
        empty_lines: stats.empty_lines,
        total_functions: stats.total_functions,
        total_classes: stats.total_classes,
        files: if options.all {
            Some(stats.file_metrics.clone())
        } else {
            None
        },
        secrets: if include_secrets {
            analysis_result.map(|r| {
                r.secrets
                    .iter()
                    .map(|s| format!("{}:{}: {}", s.file.display(), s.line, s.message))
                    .collect()
            })
        } else {
            None
        },
        danger: if include_danger {
            analysis_result.map(|r| {
                r.danger
                    .iter()
                    .map(|d| format!("{}:{}: {}", d.file.display(), d.line, d.message))
                    .collect()
            })
        } else {
            None
        },
        quality: if include_quality {
            analysis_result.map(|r| {
                r.quality
                    .iter()
                    .map(|q| format!("{}:{}: {}", q.file.display(), q.line, q.message))
                    .collect()
            })
        } else {
            None
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn generate_stats_output<W: Write>(
    report: &StatsReport,
    analysis_result: Option<&crate::analyzer::AnalysisResult>,
    file_metrics: &[FileMetrics],
    output: Option<PathBuf>,
    options: ScanOptions,
    mut writer: W,
) -> Result<()> {
    if options.json {
        let json_output = serde_json::to_string_pretty(report)?;
        if let Some(ref file_path) = output {
            fs::write(file_path, &json_output)?;
            writeln!(writer, "Report written to: {}", file_path.display())?;
        } else {
            writeln!(writer, "{json_output}")?;
        }
    } else {
        let md = generate_markdown_report_v2(report, analysis_result, file_metrics, options);
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
    Ok(())
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
            let size_bytes = fs::metadata(file_path).map_or(0, |m| m.len());
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
        return Ok(());
    }

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
            .map_or_else(|| f.file.clone(), |n| n.to_string_lossy().to_string());
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
    Ok(())
}

/// Executes the stats command (original signature for backward compatibility).
///
/// # Errors
///
/// Returns an error if file I/O fails or JSON serialization fails.
#[deprecated(since = "1.2.2", note = "use run_stats_v2 instead")]
#[allow(clippy::fn_params_excessive_bools)]
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
        ScanOptions {
            all,
            inspections: Inspections {
                secrets,
                danger,
                quality,
            },
            json,
        },
        output,
        exclude,
        false,
        &[],
        verbose,
        Config::default(),
        writer,
    )
}
