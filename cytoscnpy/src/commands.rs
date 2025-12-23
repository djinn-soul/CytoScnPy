use crate::analyzer::CytoScnPy;
use crate::complexity::analyze_complexity;
use crate::config::Config;
use crate::constants::DEFAULT_EXCLUDE_FOLDERS;
use crate::halstead::{analyze_halstead, analyze_halstead_functions};
use crate::metrics::{mi_compute, mi_rank};
use crate::raw_metrics::analyze_raw;

use anyhow::Result;
use colored::Colorize;
use comfy_table::{Cell, Color, Table};
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

/// Options for clone detection
#[derive(Debug, Default)]
pub struct CloneOptions {
    /// Minimum similarity threshold (0.0-1.0)
    pub similarity: f64,
    /// Output in JSON format
    pub json: bool,
    /// Auto-fix mode
    pub fix: bool,
    /// Dry-run mode (show what would change)
    pub dry_run: bool,
    /// List of paths to exclude
    pub exclude: Vec<String>,
    /// Verbose output
    pub verbose: bool,
    /// Use CST for precise fixing (comment preservation)
    pub with_cst: bool,
}

/// Generates context-aware refactoring suggestions for clone findings.
///
/// Provides actionable recommendations based on:
/// - Clone type (Exact, Renamed, Similar)
/// - Code element kind (function, class, method) from AST
/// - Similarity percentage
fn generate_clone_suggestion(
    clone_type: &crate::clones::CloneType,
    node_kind: &crate::clones::NodeKind,
    name: &str,
    similarity: f64,
) -> String {
    use crate::clones::{CloneType, NodeKind};

    // Use actual AST node type for context-aware suggestions
    let is_init = name == "__init__";
    let is_dunder = name.starts_with("__") && name.ends_with("__");

    match clone_type {
        CloneType::Type1 => {
            // Exact copy - most severe, should eliminate
            match node_kind {
                NodeKind::Class => "Remove duplicate class, import from original".to_owned(),
                NodeKind::Method if is_init => "Extract shared __init__ to base class".to_owned(),
                NodeKind::Method => "Move to base class or mixin".to_owned(),
                NodeKind::Function | NodeKind::AsyncFunction => {
                    "Remove duplicate, import from original module".to_owned()
                }
            }
        }
        CloneType::Type2 => {
            // Renamed copy - logic identical, names differ
            match node_kind {
                NodeKind::Class => "Consider inheritance or factory pattern".to_owned(),
                NodeKind::Method if is_init || is_dunder => {
                    "Extract to mixin or base class".to_owned()
                }
                NodeKind::Method => "Parameterize and move to base class".to_owned(),
                NodeKind::Function | NodeKind::AsyncFunction => {
                    "Parameterize into single configurable function".to_owned()
                }
            }
        }
        CloneType::Type3 => {
            // Similar code - structural similarity
            if similarity >= 0.9 {
                match node_kind {
                    NodeKind::Class => "High similarity: use inheritance".to_owned(),
                    NodeKind::Method if is_init => "Extract common init to base class".to_owned(),
                    NodeKind::Method => "Consider template method pattern".to_owned(),
                    NodeKind::Function | NodeKind::AsyncFunction => {
                        "Consider higher-order function or decorator".to_owned()
                    }
                }
            } else if similarity >= 0.8 {
                match node_kind {
                    NodeKind::Class => "Review for composition pattern".to_owned(),
                    NodeKind::Method => "Consider template method pattern".to_owned(),
                    NodeKind::Function | NodeKind::AsyncFunction => {
                        "Review for potential abstraction".to_owned()
                    }
                }
            } else {
                "Review for potential consolidation".to_owned()
            }
        }
    }
}

