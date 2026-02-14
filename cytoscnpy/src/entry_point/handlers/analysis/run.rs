use anyhow::Result;

use super::context::AnalysisContext;
use colored::Colorize;

pub(crate) struct AnalysisRun {
    pub(crate) result: crate::analyzer::AnalysisResult,
    pub(crate) clone_pairs_found: usize,
    pub(crate) start_time: std::time::Instant,
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
    if cli_var.clones || cli_var.output.html {
        let clone_options = crate::commands::CloneOptions {
            similarity: cli_var.clone_similarity,
            json: cli_var.output.json,
            fix: false, // Clones are report-only, never auto-fixed
            dry_run: !cli_var.apply,
            exclude: context.exclude_folders.clone().into_iter().collect(),
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
        let (count, findings) = if context.is_structured || !cli_var.clones {
            // Suppress clone table unless explicitly requested (or for structured output)
            let mut sink = std::io::sink();
            crate::commands::run_clones(effective_paths, &clone_options, &mut sink)?
        } else {
            // Text output: print findings to stdout (writer)
            crate::commands::run_clones(effective_paths, &clone_options, &mut *writer)?
        };

        clone_pairs_found = count;
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
        start_time,
    })
}
