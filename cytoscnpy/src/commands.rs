use crate::complexity::analyze_complexity;
use crate::halstead::{analyze_halstead, analyze_halstead_functions};
use crate::metrics::{mi_compute, mi_rank};
use crate::raw_metrics::analyze_raw;

use anyhow::Result;
use colored::Colorize;
use comfy_table::Table;
use rayon::prelude::*;

use serde::{Deserialize, Serialize};
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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
}

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
    mut writer: W,
) -> Result<()> {
    let mut all_exclude = exclude;
    all_exclude.extend(ignore);
    let files = find_python_files(path, &all_exclude);

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

fn write_output<W: Write>(
    writer: &mut W,
    content: &str,
    output_file: Option<String>,
) -> Result<()> {
    if let Some(path) = output_file {
        let mut file = fs::File::create(path)?;
        writeln!(file, "{content}")?;
    } else {
        writeln!(writer, "{content}")?;
    }
    Ok(())
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
            "lines" => results.sort_by(|a, b| a.line.cmp(&b.line)), // Approximate line order
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
#[derive(Serialize)]
struct HalResult {
    file: String,
    name: String,
    h1: usize,
    h2: usize,
    n1: usize,
    n2: usize,
    vocabulary: f64,
    volume: f64,
    difficulty: f64,
    effort: f64,
}

/// Executes the Halstead metrics analysis.
///
/// # Errors
///
/// Returns an error if file I/O fails or JSON serialization fails.
pub fn run_hal<W: Write>(
    path: &Path,
    json: bool,
    exclude: Vec<String>,
    ignore: Vec<String>,
    functions: bool,
    output_file: Option<String>,
    mut writer: W,
) -> Result<()> {
    let mut all_exclude = exclude;
    all_exclude.extend(ignore);
    let files = find_python_files(path, &all_exclude);

    let results: Vec<HalResult> = files
        .par_iter()
        .flat_map(|file_path| {
            let code = fs::read_to_string(file_path).unwrap_or_default();
            let mut file_results = Vec::new();

            // Use ruff's parse_module which returns the parsed AST directly
            if let Ok(parsed) = ruff_python_parser::parse_module(&code) {
                let module = parsed.into_syntax();
                // ruff's analyze functions expect the specific Mod variant or we need to adapt
                // analyze_halstead expects &Mod, but module is ModModule.
                // ModModule is a struct, not an enum variant directly comparable to Mod::Module?
                // Actually ruff_python_ast::Mod is the enum. ModModule is a variant wrapper?
                // No, ModModule is the struct for Module.
                // We likely need to wrap it in Mod::Module(module) if the analyze functions expect Mod::Module
                // But wait, the previous code was `Mod::Module { body: module.body, ... }`.
                // Let's assume we can construct Mod::Module from it or chang analyze_halstead signature.
                // Easier to construct Mod::Module for now if possible, or cast.
                // Actually, let's look at `analyze_halstead` signature in halstead.rs via earlier read...
                // It likely takes &Mod.
                // Let's construct a Mod::Module wrapping the body.
                let mod_enum = ruff_python_ast::Mod::Module(module);
                if functions {
                    let function_metrics = analyze_halstead_functions(&mod_enum);
                    for (name, metrics) in function_metrics {
                        file_results.push(HalResult {
                            file: file_path.to_string_lossy().to_string(),
                            name,
                            h1: metrics.h1,
                            h2: metrics.h2,
                            n1: metrics.n1,
                            n2: metrics.n2,
                            vocabulary: metrics.vocabulary,
                            volume: metrics.volume,
                            difficulty: metrics.difficulty,
                            effort: metrics.effort,
                        });
                    }
                } else {
                    let metrics = analyze_halstead(&mod_enum);
                    file_results.push(HalResult {
                        file: file_path.to_string_lossy().to_string(),
                        name: "<module>".to_owned(),
                        h1: metrics.h1,
                        h2: metrics.h2,
                        n1: metrics.n1,
                        n2: metrics.n2,
                        vocabulary: metrics.vocabulary,
                        volume: metrics.volume,
                        difficulty: metrics.difficulty,
                        effort: metrics.effort,
                    });
                }
            }
            file_results
        })
        .collect();

    if json {
        write_output(
            &mut writer,
            &serde_json::to_string_pretty(&results)?,
            output_file,
        )?;
    } else {
        let mut table = Table::new();
        if functions {
            table.set_header(vec![
                "File", "Name", "h1", "h2", "N1", "N2", "Vocab", "Volume", "Diff", "Effort",
            ]);
        } else {
            table.set_header(vec![
                "File", "h1", "h2", "N1", "N2", "Vocab", "Volume", "Diff", "Effort",
            ]);
        }

        for r in results {
            let mut row = vec![r.file.clone()];
            if functions {
                row.push(r.name.clone());
            }
            row.extend(vec![
                r.h1.to_string(),
                r.h2.to_string(),
                r.n1.to_string(),
                r.n2.to_string(),
                format!("{:.2}", r.vocabulary),
                format!("{:.2}", r.volume),
                format!("{:.2}", r.difficulty),
                format!("{:.2}", r.effort),
            ]);
            table.add_row(row);
        }
        write_output(&mut writer, &table.to_string(), output_file)?;
    }
    Ok(())
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
    let files = find_python_files(path, &all_exclude);

    let mut results: Vec<MiResult> = files
        .par_iter()
        .map(|file_path| {
            let code = fs::read_to_string(file_path).unwrap_or_default();

            let raw = analyze_raw(&code);
            let mut volume = 0.0;

            // Use ruff's parse_module
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

fn find_python_files(root: &Path, exclude: &[String]) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|e| {
            let path = e.path();
            // Skip excluded directories
            if path.is_dir() {
                return !exclude.iter().any(|ex| path.to_string_lossy().contains(ex));
            }
            // Only include .py files
            path.extension().is_some_and(|ext| ext == "py")
        })
        .filter(|e| e.path().is_file()) // Exclude directories from results
        .map(|e| e.path().to_path_buf())
        .collect()
}
