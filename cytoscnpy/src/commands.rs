use crate::analyzer::CytoScnPy;
use crate::complexity::analyze_complexity;
use crate::config::Config;
use crate::constants::DEFAULT_EXCLUDE_FOLDERS;
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
    // Merge user excludes with default excludes (.venv, __pycache__, etc.)
    let default_excludes: Vec<String> = DEFAULT_EXCLUDE_FOLDERS()
        .iter()
        .map(|&s| s.to_owned())
        .collect();
    let all_excludes: Vec<String> = exclude.iter().cloned().chain(default_excludes).collect();

    WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            // Prune excluded directories (prevents descent)
            let path = e.path();
            if path.is_dir() {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default();
                return !all_excludes.iter().any(|ex| name.contains(ex));
            }
            true
        })
        .filter_map(std::result::Result::ok)
        .filter(|e| {
            let path = e.path();
            // Only include .py files
            path.is_file() && path.extension().is_some_and(|ext| ext == "py")
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

fn count_directories(root: &Path, exclude: &[String]) -> usize {
    // Merge user excludes with default excludes
    let default_excludes: Vec<String> = DEFAULT_EXCLUDE_FOLDERS()
        .iter()
        .map(|&s| s.to_owned())
        .collect();
    let all_excludes: Vec<String> = exclude.iter().cloned().chain(default_excludes).collect();

    WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            // Prune excluded directories
            let path = e.path();
            if path.is_dir() {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default();
                return !all_excludes.iter().any(|ex| name.contains(ex));
            }
            true
        })
        .filter_map(std::result::Result::ok)
        .filter(|e| {
            let path = e.path();
            path.is_dir() && path != root
        })
        .count()
}

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
                    // Count methods inside classes
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
pub fn run_stats<W: Write>(
    path: &Path,
    all: bool,
    secrets: bool,
    danger: bool,
    quality: bool,
    json: bool,
    output: Option<String>,
    exclude: &[String],
    mut writer: W,
) -> Result<()> {
    let files = find_python_files(path, exclude);
    let num_directories = count_directories(path, exclude);

    // Collect metrics in parallel
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

    // Count functions and classes
    let (total_functions, total_classes): (usize, usize) = files
        .par_iter()
        .filter(|p| p.is_file())
        .map(|file_path| {
            let code = fs::read_to_string(file_path).unwrap_or_default();
            count_functions_and_classes(&code, file_path)
        })
        .reduce(|| (0, 0), |(f1, c1), (f2, c2)| (f1 + f2, c1 + c2));

    // Aggregate totals
    let total_files = file_metrics.len();
    let total_size_kb: f64 = file_metrics.iter().map(|f| f.size_kb).sum();
    let total_lines: usize = file_metrics.iter().map(|f| f.total_lines).sum();
    let code_lines: usize = file_metrics.iter().map(|f| f.code_lines).sum();
    let comment_lines: usize = file_metrics.iter().map(|f| f.comment_lines).sum();
    let empty_lines: usize = file_metrics.iter().map(|f| f.empty_lines).sum();

    // Build report
    let include_files = all;
    let include_secrets = all || secrets;
    let include_danger = all || danger;
    let include_quality = all || quality;

    // Run full analysis if requested
    let analysis_result = if include_secrets || include_danger || include_quality {
        let mut analyzer = CytoScnPy::default()
            .with_secrets(include_secrets)
            .with_danger(include_danger)
            .with_quality(include_quality)
            .with_excludes(exclude.to_vec())
            .with_config(Config::default());
        Some(analyzer.analyze_paths(&[path.to_path_buf()]))
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
            writeln!(writer, "Report written to: {file_path}")?;
        } else {
            writeln!(writer, "{json_output}")?;
        }
    } else {
        // Generate markdown report with better alignment
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

        // Output: write to file if -o specified, otherwise print to stdout
        if let Some(output_path) = output {
            fs::write(&output_path, &md)?;
            writeln!(writer, "{}", "Report generated successfully!".green())?;
            writeln!(writer, "Output: {}", output_path.cyan())?;
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
    path: &Path,
    json: bool,
    exclude: &[String],
    mut writer: W,
) -> Result<()> {
    let files = find_python_files(path, exclude);

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