/// Executes clone detection analysis.
///
/// # Errors
///
/// Returns an error if file I/O fails or analysis fails.
///
/// Returns the number of clone pairs found.
#[allow(clippy::too_many_lines)]
pub fn run_clones<W: Write>(
    paths: &[PathBuf],
    options: CloneOptions,
    mut writer: W,
) -> Result<usize> {
    use crate::clones::{CloneConfig, CloneDetector};

    // Collect all Python files
    let all_files = load_python_files(paths, &options.exclude);

    if all_files.is_empty() {
        writeln!(writer, "No Python files found.")?;
        return Ok(0);
    }

    // Configure detector
    let config = CloneConfig::default().with_min_similarity(options.similarity);
    let detector = CloneDetector::with_config(config);

    // Run detection
    let result = detector.detect(&all_files)?;

    // Verbose: show detection statistics
    if !options.json {
        print_clone_stats(&mut writer, &all_files, &result.pairs, options.verbose)?;
    }

    if result.pairs.is_empty() {
        if options.json {
            writeln!(writer, "[]")?;
        } else {
            writeln!(writer, "{}", "No clones detected.".green())?;
        }
        return Ok(0);
    }

    // Convert to findings for output
    let findings = generate_clone_findings(&result.pairs, &all_files, options.with_cst);

    // Output results
    if options.json {
        let output = serde_json::to_string_pretty(&findings)?;
        writeln!(writer, "{output}")?;
    } else {
        writeln!(writer, "\n{}", "Clone Detection Results".bold().cyan())?;
        writeln!(writer, "{}\n", "=".repeat(40))?;

        let mut table = Table::new();
        table
            .load_preset(comfy_table::presets::UTF8_FULL)
            .set_content_arrangement(comfy_table::ContentArrangement::Dynamic)
            .set_header(vec![
                "Type",
                "Name",
                "Related To",
                "Location",
                "Similarity",
                "Suggestion",
            ]);

        for finding in &findings {
            if finding.is_duplicate {
                let type_str = finding.clone_type.display_name();
                let name = finding
                    .name
                    .clone()
                    .unwrap_or_else(|| "<anonymous>".to_owned());
                // Use file:line format like other tables
                let location = format!(
                    "{}:{}",
                    crate::utils::normalize_display_path(&finding.file),
                    finding.line
                );
                let similarity = format!("{:.0}%", finding.similarity * 100.0);
                let related = format!(
                    "{}:{}",
                    crate::utils::normalize_display_path(&finding.related_clone.file),
                    finding.related_clone.line
                );

                // Generate context-aware suggestion based on clone type and AST node
                let suggestion = generate_clone_suggestion(
                    &finding.clone_type,
                    &finding.node_kind,
                    &name,
                    finding.similarity,
                );

                table.add_row(vec![
                    Cell::new(type_str).fg(Color::Yellow),
                    Cell::new(name),
                    Cell::new(related),
                    Cell::new(location),
                    Cell::new(similarity),
                    Cell::new(suggestion).fg(Color::Cyan),
                ]);
            }
        }

        writeln!(writer, "{table}")?;
    }

    // Handle --fix mode
    if options.fix {
        apply_clone_fixes_internal(
            &mut writer,
            &findings,
            &all_files,
            options.dry_run,
            options.with_cst,
        )?;
    }

    Ok(result.pairs.len())
}

/// Helper to load all Python files from the given paths.
fn load_python_files(paths: &[PathBuf], exclude: &[String]) -> Vec<(PathBuf, String)> {
    let mut all_files = Vec::new();
    for path in paths {
        let files = find_python_files(path, exclude);
        for file in files {
            if let Ok(content) = fs::read_to_string(&file) {
                all_files.push((file, content));
            }
        }
    }
    all_files
}

/// Helper to print clone detection statistics.
fn print_clone_stats<W: Write>(
    mut writer: W,
    _all_files: &[(PathBuf, String)],
    pairs: &[crate::clones::ClonePair],
    verbose: bool,
) -> Result<()> {
    if verbose {
        writeln!(writer, "[VERBOSE] Clone Detection Statistics:")?;
        writeln!(writer, "   Files scanned: {}", _all_files.len())?;
        writeln!(writer, "   Clone pairs found: {}", pairs.len())?;

        // Count by type
        let mut type1_count = 0;
        let mut type2_count = 0;
        let mut type3_count = 0;
        for pair in pairs {
            match pair.clone_type {
                crate::clones::CloneType::Type1 => type1_count += 1,
                crate::clones::CloneType::Type2 => type2_count += 1,
                crate::clones::CloneType::Type3 => type3_count += 1,
            }
        }
        writeln!(writer, "   Exact Copies: {type1_count}")?;
        writeln!(writer, "   Renamed Copies: {type2_count}")?;
        writeln!(writer, "   Similar Code: {type3_count}")?;

        // Show average similarity
        if !pairs.is_empty() {
            let avg_similarity: f64 =
                pairs.iter().map(|p| p.similarity).sum::<f64>() / pairs.len() as f64;
            writeln!(
                writer,
                "   Average similarity: {:.0}%",
                avg_similarity * 100.0
            )?;
        }
        writeln!(writer)?;
    }
    Ok(())
}

