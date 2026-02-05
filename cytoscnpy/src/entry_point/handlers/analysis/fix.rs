use anyhow::Result;

pub(crate) fn run_fix_if_requested<W: std::io::Write>(
    cli_var: &crate::cli::Cli,
    analysis_root: &std::path::Path,
    confidence: u8,
    result: &crate::analyzer::AnalysisResult,
    writer: &mut W,
) -> Result<()> {
    // Handle --fix flag for dead code removal
    // Only run if we didn't also run clones (clones are report-only)
    if cli_var.fix && !cli_var.clones {
        if cli_var.output.verbose && !cli_var.output.json {
            eprintln!("[VERBOSE] Dead code fix mode enabled");
            eprintln!(
                "   Mode: {}",
                if cli_var.apply {
                    "apply changes"
                } else {
                    "dry-run (preview)"
                }
            );
            eprintln!("   Min confidence: {confidence}%");
            eprintln!("   Targets: functions, classes, imports, variables");
            eprintln!("   CST mode: enabled (precise byte ranges)");
            eprintln!();
        }
        let fix_options = crate::commands::DeadCodeFixOptions {
            min_confidence: confidence, // Use configured confidence
            dry_run: !cli_var.apply,
            fix_functions: true,
            fix_classes: true,
            fix_imports: true,
            fix_variables: true,
            verbose: cli_var.output.verbose,
            with_cst: true, // CST is always enabled by default
            analysis_root: analysis_root.to_path_buf(),
        };
        crate::commands::run_fix_deadcode(result, &fix_options, &mut *writer)?;
    }
    Ok(())
}
