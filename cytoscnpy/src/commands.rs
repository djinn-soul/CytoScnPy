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
}

/// Executes clone detection analysis.
///
/// # Errors
///
/// Returns an error if file I/O fails or analysis fails.
#[allow(clippy::too_many_lines)]
pub fn run_clones<W: Write>(paths: &[PathBuf], options: CloneOptions, mut writer: W) -> Result<()> {
    use crate::clones::{CloneConfig, CloneDetector, CloneFinding};
    use crate::fix::{ByteRangeRewriter, Edit};

    // Collect all Python files
    let mut all_files: Vec<(PathBuf, String)> = Vec::new();
    for path in paths {
        let files = find_python_files(path, &options.exclude);
        for file in files {
            if let Ok(content) = fs::read_to_string(&file) {
                all_files.push((file, content));
            }
        }
    }

    if all_files.is_empty() {
        writeln!(writer, "No Python files found.")?;
        return Ok(());
    }

    // Configure detector
    let config = CloneConfig::default().with_min_similarity(options.similarity);
    let detector = CloneDetector::with_config(config);

    // Run detection
    let result = detector.detect(&all_files)?;

    // Verbose: show detection statistics
    if options.verbose && !options.json {
        eprintln!("[VERBOSE] Clone Detection Statistics:");
        eprintln!("   Files scanned: {}", all_files.len());
        eprintln!("   Clone pairs found: {}", result.pairs.len());

        // Count by type
        let mut type1_count = 0;
        let mut type2_count = 0;
        let mut type3_count = 0;
        for pair in &result.pairs {
            match pair.clone_type {
                crate::clones::CloneType::Type1 => type1_count += 1,
                crate::clones::CloneType::Type2 => type2_count += 1,
                crate::clones::CloneType::Type3 => type3_count += 1,
            }
        }
        eprintln!("   Exact Copies: {type1_count}");
        eprintln!("   Renamed Copies: {type2_count}");
        eprintln!("   Similar Code: {type3_count}");

        // Show average similarity
        if !result.pairs.is_empty() {
            let avg_similarity: f64 =
                result.pairs.iter().map(|p| p.similarity).sum::<f64>() / result.pairs.len() as f64;
            eprintln!("   Average similarity: {:.0}%", avg_similarity * 100.0);
        }
        eprintln!();
    }

    if result.pairs.is_empty() {
        if options.json {
            writeln!(writer, "[]")?;
        } else {
            writeln!(writer, "{}", "No clones detected.".green())?;
        }
        return Ok(());
    }

    // Convert to findings for output
    let findings: Vec<CloneFinding> = result
        .pairs
        .iter()
        .flat_map(|pair| {
            // Set confidence based on clone type
            let base_confidence = match pair.clone_type {
                crate::clones::CloneType::Type1 => 95, // Exact copy - safe to auto-fix
                crate::clones::CloneType::Type2 => 85, // Renamed - review recommended
                crate::clones::CloneType::Type3 => 70, // Similar - manual review needed
            };
            vec![
                CloneFinding::from_pair(pair, false, base_confidence), // canonical
                CloneFinding::from_pair(pair, true, base_confidence),  // duplicate
            ]
        })
        .collect();

    // Output results
    if options.json {
        let output = serde_json::to_string_pretty(&findings)?;
        writeln!(writer, "{output}")?;
    } else {
        writeln!(writer, "\n{}", "Clone Detection Results".bold().cyan())?;
        writeln!(writer, "{}\n", "=".repeat(40))?;

        let mut table = Table::new();
        table.set_header(vec![
            "Type",
            "File",
            "Name",
            "Lines",
            "Similarity",
            "Related To",
        ]);

        for finding in &findings {
            if finding.is_duplicate {
                let type_str = finding.clone_type.display_name();

                table.add_row(vec![
                    type_str.yellow().to_string(),
                    finding.file.to_string_lossy().to_string(),
                    finding
                        .name
                        .clone()
                        .unwrap_or_else(|| "<anonymous>".to_owned()),
                    format!("{}-{}", finding.line, finding.end_line),
                    format!("{:.0}%", finding.similarity * 100.0),
                    finding
                        .related_clone
                        .name
                        .clone()
                        .unwrap_or_else(|| "canonical".to_owned()),
                ]);
            }
        }

        writeln!(writer, "{table}")?;
        writeln!(
            writer,
            "\n{}: {} clone pairs found",
            "Summary".bold(),
            result.pairs.len()
        )?;
    }

    // Handle --fix mode
    if options.fix {
        if options.dry_run {
            writeln!(
                writer,
                "\n{}",
                "[DRY-RUN] Would apply the following fixes:".yellow()
            )?;
        } else {
            writeln!(writer, "\n{}", "Applying fixes...".cyan())?;
        }

        // Group by file for batch editing - use HashSet to track seen ranges
        let mut edits_by_file: std::collections::HashMap<PathBuf, Vec<Edit>> =
            std::collections::HashMap::new();
        let mut seen_ranges: std::collections::HashSet<(PathBuf, usize, usize)> =
            std::collections::HashSet::new();

        for finding in &findings {
            if finding.is_duplicate && finding.fix_confidence >= 90 {
                // Use AST-derived byte ranges directly (no manual calculation needed)
                let start_byte = finding.start_byte;
                let end_byte = finding.end_byte;

                // Skip if we've already seen this range (avoid duplicate edits)
                let range_key = (finding.file.clone(), start_byte, end_byte);
                if seen_ranges.contains(&range_key) {
                    continue;
                }
                seen_ranges.insert(range_key);

                if options.dry_run {
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

        if !options.dry_run {
            for (file_path, edits) in edits_by_file {
                if let Some((_, content)) = all_files.iter().find(|(p, _)| p == &file_path) {
                    let mut rewriter = ByteRangeRewriter::new(content.clone());
                    rewriter.add_edits(edits);

                    match rewriter.apply() {
                        Ok(fixed_content) => {
                            fs::write(&file_path, fixed_content)?;
                            writeln!(writer, "  {} {}", "Fixed:".green(), file_path.display())?;
                        }
                        Err(e) => {
                            writeln!(
                                writer,
                                "  {} {}: {}",
                                "Error:".red(),
                                file_path.display(),
                                e
                            )?;
                        }
                    }
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
    use crate::fix::{ByteRangeRewriter, Edit};
    use std::collections::HashMap;

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
    let mut items_by_file: HashMap<PathBuf, Vec<(&str, &crate::visitor::Definition)>> =
        HashMap::new();

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

    if items_by_file.is_empty() {
        writeln!(
            writer,
            "  No items with confidence >= {} to fix.",
            options.min_confidence
        )?;
        return Ok(vec![]);
    }

    // Verbose: show fix statistics
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

        eprintln!("[VERBOSE] Fix Statistics:");
        eprintln!("   Files to modify: {files_count}");
        eprintln!("   Items to remove: {total_items}");
        eprintln!("   Functions: {func_count}");
        eprintln!("   Classes: {class_count}");
        eprintln!("   Imports: {import_count}");

        // Show items below threshold that were skipped
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
            eprintln!(
                "   Skipped (confidence < {}%): {}",
                options.min_confidence, total_skipped
            );
        }
        eprintln!();
    }

    let mut all_results = Vec::new();

    for (file_path, items) in items_by_file {
        let content = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                writeln!(
                    writer,
                    "  {} {}: {}",
                    "Skip:".yellow(),
                    file_path.display(),
                    e
                )?;
                continue;
            }
        };

        // Re-parse to get exact byte ranges
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
                continue;
            }
        };

        let module = parsed.into_syntax();
        let mut edits = Vec::new();
        let mut removed_names = Vec::new();

        for (item_type, def) in &items {
            if let Some((start, end)) = find_def_range(&module.body, &def.simple_name, item_type) {
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
                    edits.push(Edit::delete(start, end));
                    removed_names.push(def.simple_name.clone());
                }
            }
        }

        if !options.dry_run && !edits.is_empty() {
            let mut rewriter = ByteRangeRewriter::new(content);
            rewriter.add_edits(edits);

            match rewriter.apply() {
                Ok(fixed) => {
                    let count = removed_names.len();
                    fs::write(&file_path, fixed)?;
                    writeln!(
                        writer,
                        "  {} {} ({} removed)",
                        "Fixed:".green(),
                        file_path.display(),
                        count
                    )?;
                    all_results.push(FixResult {
                        file: file_path.to_string_lossy().to_string(),
                        items_removed: count,
                        removed_names,
                    });
                }
                Err(e) => {
                    writeln!(
                        writer,
                        "  {} {}: {}",
                        "Error:".red(),
                        file_path.display(),
                        e
                    )?;
                }
            }
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
