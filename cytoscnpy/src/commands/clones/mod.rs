//! Clone detection command.

mod findings;
mod stats;
mod suggestions;

use crate::clones::{CloneConfig, CloneDetector, CloneFinding};
use anyhow::Result;
use colored::Colorize;
use comfy_table::{Cell, Color, Table};
use std::io::Write;
use std::path::PathBuf;

use stats::{load_matched_files, print_clone_stats_simple};
use suggestions::generate_clone_suggestion;

pub use findings::{generate_clone_findings, generate_clone_findings_with_thresholds};

/// Options for clone detection
#[derive(Debug, Default, Clone)]
#[allow(clippy::struct_excessive_bools)]
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
    /// Include test files in clone detection
    pub include_tests: bool,
    /// Folders to force-include during file discovery
    pub include_folders: Vec<String>,
    /// Verbose output
    pub verbose: bool,
    /// Use CST for precise fixing (comment preservation)
    pub with_cst: bool,
    /// Progress bar for tracking progress
    pub progress_bar: Option<std::sync::Arc<indicatif::ProgressBar>>,
}

fn create_detector(options: &CloneOptions) -> Result<CloneDetector> {
    let config = CloneConfig::default()
        .with_min_similarity(options.similarity)
        // Command-level discovery already applies root-relative test filtering.
        .with_tests(true);
    let mut detector = CloneDetector::with_config(config).map_err(anyhow::Error::msg)?;
    if let Some(ref pb) = options.progress_bar {
        detector.progress_bar = Some(std::sync::Arc::clone(pb));
    }
    Ok(detector)
}

pub(crate) fn print_clone_results<W: Write>(
    writer: &mut W,
    findings: &[CloneFinding],
) -> Result<()> {
    if findings.is_empty() {
        writeln!(writer, "{}", "No clones detected.".green())?;
        return Ok(());
    }

    writeln!(writer, "\n{}", "Clone Detection Results".bold().cyan())?;
    writeln!(writer, "{}\n", "=".repeat(40))?;
    write_clone_table(writer, findings)?;
    Ok(())
}

fn write_clone_table<W: Write>(writer: &mut W, findings: &[CloneFinding]) -> Result<()> {
    let mut table = Table::new();
    table
        .load_preset(comfy_table::presets::UTF8_FULL)
        .set_header(vec![
            "Type",
            "Name",
            "Related To",
            "Location",
            "Similarity",
            "Suggestion",
        ]);

    let duplicate_count = findings.iter().filter(|f| f.is_duplicate).count();
    let display_limit = 100;
    for finding in findings
        .iter()
        .filter(|f| f.is_duplicate)
        .take(display_limit)
    {
        let type_str = finding.clone_type.display_name();
        let name = finding
            .name
            .clone()
            .unwrap_or_else(|| "<anonymous>".to_owned());
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
        let suggestion = generate_clone_suggestion(
            finding.clone_type,
            finding.node_kind,
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

    writeln!(writer, "{table}")?;
    if duplicate_count > display_limit {
        writeln!(
            writer,
            "\n{} Showing first {} results. Use --json to see all {} clone findings.",
            "Note:".yellow().bold(),
            display_limit,
            duplicate_count
        )?;
    }

    Ok(())
}

/// Executes clone detection analysis.
///
/// # Errors
///
/// Returns an error if file I/O fails or analysis fails.
///
/// Returns the number of clone pairs found.
pub fn run_clones<W: Write>(
    paths: &[PathBuf],
    options: &CloneOptions,
    mut writer: W,
) -> Result<(usize, Vec<CloneFinding>)> {
    anyhow::ensure!(
        options.similarity.is_finite() && (0.0..=1.0).contains(&options.similarity),
        "clone similarity must be a finite value between 0.0 and 1.0"
    );
    anyhow::ensure!(
        !options.fix,
        "clone auto-fix is disabled because deleting a duplicate definition cannot safely rewrite its callers"
    );
    let file_paths: Vec<PathBuf> = super::utils::find_python_files_with_options(
        paths,
        &options.exclude,
        &options.include_folders,
        options.include_tests,
        options.verbose,
    );

    if file_paths.is_empty() {
        if options.json {
            writeln!(writer, "[]")?;
        } else {
            writeln!(writer, "No Python files found.")?;
        }
        return Ok((0, Vec::new()));
    }

    let file_count = file_paths.len();
    let detector = create_detector(options)?;
    let (auto_fix_threshold, suggest_threshold) = detector.confidence_thresholds();
    let result = detector.detect_from_paths(&file_paths);

    if !options.json && options.verbose {
        print_clone_stats_simple(&mut writer, file_count, &result.pairs)?;
    }

    if result.pairs.is_empty() {
        if options.json {
            writeln!(writer, "[]")?;
        } else {
            writeln!(writer, "{}", "No clones detected.".green())?;
        }
        return Ok((0, Vec::new()));
    }

    let matched_files = load_matched_files(&result.pairs);
    let findings = generate_clone_findings_with_thresholds(
        &result.pairs,
        &matched_files,
        options.with_cst,
        auto_fix_threshold,
        suggest_threshold,
    );

    if let Some(ref pb) = options.progress_bar {
        pb.finish_and_clear();
    }

    if options.json {
        let output = serde_json::to_string_pretty(&findings)?;
        writeln!(writer, "{output}")?;
    } else {
        print_clone_results(&mut writer, &findings)?;
    }

    Ok((result.pairs.len(), findings))
}
