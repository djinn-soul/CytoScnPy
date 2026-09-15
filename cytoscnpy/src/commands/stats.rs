//! Stats and files commands.

mod collect;
mod files;
mod markdown;
mod model;
pub use files::{run_files, run_files_with_tests};

pub use model::{Inspections, ScanOptions};

use crate::analyzer::CytoScnPy;
use crate::config::Config;

use anyhow::Result;
use colored::Colorize;
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