/// Helper to generate findings from clone pairs.
fn generate_clone_findings(
    pairs: &[crate::clones::ClonePair],
    _all_files: &[(PathBuf, String)],
    _with_cst: bool,
) -> Vec<crate::clones::CloneFinding> {
    use crate::clones::{CloneFinding, ConfidenceScorer, FixContext};
    #[cfg(feature = "cst")]
    use crate::cst::{AstCstMapper, CstParser};

    let scorer = ConfidenceScorer::default();

    let mut findings: Vec<CloneFinding> = pairs
        .iter()
        .flat_map(|pair| {
            // Helper to calc confidence for an instance removal
            #[allow(unused_variables)]
            let calc_conf = |inst: &crate::clones::CloneInstance| -> u8 {
                let mut ctx = FixContext::default();
                ctx.same_file = pair.is_same_file();

                #[cfg(feature = "cst")]
                if _with_cst {
                    if let Some((_, content)) = _all_files.iter().find(|(p, _)| p == &inst.file) {
                        if let Ok(mut parser) = CstParser::new() {
                            if let Ok(tree) = parser.parse(content) {
                                let mapper = AstCstMapper::new(tree);
                                ctx.has_interleaved_comments =
                                    mapper.has_interleaved_comments(inst.start_byte, inst.end_byte);
                                ctx.deeply_nested =
                                    mapper.is_deeply_nested(inst.start_byte, inst.end_byte);
                            }
                        }
                    }
                }

                scorer.score(pair, &ctx).score
            };

            vec![
                CloneFinding::from_pair(pair, false, calc_conf(&pair.instance_a)),
                CloneFinding::from_pair(pair, true, calc_conf(&pair.instance_b)),
            ]
        })
        .collect();

    // Populate suggestions
    for finding in &mut findings {
        let name = finding.name.as_deref().unwrap_or("<anonymous>");
        finding.suggestion = Some(generate_clone_suggestion(
            &finding.clone_type,
            &finding.node_kind,
            name,
            finding.similarity,
        ));
    }

    findings
}

/// Helper to apply clone fixes.
fn apply_clone_fixes_internal<W: Write>(
    mut writer: W,
    findings: &[crate::clones::CloneFinding],
    all_files: &[(PathBuf, String)],
    dry_run: bool,
    _with_cst: bool,
) -> Result<()> {
    #[cfg(feature = "cst")]
    use crate::cst::{AstCstMapper, CstParser};
    use crate::fix::{ByteRangeRewriter, Edit};
    use colored::Colorize;

    if dry_run {
        writeln!(
            writer,
            "\n{}",
            "[DRY-RUN] Would apply the following fixes:".yellow()
        )?;
    } else {
        writeln!(writer, "\n{}", "Applying fixes...".cyan())?;
    }

    let mut edits_by_file: std::collections::HashMap<PathBuf, Vec<Edit>> =
        std::collections::HashMap::new();
    let mut seen_ranges: std::collections::HashSet<(PathBuf, usize, usize)> =
        std::collections::HashSet::new();

    for finding in findings {
        if finding.is_duplicate && finding.fix_confidence >= 90 {
            #[allow(unused_mut)]
            let mut start_byte = finding.start_byte;
            #[allow(unused_mut)]
            let mut end_byte = finding.end_byte;

            #[cfg(feature = "cst")]
            if _with_cst {
                if let Some((_, content)) = all_files.iter().find(|(p, _)| p == &finding.file) {
                    if let Ok(mut parser) = CstParser::new() {
                        if let Ok(tree) = parser.parse(content) {
                            let mapper = AstCstMapper::new(tree);
                            let (s, e) = mapper.precise_range_for_def(start_byte, end_byte);
                            start_byte = s;
                            end_byte = e;
                        }
                    }
                }
            }

            let range_key = (finding.file.clone(), start_byte, end_byte);
            if seen_ranges.contains(&range_key) {
                continue;
            }
            seen_ranges.insert(range_key);

            if dry_run {
                writeln!(
                    writer,
                    "  Would remove {} (lines {}-{}, bytes {}-{}) from {}",
                    finding.name.as_deref().unwrap_or("<anonymous>"),
                    finding.line,
                    finding.end_line,
                    start_byte,
                    end_byte,
                    finding.file.display()
                )?;
            } else {
                edits_by_file
                    .entry(finding.file.clone())
                    .or_default()
                    .push(Edit::delete(start_byte, end_byte));
            }
        }
    }

    if !dry_run {
        for (file_path, edits) in edits_by_file {
            if let Some((_, content)) = all_files.iter().find(|(p, _)| p == &file_path) {
                let mut rewriter = ByteRangeRewriter::new(content.clone());
                rewriter.add_edits(edits);
                if let Ok(fixed_content) = rewriter.apply() {
                    fs::write(&file_path, fixed_content)?;
                    writeln!(writer, "  {} {}", "Fixed:".green(), file_path.display())?;
                }
            }
        }
    }
    Ok(())
}

