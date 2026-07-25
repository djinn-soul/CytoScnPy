use anyhow::Result;
use std::path::Path;

use super::context::AnalysisContext;
use colored::Colorize;

pub(crate) struct AnalysisRun {
    pub(crate) result: crate::analyzer::AnalysisResult,
    pub(crate) clone_pairs_found: usize,
    pub(crate) clone_similarity: Option<f64>,
    pub(crate) emit_clone_text: bool,
    pub(crate) start_time: std::time::Instant,
}

const fn clone_activation(cli: bool, config: bool, html: bool) -> (bool, bool) {
    let emit_text = cli || config;
    (emit_text || html, emit_text)
}

pub(crate) fn run_analysis<W: std::io::Write>(
    effective_paths: &[std::path::PathBuf],
    analysis_root: &std::path::Path,
    cli_var: &crate::cli::Cli,
    config: &crate::config::Config,
    context: &AnalysisContext,
    writer: &mut W,
) -> Result<AnalysisRun> {
    let start_time = std::time::Instant::now();

    if !context.is_structured && !cfg!(test) {
        // Print active configuration summary
        let mut config_summary = Vec::new();
        if context.secrets {
            config_summary.push("Secrets");
        }
        if context.danger {
            config_summary.push("Danger");
        }
        if context.quality {
            config_summary.push("Quality");
        }
        if context.include_tests {
            config_summary.push("Tests");
        }
        if context.deps {
            config_summary.push("Deps");
        }

        if config_summary.is_empty() {
            config_summary.push("Dead Code Only");
        }

        eprintln!(
            "{} {} (Confidence: {}%)",
            "[INFO] Active Checks:".blue().bold(),
            config_summary.join(", "),
            context.confidence
        );

        crate::output::print_exclusion_list(writer, &context.exclude_folders).ok();
    }

    // Print verbose configuration info (before progress bar)
    if cli_var.output.verbose && !context.is_structured {
        eprintln!("[VERBOSE] CytoScnPy v{}", env!("CARGO_PKG_VERSION"));
        eprintln!("[VERBOSE] Using {} threads", rayon::current_num_threads());
        eprintln!("[VERBOSE] Configuration:");
        eprintln!("   Confidence threshold: {}", context.confidence);
        eprintln!("   Secrets scanning: {}", context.secrets);
        eprintln!("   Danger scanning: {}", context.danger);
        eprintln!("   Quality scanning: {}", context.quality);
        eprintln!("   Include tests: {}", context.include_tests);
        eprintln!("   Target Path: {effective_paths:?}");
        if !context.exclude_folders.is_empty() {
            eprintln!("   Exclude folders: {:?}", context.exclude_folders);
        }
        eprintln!();
    }

    let mut analyzer = crate::analyzer::CytoScnPy::new(
        context.confidence,
        context.secrets,
        context.danger,
        context.quality,
        context.include_tests,
        context.exclude_folders.clone(),
        context.include_folders.clone(),
        context.include_ipynb,
        cli_var.include.ipynb_cells,
        config.clone(),
    )
    .with_verbose(cli_var.output.verbose)
    .with_root(analysis_root.to_path_buf());
    analyzer.whitelist_matcher = build_whitelist_matcher(config, &cli_var.whitelist_files)?;

    // Set debug delay if provided
    if let Some(delay_ms) = cli_var.debug_delay {
        analyzer.debug_delay_ms = Some(delay_ms);
    }

    // Count files first to create progress bar with accurate total
    let total_files = analyzer.count_files(effective_paths);

    // Create progress bar with file count for visual feedback
    let progress: Option<indicatif::ProgressBar> = if context.is_structured {
        None
    } else if total_files > 0 {
        let pb = crate::output::create_progress_bar(total_files as u64);
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template(
                    "{spinner:.cyan} [{bar:40.cyan/blue}] {percent}% - Analyzing source code...",
                )
                .unwrap_or_else(|_| indicatif::ProgressStyle::default_bar())
                .progress_chars("█▓░"),
        );
        Some(pb)
    } else {
        Some(crate::output::create_spinner())
    };

    // Pass progress bar to analyzer for real-time updates
    if let Some(ref pb) = progress {
        analyzer.progress_bar = Some(std::sync::Arc::new(pb.clone()));
    }

    // --- PROCESSING PHASE ---
    // Both analysis and clone detection happen here while the progress bar is active.

    // 1. Run main analysis (dead code, secrets, quality)
    let mut result = analyzer.analyze_paths(effective_paths);

    // 2. Run clone detection if enabled (using the same progress bar)
    let mut clone_pairs_found = 0usize;
    let mut clone_similarity_used = None;
    let clones_from_config = config.cytoscnpy.clones.unwrap_or(false);
    // CLI flag takes priority, then config, then built-in default.
    let clone_similarity = cli_var
        .clone_similarity
        .or(config.cytoscnpy.clone_similarity)
        .unwrap_or(0.8);
    let clone_similarity = crate::cli::validators::validate_similarity(clone_similarity)
        .map_err(anyhow::Error::msg)?;
    #[cfg(feature = "html_report")]
    let html_output_requested = cli_var.output.html;
    #[cfg(not(feature = "html_report"))]
    let html_output_requested = false;
    let (run_clones, emit_clone_text) =
        clone_activation(cli_var.clones, clones_from_config, html_output_requested);

    if run_clones {
        clone_similarity_used = Some(clone_similarity);
        let clone_options = crate::commands::CloneOptions {
            similarity: clone_similarity,
            json: cli_var.output.json,
            fix: false, // Clones are report-only, never auto-fixed
            dry_run: !cli_var.apply,
            exclude: context.exclude_folders.clone().into_iter().collect(),
            include_tests: context.include_tests,
            include_folders: context.include_folders.clone(),
            verbose: cli_var.output.verbose,
            with_cst: true, // CST is always enabled by default
            progress_bar: progress.as_ref().map(|pb| std::sync::Arc::new(pb.clone())),
        };

        // If we have a progress bar, reset it for the clone detection phase
        if let Some(ref pb) = progress {
            pb.set_position(0);
            pb.set_message(""); // Clear message
            pb.set_style(
                indicatif::ProgressStyle::default_bar()
                    .template("{spinner:.cyan} [{bar:40.cyan/blue}] {percent}% - Checking code similarity...")
                    .unwrap_or_else(|_| indicatif::ProgressStyle::default_bar())
                    .progress_chars("█▓░"),
            );
        }

        // Run detection
        // Clone findings are collected here and rendered later with the rest of
        // the report so the main analysis heading always appears first.
        let mut sink = std::io::sink();
        let (_, findings) =
            crate::commands::run_clones(effective_paths, &clone_options, &mut sink)?;

        // The text table shows one row per deduplicated, reportable duplicate,
        // not every internal detector edge. Keep the summary aligned with it.
        clone_pairs_found = findings
            .iter()
            .filter(|finding| finding.is_duplicate)
            .count();
        result.clones = findings;
    }

    // --- COMPLETION ---
    // All background processing is DONE. Hide the progress bar forever.
    if let Some(ref pb) = progress {
        pb.finish_and_clear();
    }

    Ok(AnalysisRun {
        result,
        clone_pairs_found,
        clone_similarity: clone_similarity_used,
        emit_clone_text,
        start_time,
    })
}

