use super::model::FileMetrics;
use crate::commands::metric_source::{discover_files, read_source};
use crate::raw_metrics::analyze_raw;
use anyhow::{Context, Result};
use comfy_table::Table;
use rayon::prelude::*;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

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
    writer: W,
) -> Result<()> {
    run_files_with_tests(roots, json, exclude, false, verbose, writer)
}

/// Executes the files command with explicit test-file inclusion behavior.
pub fn run_files_with_tests<W: Write>(
    roots: &[PathBuf],
    json: bool,
    exclude: &[String],
    include_tests: bool,
    verbose: bool,
    mut writer: W,
) -> Result<()> {
    let files = discover_files(roots, exclude, include_tests, verbose)?;
    let file_metrics: Vec<FileMetrics> = files
        .par_iter()
        .map(|file_path| {
            let code = read_source(file_path)?;
            let metrics = analyze_raw(&code);
            let size_bytes = fs::metadata(file_path)
                .with_context(|| format!("Failed to inspect {}", file_path.display()))?
                .len();
            Ok(FileMetrics {
                file: file_path.to_string_lossy().to_string(),
                code_lines: metrics.sloc,
                comment_lines: metrics.comments,
                empty_lines: metrics.blank,
                total_lines: metrics.loc,
                size_kb: size_bytes as f64 / 1024.0,
            })
        })
        .collect::<Result<Vec<_>>>()?;

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
