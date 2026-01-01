//! Maintainability Index (MI) analysis command.

use super::utils::{find_python_files, write_output};
use crate::halstead::analyze_halstead;
use crate::metrics::{mi_compute, mi_rank};
use crate::raw_metrics::analyze_raw;

use anyhow::Result;
use colored::Colorize;
use comfy_table::Table;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;

/// Options for Maintainability Index analysis
#[derive(Debug, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct MiOptions {
    /// Output in JSON format.
    pub json: bool,
    /// List of paths to exclude patterns.
    pub exclude: Vec<String>,
    /// List of specific file patterns to ignore.
    pub ignore: Vec<String>,
    /// Minimum rank to show.
    pub min_rank: Option<char>,
    /// Maximum rank to show.
    pub max_rank: Option<char>,
    /// Use multi-line comments in calculation.
    pub multi: bool,
    /// Show MI value in output table.
    pub show: bool,
    /// Calculate and show average MI.
    pub average: bool,
    /// Fail if any file MI is under this threshold.
    pub fail_threshold: Option<f64>,
    /// Write output to this file path.
    pub output_file: Option<String>,
    /// Verbose output.
    pub verbose: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct MiResult {
    file: String,
    mi: f64,
    rank: char,
}

/// Executes the Maintainability Index (MI) analysis.
///
/// # Errors
///
/// Returns an error if file I/O fails or JSON serialization fails.
#[allow(clippy::cast_precision_loss)]
pub fn run_mi<W: Write>(path: &Path, options: MiOptions, mut writer: W) -> Result<()> {
    let mut all_exclude = options.exclude;
    all_exclude.extend(options.ignore);
    let files = find_python_files(path, &all_exclude, options.verbose);

    let mut results: Vec<MiResult> = files
        .par_iter()
        .map(|file_path| {
            let code = fs::read_to_string(file_path).unwrap_or_default();

            let raw = analyze_raw(&code);
            let mut volume = 0.0;

            if let Ok(parsed) = ruff_python_parser::parse_module(&code) {
                let module = parsed.into_syntax();
                let mod_enum = ruff_python_ast::Mod::Module(module);
                let h_metrics = analyze_halstead(&mod_enum);
                volume = h_metrics.volume;
            }

            let complexity = crate::complexity::calculate_module_complexity(&code).unwrap_or(1);

            let comments = if options.multi {
                raw.comments + raw.multi
            } else {
                raw.comments
            };

            let mi = mi_compute(volume, complexity, raw.sloc, comments);
            let rank = mi_rank(mi);

            MiResult {
                file: file_path.to_string_lossy().to_string(),
                mi,
                rank,
            }
        })
        .collect();

    // Calculate and show average if requested
    if options.average {
        let total_mi: f64 = results.iter().map(|r| r.mi).sum();
        let count = results.len();
        let avg = if count > 0 {
            total_mi / count as f64
        } else {
            0.0
        };
        let msg = format!("Average MI: {avg:.2}");
        write_output(&mut writer, &msg, options.output_file.clone())?;
    }

    // Check failure threshold
    if let Some(threshold) = options.fail_threshold {
        let violations: Vec<&MiResult> = results.iter().filter(|r| r.mi < threshold).collect();
        if !violations.is_empty() {
            eprintln!(
                "\n[Error] The following files have a Maintainability Index below {threshold}:"
            );
            for v in violations {
                eprintln!("  {} - MI: {:.2}", v.file, v.mi);
            }
            std::process::exit(1);
        }
    }

    // Filter by rank
    if let Some(min) = options.min_rank {
        results.retain(|r| r.rank >= min);
    }
    if let Some(max) = options.max_rank {
        results.retain(|r| r.rank <= max);
    }

    if options.json {
        write_output(
            &mut writer,
            &serde_json::to_string_pretty(&results)?,
            options.output_file,
        )?;
    } else {
        let mut table = Table::new();
        if options.show {
            table.set_header(vec!["File", "MI", "Rank"]);
        } else {
            table.set_header(vec!["File", "Rank"]);
        }

        for r in results {
            let rank_colored = match r.rank {
                'A' => r.rank.to_string().green(),
                'B' => r.rank.to_string().yellow(),
                'C' => r.rank.to_string().red(),
                _ => r.rank.to_string().normal(),
            };

            let mut row = vec![r.file.clone()];
            if options.show {
                row.push(format!("{:.2}", r.mi));
            }
            row.push(rank_colored.to_string());
            table.add_row(row);
        }
        write_output(&mut writer, &table.to_string(), options.output_file)?;
    }
    Ok(())
}