/// Options for dead code fix
#[derive(Debug, Default)]
pub struct DeadCodeFixOptions {
    /// Minimum confidence threshold for auto-fix (0-100)
    pub min_confidence: u8,
    /// Dry-run mode (show what would change)
    pub dry_run: bool,
    /// Types to fix
    /// Fix functions
    pub fix_functions: bool,
    /// Fix classes
    pub fix_classes: bool,
    /// Fix imports
    pub fix_imports: bool,
    /// Verbose output
    pub verbose: bool,
    /// Use CST for precise fixing
    pub with_cst: bool,
}

/// Result of dead code fix operation
#[derive(Debug, Serialize)]
pub struct FixResult {
    /// File that was fixed
    pub file: String,
    /// Number of items removed
    pub items_removed: usize,
    /// Names of removed items
    pub removed_names: Vec<String>,
}

/// Apply --fix to dead code findings.
///
/// # Errors
///
/// Returns an error if file I/O fails or fix fails.
#[allow(clippy::too_many_lines)]
pub fn run_fix_deadcode<W: Write>(
    results: &crate::analyzer::AnalysisResult,
    options: DeadCodeFixOptions,
    mut writer: W,
) -> Result<Vec<FixResult>> {
    if options.dry_run {
        writeln!(
            writer,
            "\n{}",
            "[DRY-RUN] Dead code that would be removed:".yellow()
        )?;
    } else {
        writeln!(writer, "\n{}", "Applying dead code fixes...".cyan())?;
    }

    // Collect items to remove, grouped by file
    let items_by_file = collect_items_to_fix(results, &options);

    if items_by_file.is_empty() {
        writeln!(
            writer,
            "  No items with confidence >= {} to fix.",
            options.min_confidence
        )?;
        return Ok(vec![]);
    }

    // Verbose: show fix statistics
    print_fix_stats(&mut writer, &items_by_file, results, &options)?;

    let mut all_results = Vec::new();

    for (file_path, items) in items_by_file {
        if let Some(res) = apply_dead_code_fix_to_file(&mut writer, &file_path, items, &options)? {
            all_results.push(res);
        }
    }

    Ok(all_results)
}

