//! Clone detection command.

use crate::clones::{
    CloneConfig, CloneDetector, CloneFinding, ClonePair, CloneType, ConfidenceScorer, FixContext,
    NodeKind,
};
use crate::fix::{ByteRangeRewriter, Edit};

use anyhow::Result;
use colored::Colorize;
use comfy_table::{Cell, Color, Table};

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// Options for clone detection
#[derive(Debug, Default)]
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
    // Collect file paths (not content) for OOM-safe processing
    let file_paths: Vec<PathBuf> = paths
        .iter()
        .flat_map(|p| super::utils::find_python_files(p, &options.exclude, options.verbose))
        .collect();

    if file_paths.is_empty() {
        writeln!(writer, "No Python files found.")?;
        return Ok((0, Vec::new()));
    }

    let file_count = file_paths.len();

    // Use OOM-safe detection - processes files in chunks
    let config = CloneConfig::default().with_min_similarity(options.similarity);
    let detector = CloneDetector::with_config(config);
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

    // Lazy load only files involved in clone pairs (OOM-safe for large repos)
    let matched_files = load_matched_files(&result.pairs);

    let findings = generate_clone_findings(&result.pairs, &matched_files, options.with_cst);

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
        }

        writeln!(writer, "{table}")?;
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

/// Load only files that are involved in clone pairs (lazy loading for OOM safety)
fn load_matched_files(pairs: &[ClonePair]) -> Vec<(PathBuf, String)> {
    use std::collections::HashSet;

    // Collect unique file paths from pairs
    let unique_paths: HashSet<PathBuf> = pairs
        .iter()
        .flat_map(|p| [p.instance_a.file.clone(), p.instance_b.file.clone()])
        .collect();

    // Load only these files
    unique_paths
        .into_iter()
        .filter_map(|path| {
            std::fs::read_to_string(&path)
                .ok()
                .map(|content| (path, content))
        })
        .collect()
}

/// Print simple clone stats without file content (OOM-safe)
fn print_clone_stats_simple<W: Write>(
    mut writer: W,
    file_count: usize,
    pairs: &[ClonePair],
) -> Result<()> {
    writeln!(writer, "[VERBOSE] Clone Detection Statistics:")?;
    writeln!(writer, "   Files scanned: {}", file_count)?;
    writeln!(writer, "   Clone pairs found: {}", pairs.len())?;

    let mut type1_count = 0;
    let mut type2_count = 0;
    let mut type3_count = 0;
    for pair in pairs {
        match pair.clone_type {
            CloneType::Type1 => type1_count += 1,
            CloneType::Type2 => type2_count += 1,
            CloneType::Type3 => type3_count += 1,
        }
    }
    writeln!(writer, "   Exact Copies: {type1_count}")?;
    writeln!(writer, "   Renamed Copies: {type2_count}")?;
    writeln!(writer, "   Similar Code: {type3_count}")?;

    if !pairs.is_empty() {
        #[allow(clippy::cast_precision_loss)]
        let avg_similarity: f64 =
            pairs.iter().map(|p| p.similarity).sum::<f64>() / pairs.len() as f64;
        writeln!(
            writer,
            "   Average similarity: {:.0}%",
            avg_similarity * 100.0
        )?;
    }
    writeln!(writer)?;
    Ok(())
}

/// Helper to generate findings from clone pairs.
#[must_use]
pub fn generate_clone_findings(
    pairs: &[ClonePair],
    #[allow(unused_variables)] all_files: &[(PathBuf, String)],
    #[allow(unused_variables)] with_cst: bool,
) -> Vec<CloneFinding> {
    #[cfg(feature = "cst")]
    use crate::cst::{AstCstMapper, CstParser};

    let scorer = ConfidenceScorer::default();

    let mut findings: Vec<CloneFinding> = pairs
        .iter()
        .flat_map(|pair| {
            #[allow(unused_variables)]
            let calc_conf = |inst: &crate::clones::CloneInstance| -> u8 {
                #[allow(unused_mut)]
                let mut ctx = FixContext {
                    same_file: pair.is_same_file(),
                    ..FixContext::default()
                };

                #[cfg(feature = "cst")]
                if with_cst {
                    if let Some((_, content)) = all_files.iter().find(|(p, _)| p == &inst.file) {
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

    for finding in &mut findings {
        let name = finding.name.as_deref().unwrap_or("<anonymous>");
        finding.suggestion = Some(generate_clone_suggestion(
            finding.clone_type,
            finding.node_kind,
            name,
            finding.similarity,
        ));
    }

    // Deduplicate: keep only the highest-similarity finding per (file, line)
    let mut best_by_location: HashMap<(String, usize), CloneFinding> = HashMap::new();

    for finding in findings {
        let key = (finding.file.display().to_string(), finding.line);
        match best_by_location.entry(key) {
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(finding);
            }
            std::collections::hash_map::Entry::Occupied(mut e) => {
                if finding.similarity > e.get().similarity {
                    e.insert(finding);
                }
            }
        }
    }

    best_by_location.into_values().collect()
}

fn apply_clone_fixes_internal<W: Write>(
    mut writer: W,
    findings: &[CloneFinding],
    all_files: &[(PathBuf, String)],
    dry_run: bool,
    #[allow(unused_variables)] with_cst: bool,
) -> Result<()> {
    #[cfg(feature = "cst")]
    use crate::cst::{AstCstMapper, CstParser};

    if dry_run {
        writeln!(
            writer,
            "\n{}",
            "[DRY-RUN] Would apply the following fixes:".yellow()
        )?;
    } else {
        writeln!(writer, "\n{}", "Applying fixes...".cyan())?;
    }

    let mut edits_by_file: HashMap<PathBuf, Vec<Edit>> = HashMap::new();
    let mut seen_ranges: HashSet<(PathBuf, usize, usize)> = HashSet::new();

    for finding in findings {
        if finding.is_duplicate && finding.fix_confidence >= 90 {
            #[allow(unused_mut)]
            let mut start_byte = finding.start_byte;
            #[allow(unused_mut)]
            let mut end_byte = finding.end_byte;

            #[cfg(feature = "cst")]
            if with_cst {
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