fn build_whitelist_matcher(
    config: &crate::config::Config,
    whitelist_files: &[std::path::PathBuf],
) -> Result<Option<crate::whitelist::WhitelistMatcher>> {
    if config.cytoscnpy.whitelist.is_empty() && whitelist_files.is_empty() {
        return Ok(None);
    }

    let mut matcher =
        crate::whitelist::WhitelistMatcher::with_user_entries(config.cytoscnpy.whitelist.clone());

    for path in whitelist_files {
        let whitelist = crate::whitelist::load_whitelist_file(path).map_err(|err| {
            anyhow::anyhow!(
                "failed to load whitelist file '{}': {err}",
                normalize_path_for_error(path)
            )
        })?;
        matcher.add_external(whitelist);
    }

    Ok(Some(matcher))
}

fn normalize_path_for_error(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::{build_whitelist_matcher, clone_activation};

    #[test]
    fn html_only_runs_clones_without_enabling_clone_text() {
        assert_eq!(clone_activation(false, false, true), (true, false));
        assert_eq!(clone_activation(true, false, true), (true, true));
        assert_eq!(clone_activation(false, true, true), (true, true));
    }

    #[test]
    fn build_whitelist_matcher_skips_when_no_entries() {
        let config = crate::config::Config::default();
        let matcher = build_whitelist_matcher(&config, &[]).expect("matcher build should succeed");
        assert!(matcher.is_none());
    }

    #[test]
    fn build_whitelist_matcher_loads_external_whitelist_file() {
        let dir = tempfile::tempdir().expect("tempdir should be created");
        let whitelist_path = dir.path().join("whitelist.py");
        std::fs::write(&whitelist_path, "known_symbol\n")
            .expect("whitelist file should be written");

        let config = crate::config::Config::default();
        let files = vec![whitelist_path];
        let matcher = build_whitelist_matcher(&config, &files)
            .expect("matcher build should succeed")
            .expect("matcher should be present");

        assert!(matcher.is_whitelisted("known_symbol", None));
    }
}
