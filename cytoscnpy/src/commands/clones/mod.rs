//! Clone detection command.

mod findings;
mod fixes;
mod stats;

use crate::clones::{CloneConfig, CloneDetector, CloneFinding, CloneType, NodeKind};
use anyhow::Result;
use colored::Colorize;
use comfy_table::{Cell, Color, Table};
use std::io::Write;
use std::path::PathBuf;

use fixes::apply_clone_fixes_internal;
use stats::{load_matched_files, print_clone_stats_simple};

pub use findings::generate_clone_findings;

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
    /// Verbose output
    pub verbose: bool,
    /// Use CST for precise fixing (comment preservation)
    pub with_cst: bool,
    /// Progress bar for tracking progress
    pub progress_bar: Option<std::sync::Arc<indicatif::ProgressBar>>,
}

/// Generates context-aware refactoring suggestions for clone findings.
fn generate_clone_suggestion(
    clone_type: CloneType,
    node_kind: NodeKind,
    name: &str,
    similarity: f64,
) -> String {
    let is_init = name == "__init__";
    let is_dunder = name.starts_with("__") && name.ends_with("__");

    match clone_type {
        CloneType::Type1 => match node_kind {
            NodeKind::Class => "Remove duplicate class, import from original".to_owned(),
            NodeKind::Method if is_init => "Extract shared __init__ to base class".to_owned(),
            NodeKind::Method => "Move to base class or mixin".to_owned(),
            NodeKind::Function | NodeKind::AsyncFunction => {
                "Remove duplicate, import from original module".to_owned()
            }
        },
        CloneType::Type2 => match node_kind {
            NodeKind::Class => "Consider inheritance or factory pattern".to_owned(),
            NodeKind::Method if is_init || is_dunder => "Extract to mixin or base class".to_owned(),
            NodeKind::Method => "Parameterize and move to base class".to_owned(),
            NodeKind::Function | NodeKind::AsyncFunction => {
                "Parameterize into single configurable function".to_owned()
            }
        },
        CloneType::Type3 => {
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
    options: &CloneOptions,
    mut writer: W,
) -> Result<(usize, Vec<CloneFinding>)> {
    let file_paths: Vec<PathBuf> =
        super::utils::find_python_files(paths, &options.exclude, options.verbose);

    if file_paths.is_empty() {
        writeln!(writer, "No Python files found.")?;
        return Ok((0, Vec::new()));
    }

    let file_count = file_paths.len();
    let config = CloneConfig::default().with_min_similarity(options.similarity);
    let mut detector = CloneDetector::with_config(config);
    if let Some(ref pb) = options.progress_bar {
        detector.progress_bar = Some(std::sync::Arc::clone(pb));
    }
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
    let findings = generate_clone_findings(&result.pairs, &matched_files, options.with_cst);

    if let Some(ref pb) = options.progress_bar {
        pb.finish_and_clear();
    }

    if options.json {
        let output = serde_json::to_string_pretty(&findings)?;
        writeln!(writer, "{output}")?;
    } else {
        writeln!(writer, "\n{}", "Clone Detection Results".bold().cyan())?;
        writeln!(writer, "{}\n", "=".repeat(40))?;

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

        let display_limit = 100;
        let mut count = 0;
        for finding in &findings {
            if !finding.is_duplicate {
                continue;
            }
            count += 1;
            if count > display_limit {
                continue;
            }

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

        if findings.len() / 2 > display_limit {
            writeln!(
                writer,
                "\n{} Showing first {} results. Use --json to see all {} clone pairs.",
                "Note:".yellow().bold(),
                display_limit,
                findings.len() / 2
            )?;
        }
    }

    if options.fix {
        apply_clone_fixes_internal(
            &mut writer,
            &findings,
            &matched_files,
            options.dry_run,
            options.with_cst,
        )?;
    }

    Ok((result.pairs.len(), findings))
}
