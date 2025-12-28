//! Cyclomatic Complexity analysis command.

use super::utils::{find_python_files, write_output};
use crate::complexity::analyze_complexity;

use anyhow::Result;
use colored::Colorize;
use comfy_table::Table;
use rayon::prelude::*;
use serde::Serialize;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Options for Cyclomatic Complexity analysis
#[derive(Debug, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct CcOptions {
    /// Output in JSON format.
    pub json: bool,
    /// List of paths to exclude patterns.
    pub exclude: Vec<String>,
    /// List of specific file patterns to ignore.
    pub ignore: Vec<String>,
    /// Minimum rank to show (e.g. 'A').
    pub min_rank: Option<char>,
    /// Maximum rank to show (e.g. 'F').
    pub max_rank: Option<char>,
    /// Calculate and show average complexity.
    pub average: bool,
    /// Only show total average, no individual file details.
    pub total_average: bool,
    /// Show complexity value in output table.
    pub show_complexity: bool,
    /// Sort order ("score", "lines", "alpha").
    pub order: Option<String>,
    /// Disable assertions/panics during analysis (safe mode).
    pub no_assert: bool,
    /// Output in XML format.
    pub xml: bool,
    /// Fail if any block complexity exceeds this threshold.
    pub fail_threshold: Option<usize>,
    /// Write output to this file path.
    pub output_file: Option<String>,
}

#[derive(Serialize)]
struct CcResult {
    file: String,
    name: String,
    type_: String,
    complexity: usize,
    rank: char,
    line: usize,
}

/// Executes the cyclomatic complexity analysis.
///
/// # Errors
///
/// Returns an error if file I/O fails or JSON/XML serialization fails.
#[allow(clippy::cast_precision_loss)]
pub fn run_cc<W: Write>(path: &Path, options: CcOptions, mut writer: W) -> Result<()> {
    let mut all_exclude = options.exclude;
    all_exclude.extend(options.ignore);
    let files = find_python_files(path, &all_exclude);

    let mut results: Vec<CcResult> = files
        .par_iter()
        .flat_map(|file_path| {
            let code = fs::read_to_string(file_path).unwrap_or_default();
            let findings = analyze_complexity(&code, file_path, options.no_assert);
            findings
                .into_iter()
                .map(|f| CcResult {
                    file: file_path.to_string_lossy().to_string(),
                    name: f.name,
                    type_: f.type_,
                    complexity: f.complexity,
                    rank: f.rank,
                    line: f.line,
                })
                .collect::<Vec<_>>()
        })
        .collect();

    // Check failure threshold
    if let Some(threshold) = options.fail_threshold {
        let violations: Vec<&CcResult> = results
            .iter()
            .filter(|r| r.complexity > threshold)
            .collect();
        if !violations.is_empty() {
            eprintln!(
                "\n[Error] The following blocks exceed the complexity threshold of {threshold}:"
            );
            for v in violations {
                eprintln!(
                    "  {}:{}:{} - Complexity: {}",
                    v.file, v.line, v.name, v.complexity
                );
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

    // Order results
    if let Some(ord) = options.order {
        match ord.as_str() {
            "score" => results.sort_by(|a, b| b.complexity.cmp(&a.complexity)),
            "lines" => results.sort_by(|a, b| a.line.cmp(&b.line)),
            "alpha" => results.sort_by(|a, b| a.name.cmp(&b.name)),
            _ => {}
        }
    }

    if options.average || options.total_average {
        let total_complexity: usize = results.iter().map(|r| r.complexity).sum();
        let count = results.len();
        let avg = if count > 0 {
            total_complexity as f64 / count as f64
        } else {
            0.0
        };

        let msg = format!("Average complexity: {avg:.2} ({count} blocks)");
        write_output(&mut writer, &msg, options.output_file.clone())?;
        if options.total_average {
            return Ok(());
        }
    }

    if options.json {
        write_output(
            &mut writer,
            &serde_json::to_string_pretty(&results)?,
            options.output_file,
        )?;
    } else if options.xml {
        // Simple XML output
        let mut xml_out = String::from("<cc_metrics>\n");
        for r in results {
            let _ = write!(
                xml_out,
                "  <block>\n    <file>{}</file>\n    <name>{}</name>\n    <complexity>{}</complexity>\n    <rank>{}</rank>\n  </block>\n",
                r.file, r.name, r.complexity, r.rank
            );
        }
        xml_out.push_str("</cc_metrics>");
        write_output(&mut writer, &xml_out, options.output_file)?;
    } else {
        let mut table = Table::new();
        if options.show_complexity {
            table.set_header(vec!["File", "Name", "Type", "Line", "Complexity", "Rank"]);
        } else {
            table.set_header(vec!["File", "Name", "Type", "Line", "Rank"]);
        }

        for r in results {
            let rank_colored = match r.rank {
                'A' | 'B' => r.rank.to_string().green(),
                'C' | 'D' => r.rank.to_string().yellow(),
                'E' => r.rank.to_string().red(),
                'F' => r.rank.to_string().red().bold(),
                _ => r.rank.to_string().normal(),
            };

            let mut row = vec![
                r.file.clone(),
                r.name.clone(),
                r.type_.clone(),
                r.line.to_string(),
            ];
            if options.show_complexity {
                row.push(r.complexity.to_string());
            }
            row.push(rank_colored.to_string());
            table.add_row(row);
        }
        write_output(&mut writer, &table.to_string(), options.output_file)?;
    }
    Ok(())
}
