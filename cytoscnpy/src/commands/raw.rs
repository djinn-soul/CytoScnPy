//! Raw metrics analysis command (LOC, SLOC, etc.).

use super::utils::{find_python_files, write_output};
use crate::raw_metrics::analyze_raw;

use anyhow::Result;
use comfy_table::Table;
use rayon::prelude::*;
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Serialize)]
struct RawResult {
    file: String,
    loc: usize,
    lloc: usize,
    sloc: usize,
    comments: usize,
    multi: usize,
    blank: usize,
}

/// Executes the raw metrics analysis (LOC, SLOC, etc.).
///
/// # Errors
///
/// Returns an error if file I/O fails or JSON serialization fails.
pub fn run_raw<W: Write>(
    path: &Path,
    json: bool,
    exclude: Vec<String>,
    ignore: Vec<String>,
    summary: bool,
    output_file: Option<String>,
    verbose: bool,
    mut writer: W,
) -> Result<()> {
    let mut all_exclude = exclude;
    all_exclude.extend(ignore);
    let files = find_python_files(path, &all_exclude, verbose);

    let results: Vec<RawResult> = files
        .par_iter()
        .map(|file_path| {
            let code = fs::read_to_string(file_path).unwrap_or_default();
            let metrics = analyze_raw(&code);
            RawResult {
                file: file_path.to_string_lossy().to_string(),
                loc: metrics.loc,
                lloc: metrics.lloc,
                sloc: metrics.sloc,
                comments: metrics.comments,
                multi: metrics.multi,
                blank: metrics.blank,
            }
        })
        .collect();

    if summary {
        let loc_sum: usize = results.iter().map(|r| r.loc).sum();
        let lloc_sum: usize = results.iter().map(|r| r.lloc).sum();
        let sloc_sum: usize = results.iter().map(|r| r.sloc).sum();
        let total_comments: usize = results.iter().map(|r| r.comments).sum();
        let total_multi: usize = results.iter().map(|r| r.multi).sum();
        let total_blank: usize = results.iter().map(|r| r.blank).sum();
        let total_files = results.len();

        if json {
            let summary_json = serde_json::json!({
                "files": total_files,
                "loc": loc_sum,
                "lloc": lloc_sum,
                "sloc": sloc_sum,
                "comments": total_comments,
                "multi": total_multi,
                "blank": total_blank,
            });
            write_output(
                &mut writer,
                &serde_json::to_string_pretty(&summary_json)?,
                output_file,
            )?;
        } else {
            let mut table = Table::new();
            table.set_header(vec![
                "Files", "LOC", "LLOC", "SLOC", "Comments", "Multi", "Blank",
            ]);
            table.add_row(vec![
                total_files.to_string(),
                loc_sum.to_string(),
                lloc_sum.to_string(),
                sloc_sum.to_string(),
                total_comments.to_string(),
                total_multi.to_string(),
                total_blank.to_string(),
            ]);
            write_output(&mut writer, &table.to_string(), output_file)?;
        }
        return Ok(());
    }

    if json {
        write_output(
            &mut writer,
            &serde_json::to_string_pretty(&results)?,
            output_file,
        )?;
    } else {
        let mut table = Table::new();
        table.set_header(vec![
            "File", "LOC", "LLOC", "SLOC", "Comments", "Multi", "Blank",
        ]);

        for r in results {
            table.add_row(vec![
                r.file,
                r.loc.to_string(),
                r.lloc.to_string(),
                r.sloc.to_string(),
                r.comments.to_string(),
                r.multi.to_string(),
                r.blank.to_string(),
            ]);
        }
        write_output(&mut writer, &table.to_string(), output_file)?;
    }
    Ok(())
}