/// Find byte range for a definition in AST
fn find_def_range(
    body: &[ruff_python_ast::Stmt],
    name: &str,
    def_type: &str,
) -> Option<(usize, usize)> {
    use ruff_python_ast::Stmt;
    use ruff_text_size::Ranged;

    for stmt in body {
        match stmt {
            Stmt::FunctionDef(f) if def_type == "function" => {
                if f.name.as_str() == name {
                    return Some((f.range().start().to_usize(), f.range().end().to_usize()));
                }
            }
            Stmt::ClassDef(c) if def_type == "class" => {
                if c.name.as_str() == name {
                    return Some((c.range().start().to_usize(), c.range().end().to_usize()));
                }
            }
            Stmt::Import(i) if def_type == "import" => {
                for alias in &i.names {
                    let import_name = alias.asname.as_ref().unwrap_or(&alias.name);
                    if import_name.as_str() == name {
                        return Some((i.range().start().to_usize(), i.range().end().to_usize()));
                    }
                }
            }
            Stmt::ImportFrom(i) if def_type == "import" => {
                for alias in &i.names {
                    let import_name = alias.asname.as_ref().unwrap_or(&alias.name);
                    if import_name.as_str() == name && i.names.len() == 1 {
                        return Some((i.range().start().to_usize(), i.range().end().to_usize()));
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// Helper to collect items for dead code fixing.
fn collect_items_to_fix<'a>(
    results: &'a crate::analyzer::AnalysisResult,
    options: &DeadCodeFixOptions,
) -> std::collections::HashMap<PathBuf, Vec<(&'static str, &'a crate::visitor::Definition)>> {
    let mut items_by_file: std::collections::HashMap<
        PathBuf,
        Vec<(&'static str, &crate::visitor::Definition)>,
    > = std::collections::HashMap::new();

    if options.fix_functions {
        for def in &results.unused_functions {
            if def.confidence >= options.min_confidence {
                items_by_file
                    .entry((*def.file).clone())
                    .or_default()
                    .push(("function", def));
            }
        }
    }

    if options.fix_classes {
        for def in &results.unused_classes {
            if def.confidence >= options.min_confidence {
                items_by_file
                    .entry((*def.file).clone())
                    .or_default()
                    .push(("class", def));
            }
        }
    }

    if options.fix_imports {
        for def in &results.unused_imports {
            if def.confidence >= options.min_confidence {
                items_by_file
                    .entry((*def.file).clone())
                    .or_default()
                    .push(("import", def));
            }
        }
    }

    items_by_file
}

/// Helper to print fix statistics.
fn print_fix_stats<W: Write>(
    writer: &mut W,
    items_by_file: &std::collections::HashMap<
        PathBuf,
        Vec<(&'static str, &crate::visitor::Definition)>,
    >,
    results: &crate::analyzer::AnalysisResult,
    options: &DeadCodeFixOptions,
) -> Result<()> {
    if options.verbose {
        let total_items: usize = items_by_file.values().map(std::vec::Vec::len).sum();
        let files_count = items_by_file.len();

        let mut func_count = 0;
        let mut class_count = 0;
        let mut import_count = 0;
        for items in items_by_file.values() {
            for (item_type, _) in items {
                match *item_type {
                    "function" => func_count += 1,
                    "class" => class_count += 1,
                    "import" => import_count += 1,
                    _ => {}
                }
            }
        }

        writeln!(writer, "[VERBOSE] Fix Statistics:")?;
        writeln!(writer, "   Files to modify: {files_count}")?;
        writeln!(writer, "   Items to remove: {total_items}")?;
        writeln!(writer, "   Functions: {func_count}")?;
        writeln!(writer, "   Classes: {class_count}")?;
        writeln!(writer, "   Imports: {import_count}")?;

        let skipped_funcs = results
            .unused_functions
            .iter()
            .filter(|d| d.confidence < options.min_confidence)
            .count();
        let skipped_classes = results
            .unused_classes
            .iter()
            .filter(|d| d.confidence < options.min_confidence)
            .count();
        let skipped_imports = results
            .unused_imports
            .iter()
            .filter(|d| d.confidence < options.min_confidence)
            .count();
        let total_skipped = skipped_funcs + skipped_classes + skipped_imports;

        if total_skipped > 0 {
            writeln!(
                writer,
                "   Skipped (confidence < {}%): {}",
                options.min_confidence, total_skipped
            )?;
        }
        writeln!(writer)?;
    }
    Ok(())
}

/// Helper to apply dead code fixes to a single file.
fn apply_dead_code_fix_to_file<W: Write>(
    writer: &mut W,
    file_path: &Path,
    items: Vec<(&'static str, &crate::visitor::Definition)>,
    options: &DeadCodeFixOptions,
) -> Result<Option<FixResult>> {
    #[cfg(feature = "cst")]
    use crate::cst::{AstCstMapper, CstParser};
    use crate::fix::{ByteRangeRewriter, Edit};

    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            writeln!(
                writer,
                "  {} {}: {}",
                "Skip:".yellow(),
                file_path.display(),
                e
            )?;
            return Ok(None);
        }
    };

    let parsed = match ruff_python_parser::parse_module(&content) {
        Ok(p) => p,
        Err(e) => {
            writeln!(
                writer,
                "  {} {}: {}",
                "Parse error:".red(),
                file_path.display(),
                e
            )?;
            return Ok(None);
        }
    };

    let module = parsed.into_syntax();
    let mut edits = Vec::new();
    let mut removed_names = Vec::new();

    #[cfg(feature = "cst")]
    let cst_mapper = if options.with_cst {
        CstParser::new()
            .ok()
            .and_then(|mut p| p.parse(&content).ok())
            .map(AstCstMapper::new)
    } else {
        None
    };

    for (item_type, def) in &items {
        if let Some((start, end)) = find_def_range(&module.body, &def.simple_name, item_type) {
            let start_byte = start;
            let end_byte = end;

            #[cfg(feature = "cst")]
            let (start_byte, end_byte) = if let Some(mapper) = &cst_mapper {
                mapper.precise_range_for_def(start, end)
            } else {
                (start_byte, end_byte)
            };

            if options.dry_run {
                writeln!(
                    writer,
                    "  Would remove {} '{}' at {}:{}",
                    item_type,
                    def.simple_name,
                    file_path.display(),
                    def.line
                )?;
            } else {
                edits.push(Edit::delete(start_byte, end_byte));
                removed_names.push(def.simple_name.clone());
            }
        }
    }

    if !options.dry_run && !edits.is_empty() {
        let mut rewriter = ByteRangeRewriter::new(content);
        rewriter.add_edits(edits);
        if let Ok(fixed) = rewriter.apply() {
            let count = removed_names.len();
            fs::write(file_path, fixed)?;
            writeln!(
                writer,
                "  {} {} ({} removed)",
                "Fixed:".green(),
                file_path.display(),
                count
            )?;
            return Ok(Some(FixResult {
                file: file_path.to_string_lossy().to_string(),
                items_removed: count,
                removed_names,
            }));
        }
    }

    Ok(None)
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
