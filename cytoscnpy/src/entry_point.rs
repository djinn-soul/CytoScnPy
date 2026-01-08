use crate::cli::{Cli, Commands};
use anyhow::Result;
use clap::Parser;
use ruff_python_ast::{Expr, Stmt};
use rustc_hash::FxHashSet;
/// Detects if `__name__ == "__main__"` blocks exist and extracts function calls from them.
///
/// This is crucial for correctly identifying entry points in Python scripts.
/// Functions called within this block should be considered "used" because they are the starting points of execution.
pub fn detect_entry_point_calls(stmts: &[Stmt]) -> FxHashSet<String> {
    let mut entry_point_calls = FxHashSet::default();

    // Iterate through all top-level statements in the module
    for stmt in stmts {
        // Check if the statement is the main guard (if __name__ == "__main__")
        if is_main_guard(stmt) {
            // If it is, we need to look inside the `if` block.
            if let Stmt::If(if_stmt) = stmt {
                // Iterate through statements inside the block
                for body_stmt in &if_stmt.body {
                    // Collect all function calls invoked in this block
                    collect_function_calls(body_stmt, &mut entry_point_calls);
                }
            }
        }
    }

    entry_point_calls
}

/// Checks if this statement is an `if __name__ == "__main__"` guard.
///
/// This looks for a specific AST pattern: an If statement where the test is a comparison.
fn is_main_guard(stmt: &Stmt) -> bool {
    if let Stmt::If(if_stmt) = stmt {
        // Check if the test condition is a comparison
        if let Expr::Compare(compare) = &*if_stmt.test {
            // We expect a single comparison (one operator, one comparator)
            // Check for: __name__ == "__main__" OR "__main__" == __name__
            if compare.ops.len() == 1 && compare.comparators.len() == 1 {
                let left = &*compare.left;
                let right = &compare.comparators[0];

                // Check both orders of comparison
                return is_name_dunder(left) && is_main_string(right)
                    || is_name_dunder(right) && is_main_string(left);
            }
        }
    }
    false
}

/// Checks if an expression matches the variable name `__name__`.
///
/// This is a helper for `is_main_guard`.
fn is_name_dunder(expr: &Expr) -> bool {
    if let Expr::Name(name_expr) = expr {
        return name_expr.id.as_str() == "__name__";
    }
    false
}

/// Checks if an expression is the string literal `"__main__"`.
///
/// This is a helper for `is_main_guard`.
fn is_main_string(expr: &Expr) -> bool {
    if let Expr::StringLiteral(string_lit) = expr {
        return string_lit.value.to_string() == "__main__";
    }
    false
}

/// Recursively collects all function calls from a statement.
///
/// This function traverses nested statements (like loops and nested ifs)
/// to find where functions are being called.
fn collect_function_calls(stmt: &Stmt, calls: &mut FxHashSet<String>) {
    match stmt {
        // Handle simple expressions: func()
        Stmt::Expr(expr_stmt) => {
            collect_calls_from_expr(&expr_stmt.value, calls);
        }
        // Handle assignments: x = func()
        Stmt::Assign(assign) => {
            collect_calls_from_expr(&assign.value, calls);
        }
        // Handle nested if statements
        Stmt::If(if_stmt) => {
            for body_stmt in &if_stmt.body {
                collect_function_calls(body_stmt, calls);
            }
            for else_stmt in &if_stmt.elif_else_clauses {
                for body_stmt in &else_stmt.body {
                    collect_function_calls(body_stmt, calls);
                }
            }
        }
        // Handle for loops
        Stmt::For(for_stmt) => {
            // Check the iterator expression: for x in get_items()
            collect_calls_from_expr(&for_stmt.iter, calls);
            // Check the body
            for body_stmt in &for_stmt.body {
                collect_function_calls(body_stmt, calls);
            }
        }
        // Handle while loops
        Stmt::While(while_stmt) => {
            for body_stmt in &while_stmt.body {
                collect_function_calls(body_stmt, calls);
            }
        }
        _ => {}
    }
}

/// Extracts function names from expression nodes.
///
/// This looks into function calls, attribute accesses (methods), and binary operations.
fn collect_calls_from_expr(expr: &Expr, calls: &mut FxHashSet<String>) {
    match expr {
        // Found a call: func(...)
        Expr::Call(call) => {
            // Get the name of the function being called
            if let Some(name) = get_call_name(&call.func) {
                calls.insert(name);
            }
            // Recursively check arguments, they might contain calls too: func(other_func())
            for arg in &call.arguments.args {
                collect_calls_from_expr(arg, calls);
            }
        }
        // Handle attribute access: obj.prop
        // This might be part of a call chain or just attribute access.
        Expr::Attribute(attr) => {
            collect_calls_from_expr(&attr.value, calls);
        }
        // Handle binary operations: func1() + func2()
        Expr::BinOp(binop) => {
            collect_calls_from_expr(&binop.left, calls);
            collect_calls_from_expr(&binop.right, calls);
        }
        _ => {}
    }
}

/// Extracts the function name from a call expression.
///
/// Returns `Some(name)` if it's a simple name or attribute access.
fn get_call_name(expr: &Expr) -> Option<String> {
    match expr {
        // Simple function call: name()
        Expr::Name(name) => Some(name.id.to_string()),
        // Method call: obj.method()
        Expr::Attribute(attr) => {
            // For method calls, we return the method name part.
            Some(attr.attr.to_string())
        }
        _ => None,
    }
}

// ============================================================================
// Subcommand Helper Functions
// ============================================================================

/// Resolves subcommand paths, defaulting to `.` if empty, and checks existence.
/// Returns `Ok(Vec<PathBuf>)` if all paths exist, `Err(1)` if any doesn't.
fn resolve_subcommand_paths(
    paths: Vec<std::path::PathBuf>,
    root: Option<std::path::PathBuf>,
) -> Result<Vec<std::path::PathBuf>, i32> {
    // If --root is provided, it's the only path we care about
    let final_paths = if let Some(r) = root {
        vec![r]
    } else if paths.is_empty() {
        vec![std::path::PathBuf::from(".")]
    } else {
        paths
    };

    for path in &final_paths {
        if !path.exists() {
            eprintln!(
                "Error: The file or directory '{}' does not exist.",
                path.display()
            );
            return Err(1);
        }
    }
    Ok(final_paths)
}

/// Validates and prepares an output file path for a subcommand.
/// Returns the validated path string, or propagates errors.
fn prepare_output_path(
    output_file: Option<String>,
    analysis_root: &std::path::Path,
) -> Result<Option<String>> {
    match output_file {
        Some(out) => Ok(Some(
            crate::utils::validate_output_path(std::path::Path::new(&out), Some(analysis_root))?
                .to_string_lossy()
                .to_string(),
        )),
        None => Ok(None),
    }
}

/// Merges subcommand-specific excludes with global excludes from config.
fn merge_excludes(subcommand_excludes: Vec<String>, global_excludes: &[String]) -> Vec<String> {
    let mut merged = subcommand_excludes;
    merged.extend(global_excludes.iter().cloned());
    merged
}

/// Validates that --root and positional paths are not used together.
/// Returns Ok(()) if valid, Err(1) if both are provided.
fn validate_path_args(args: &crate::cli::PathArgs) -> Result<(), i32> {
    if args.root.is_some() && !args.paths.is_empty() {
        eprintln!("Error: Cannot use both --root and positional path arguments");
        return Err(1);
    }
    Ok(())
}

/// Runs the analyzer (or other commands) with the given arguments.
///
/// # Errors
///
/// Returns an error if argument parsing fails, or if the command execution fails.
#[allow(clippy::too_many_lines)]
pub fn run_with_args(args: Vec<String>) -> Result<i32> {
    run_with_args_to(args, &mut std::io::stdout())
}

/// Run CytoScnPy with the given arguments, writing output to the specified writer.
///
/// This is the testable version of `run_with_args` that allows output capture.
///
/// # Errors
///
/// Returns an error if argument parsing fails, or if the command execution fails.
#[allow(clippy::too_many_lines)]
pub fn run_with_args_to<W: std::io::Write>(args: Vec<String>, writer: &mut W) -> Result<i32> {
    let mut program_args = vec!["cytoscnpy".to_owned()];
    program_args.extend(args);
    let cli_var = match Cli::try_parse_from(program_args) {
        Ok(c) => c,
        Err(e) => {
            match e.kind() {
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
                    // Let clap print help/version as intended, but captured by redirect
                    write!(writer, "{e}")?;
                    writer.flush()?; // Flush to ensure output is visible (required for pytest)
                    return Ok(0);
                }
                _ => {
                    eprint!("{e}");
                    return Ok(1);
                }
            }
        }
    };

    // Explicit runtime validation for mutual exclusivity of --root and positional paths
    if let Err(code) = validate_path_args(&cli_var.paths) {
        return Ok(code);
    }

    // Logic to determine analysis_root if not explicitly provided via --root
    // We look at both global paths and subcommand paths to see if any are absolute.
    let mut all_target_paths = cli_var.paths.paths.clone();
    if let Some(ref command) = cli_var.command {
        match command {
            Commands::Raw { common, .. }
            | Commands::Cc { common, .. }
            | Commands::Hal { common, .. }
            | Commands::Mi { common, .. } => {
                if let Some(r) = &common.paths.root {
                    all_target_paths.push(r.clone());
                } else {
                    all_target_paths.extend(common.paths.paths.iter().cloned());
                }
            }
            Commands::Files { args, .. } => {
                if let Some(r) = &args.paths.root {
                    all_target_paths.push(r.clone());
                } else {
                    all_target_paths.extend(args.paths.paths.iter().cloned());
                }
            }
            Commands::Stats { paths, .. } => {
                if let Some(r) = &paths.root {
                    all_target_paths.push(r.clone());
                } else {
                    all_target_paths.extend(paths.paths.iter().cloned());
                }
            }
            _ => {}
        }
    }

    let (effective_paths, analysis_root): (Vec<std::path::PathBuf>, std::path::PathBuf) =
        if let Some(ref root) = cli_var.paths.root {
            // --root was provided: use it as the analysis path AND containment boundary
            (vec![root.clone()], root.clone())
        } else {
            let mut root = std::path::PathBuf::from(".");
            if let Some(first_abs) = all_target_paths.iter().find(|p| p.is_absolute()) {
                // Determine common ancestor for absolute paths
                let mut common = if first_abs.is_dir() {
                    first_abs.clone()
                } else {
                    first_abs
                        .parent()
                        .map(std::path::Path::to_path_buf)
                        .unwrap_or_else(|| first_abs.clone())
                };

                for path in all_target_paths.iter().filter(|p| p.is_absolute()) {
                    while !path.starts_with(&common) {
                        if let Some(parent) = common.parent() {
                            common = parent.to_path_buf();
                        } else {
                            break;
                        }
                    }
                }
                root = common;
            }

            let paths = if cli_var.paths.paths.is_empty() {
                // If it's a subcommand call, we might not have global paths.
                // But loading config from the first subcommand path is better than ".".
                if let Some(first) = all_target_paths.first() {
                    vec![first.clone()]
                } else {
                    vec![std::path::PathBuf::from(".")]
                }
            } else {
                cli_var.paths.paths.clone()
            };
            (paths, root)
        };

    // Load config from the first effective path or current directory
    let config_path = effective_paths
        .first()
        .map_or(std::path::Path::new("."), std::path::PathBuf::as_path);
    let config = crate::config::Config::load_from_path(config_path);

    let mut exclude_folders = config.cytoscnpy.exclude_folders.clone().unwrap_or_default();
    exclude_folders.extend(cli_var.exclude_folders.clone());

    // Calculate include_tests once - reused by both subcommands and main analyzer
    let include_tests =
        cli_var.include.include_tests || config.cytoscnpy.include_tests.unwrap_or(false);

    // Calculate include_folders once - reused by both subcommands and main analyzer
    let mut include_folders = config.cytoscnpy.include_folders.clone().unwrap_or_default();
    include_folders.extend(cli_var.include_folders.clone());

    // Print deprecation warning if old keys are used in config
    if config.cytoscnpy.uses_deprecated_keys() && !cli_var.output.json {
        use colored::Colorize;
        eprintln!(
            "{}",
            "WARNING: 'complexity' and 'nesting' are deprecated in configuration. Please use 'max_complexity' and 'max_nesting' instead."
                .yellow()
                .bold()
        );
    }

    if cli_var.output.verbose && !cli_var.output.json {
        eprintln!("[VERBOSE] CytoScnPy v{}", env!("CARGO_PKG_VERSION"));
        eprintln!("[VERBOSE] Using {} threads", rayon::current_num_threads());
        if let Some(ref command) = cli_var.command {
            eprintln!("[VERBOSE] Executing subcommand: {command:?}");
        }
        eprintln!("[VERBOSE] Global Excludes: {exclude_folders:?}");
        eprintln!();
    }

    if let Some(command) = cli_var.command {
        match command {
            Commands::Raw { common, summary } => {
                if let Err(code) = validate_path_args(&common.paths) {
                    return Ok(code);
                }
                let paths = match resolve_subcommand_paths(common.paths.paths, common.paths.root) {
                    Ok(p) => p,
                    Err(code) => return Ok(code),
                };
                let exclude = merge_excludes(common.exclude, &exclude_folders);
                let output_file = prepare_output_path(common.output_file, &analysis_root)?;
                crate::commands::run_raw(
                    &paths,
                    common.json,
                    exclude,
                    common.ignore,
                    summary,
                    output_file,
                    cli_var.output.verbose,
                    writer,
                )?;
            }
            Commands::Cc {
                common,
                rank,
                average,
                total_average,
                show_complexity,
                order,
                no_assert,
                xml,
                fail_threshold,
            } => {
                if let Err(code) = validate_path_args(&common.paths) {
                    return Ok(code);
                }
                let paths = match resolve_subcommand_paths(common.paths.paths, common.paths.root) {
                    Ok(p) => p,
                    Err(code) => return Ok(code),
                };
                let exclude = merge_excludes(common.exclude, &exclude_folders);
                let output_file = prepare_output_path(common.output_file, &analysis_root)?;
                crate::commands::run_cc(
                    &paths,
                    crate::commands::CcOptions {
                        json: common.json,
                        exclude,
                        ignore: common.ignore,
                        min_rank: rank.min_rank,
                        max_rank: rank.max_rank,
                        average,
                        total_average,
                        show_complexity,
                        order,
                        no_assert,
                        xml,
                        fail_threshold,
                        output_file,
                        verbose: cli_var.output.verbose,
                    },
                    writer,
                )?;
            }
            Commands::Hal { common, functions } => {
                if let Err(code) = validate_path_args(&common.paths) {
                    return Ok(code);
                }
                let paths = match resolve_subcommand_paths(common.paths.paths, common.paths.root) {
                    Ok(p) => p,
                    Err(code) => return Ok(code),
                };
                let exclude = merge_excludes(common.exclude, &exclude_folders);
                let output_file = prepare_output_path(common.output_file, &analysis_root)?;
                crate::commands::run_hal(
                    &paths,
                    common.json,
                    exclude,
                    common.ignore,
                    functions,
                    output_file,
                    cli_var.output.verbose,
                    writer,
                )?;
            }
            Commands::Mi {
                common,
                rank,
                multi,
                show,
                average,
                fail_threshold,
            } => {
                if let Err(code) = validate_path_args(&common.paths) {
                    return Ok(code);
                }
                let paths = match resolve_subcommand_paths(common.paths.paths, common.paths.root) {
                    Ok(p) => p,
                    Err(code) => return Ok(code),
                };
                let exclude = merge_excludes(common.exclude, &exclude_folders);
                let output_file = prepare_output_path(common.output_file, &analysis_root)?;
                crate::commands::run_mi(
                    &paths,
                    crate::commands::MiOptions {
                        json: common.json,
                        exclude,
                        ignore: common.ignore,
                        min_rank: rank.min_rank,
                        max_rank: rank.max_rank,
                        multi,
                        show,
                        average,
                        fail_threshold,
                        output_file,
                        verbose: cli_var.output.verbose,
                    },
                    writer,
                )?;
            }
            Commands::McpServer => {
                // MCP server is handled in cytoscnpy-cli main.rs before calling entry_point
                // This should never be reached, but we need the match arm for exhaustiveness
                eprintln!("Error: mcp-server command should be handled by cytoscnpy-cli directly.");
                eprintln!("If you're seeing this, please use the cytoscnpy-cli binary.");
                return Ok(1);
            }
            Commands::Stats {
                paths,
                all,
                secrets,
                danger,
                quality,
                json,
                output,
                exclude,
            } => {
                if let Err(code) = validate_path_args(&paths) {
                    return Ok(code);
                }
                // Use --root if provided, otherwise use positional paths
                let effective_paths = match resolve_subcommand_paths(paths.paths, paths.root) {
                    Ok(p) => p,
                    Err(code) => return Ok(code),
                };
                let exclude = merge_excludes(exclude, &exclude_folders);

                let quality_count = crate::commands::run_stats_v2(
                    &analysis_root,
                    &effective_paths,
                    all,
                    secrets || config.cytoscnpy.secrets.unwrap_or(false),
                    danger || config.cytoscnpy.danger.unwrap_or(false),
                    quality || config.cytoscnpy.quality.unwrap_or(false),
                    json,
                    output,
                    &exclude,
                    include_tests,
                    &include_folders,
                    cli_var.output.verbose,
                    config.clone(),
                    writer,
                )?;

                // Quality gate check (--fail-on-quality) for stats subcommand
                if cli_var.output.fail_on_quality && quality_count > 0 {
                    if !cli_var.output.json {
                        eprintln!("\n[GATE] Quality issues: {quality_count} found - FAILED");
                    }
                    return Ok(1);
                }
            }
            Commands::Files { args } => {
                if let Err(code) = validate_path_args(&args.paths) {
                    return Ok(code);
                }
                let paths = match resolve_subcommand_paths(args.paths.paths, args.paths.root) {
                    Ok(p) => p,
                    Err(code) => return Ok(code),
                };
                let exclude = merge_excludes(args.exclude, &exclude_folders);
                crate::commands::run_files(
                    &paths,
                    args.json,
                    &exclude,
                    cli_var.output.verbose,
                    writer,
                )?;
            }
        }
        Ok(0)
    } else {
        for path in &effective_paths {
            if !path.exists() {
                eprintln!(
                    "Error: The file or directory '{}' does not exist.",
                    path.display()
                );
                return Ok(1);
            }
        }
        let confidence = cli_var
            .confidence
            .or(config.cytoscnpy.confidence)
            .unwrap_or(60);
        let secrets = cli_var.scan.secrets || config.cytoscnpy.secrets.unwrap_or(false);
        let danger = cli_var.scan.danger || config.cytoscnpy.danger.unwrap_or(false);

        // Auto-enable quality mode when:
        // - --quality flag is passed
        // - quality is enabled in config
        // - --min-mi or --max-complexity thresholds are set
        // - --html flag is passed (for dashboard metrics)
        #[cfg(feature = "html_report")]
        let html_enabled = cli_var.output.html;
        #[cfg(not(feature = "html_report"))]
        let html_enabled = false;

        let quality = cli_var.scan.quality
            || config.cytoscnpy.quality.unwrap_or(false)
            || cli_var.min_mi.is_some()
            || cli_var.max_complexity.is_some()
            || config.cytoscnpy.min_mi.is_some()
            || config.cytoscnpy.max_complexity.is_some()
            || html_enabled;

        // Re-declare exclude_folders for this scope (extends global with CLI args)
        let mut exclude_folders = config.cytoscnpy.exclude_folders.clone().unwrap_or_default();
        exclude_folders.extend(cli_var.exclude_folders);

        // Re-declare include_folders for this scope (extends global with CLI args)
        let mut include_folders = config.cytoscnpy.include_folders.clone().unwrap_or_default();
        include_folders.extend(cli_var.include_folders);

        if !cli_var.output.json {
            crate::output::print_exclusion_list(writer, &exclude_folders).ok();
        }

        // Print verbose configuration info (before progress bar)
        if cli_var.output.verbose && !cli_var.output.json {
            eprintln!("[VERBOSE] CytoScnPy v{}", env!("CARGO_PKG_VERSION"));
            eprintln!("[VERBOSE] Using {} threads", rayon::current_num_threads());
            eprintln!("[VERBOSE] Configuration:");
            eprintln!("   Confidence threshold: {confidence}");
            eprintln!("   Secrets scanning: {secrets}");
            eprintln!("   Danger scanning: {danger}");
            eprintln!("   Quality scanning: {quality}");
            eprintln!("   Include tests: {include_tests}");
            eprintln!("   Paths: {effective_paths:?}");
            if !exclude_folders.is_empty() {
                eprintln!("   Exclude folders: {exclude_folders:?}");
            }
            eprintln!();
        }

        let mut analyzer = crate::analyzer::CytoScnPy::new(
            confidence,
            secrets,
            danger,
            quality,
            include_tests,
            exclude_folders.clone(),
            include_folders,
            cli_var.include.include_ipynb,
            cli_var.include.ipynb_cells,
            danger, // taint is now automatically enabled with --danger
            config.clone(),
        )
        .with_verbose(cli_var.output.verbose)
        .with_root(analysis_root.clone());

        // Set debug delay if provided
        if let Some(delay_ms) = cli_var.debug_delay {
            analyzer.debug_delay_ms = Some(delay_ms);
        }

        // Count files first to create progress bar with accurate total
        let total_files = analyzer.count_files(&effective_paths);

        // Create progress bar with file count for visual feedback
        let progress: Option<indicatif::ProgressBar> = if cli_var.output.json {
            None
        } else if total_files > 0 {
            Some(crate::output::create_progress_bar(total_files as u64))
        } else {
            Some(crate::output::create_spinner())
        };

        // Pass progress bar to analyzer for real-time updates
        if let Some(ref pb) = progress {
            analyzer.progress_bar = Some(std::sync::Arc::new(pb.clone()));
        }

        let start_time = std::time::Instant::now();

        let mut result = analyzer.analyze_paths(&effective_paths);

        // If --no-dead flag is set, clear dead code detection results
        // (only show security/quality scans)
        if cli_var.scan.no_dead {
            result.unused_functions.clear();
            result.unused_methods.clear();
            result.unused_classes.clear();
            result.unused_imports.clear();
            result.unused_variables.clear();
            result.unused_parameters.clear();
        }

        if let Some(p) = progress {
            p.finish_and_clear();
        }

        // Print verbose timing info
        if cli_var.output.verbose && !cli_var.output.json {
            let elapsed = start_time.elapsed();
            eprintln!(
                "[VERBOSE] Analysis completed in {:.2}s",
                elapsed.as_secs_f64()
            );
            eprintln!("   Files analyzed: {}", result.analysis_summary.total_files);
            eprintln!(
                "   Lines analyzed: {}",
                result.analysis_summary.total_lines_analyzed
            );
            eprintln!("[VERBOSE] Findings breakdown:");
            eprintln!(
                "   Unreachable functions: {}",
                result.unused_functions.len()
            );
            eprintln!("   Unreachable methods: {}", result.unused_methods.len());
            eprintln!("   Unused classes: {}", result.unused_classes.len());
            eprintln!("   Unused imports: {}", result.unused_imports.len());
            eprintln!("   Unused variables: {}", result.unused_variables.len());
            eprintln!("   Unused parameters: {}", result.unused_parameters.len());
            eprintln!("   Parse errors: {}", result.parse_errors.len());

            // Show files with most issues
            let mut file_counts: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for item in &result.unused_functions {
                *file_counts
                    .entry(crate::utils::normalize_display_path(&item.file))
                    .or_insert(0) += 1;
            }
            for item in &result.unused_methods {
                *file_counts
                    .entry(crate::utils::normalize_display_path(&item.file))
                    .or_insert(0) += 1;
            }
            for item in &result.unused_classes {
                *file_counts
                    .entry(crate::utils::normalize_display_path(&item.file))
                    .or_insert(0) += 1;
            }
            for item in &result.unused_imports {
                *file_counts
                    .entry(crate::utils::normalize_display_path(&item.file))
                    .or_insert(0) += 1;
            }
            for item in &result.unused_variables {
                *file_counts
                    .entry(crate::utils::normalize_display_path(&item.file))
                    .or_insert(0) += 1;
            }
            for item in &result.unused_parameters {
                *file_counts
                    .entry(crate::utils::normalize_display_path(&item.file))
                    .or_insert(0) += 1;
            }

            if !file_counts.is_empty() {
                let mut sorted: Vec<_> = file_counts.into_iter().collect();
                sorted.sort_by(|a, b| b.1.cmp(&a.1));
                eprintln!("[VERBOSE] Files with most issues:");
                for (file, count) in sorted.iter().take(5) {
                    eprintln!("   {count:3} issues: {file}");
                }
            }
            eprintln!();
        }

        // Print JSON or report (but defer the summary and time for combined output later)
        if cli_var.output.json {
            // If clones are enabled, include clone_findings in the JSON output
            if cli_var.clones {
                // Run clone detection
                let clone_findings = run_clone_detection_for_json(
                    &effective_paths,
                    cli_var.clone_similarity,
                    &exclude_folders,
                    cli_var.output.verbose,
                );

                // Create combined output with clone_findings
                #[derive(serde::Serialize)]
                struct CombinedOutput<'a> {
                    #[serde(flatten)]
                    analysis: &'a crate::analyzer::AnalysisResult,
                    clone_findings: Vec<crate::clones::CloneFinding>,
                }

                let combined = CombinedOutput {
                    analysis: &result,
                    clone_findings,
                };
                writeln!(writer, "{}", serde_json::to_string_pretty(&combined)?)?;
            } else {
                writeln!(writer, "{}", serde_json::to_string_pretty(&result)?)?;
            }
        } else {
            // Determine if we should show standard CLI output
            #[cfg(feature = "html_report")]
            let show_cli = !cli_var.output.html;
            #[cfg(not(feature = "html_report"))]
            let show_cli = true;

            if show_cli {
                if cli_var.output.quiet {
                    crate::output::print_report_quiet(writer, &result)?;
                } else {
                    crate::output::print_report(writer, &result)?;
                }
            }

            // Track clone count for combined summary
            let mut clone_pairs_found = 0usize;

            // Handle --clones flag (or implicit execution for HTML report)
            if cli_var.clones || cli_var.output.html {
                if cli_var.output.verbose && !cli_var.output.json {
                    eprintln!("[VERBOSE] Clone detection enabled");
                    eprintln!(
                        "   Similarity threshold: {:.0}%",
                        cli_var.clone_similarity * 100.0
                    );
                    if cli_var.fix {
                        eprintln!(
                            "   Fix mode: {} (confidence >= 90%)",
                            if cli_var.apply {
                                "apply"
                            } else {
                                "dry-run (preview)"
                            }
                        );
                    }
                    eprintln!();
                }
                let clone_options = crate::commands::CloneOptions {
                    similarity: cli_var.clone_similarity,
                    json: cli_var.output.json,
                    fix: false, // Clones are report-only, never auto-fixed
                    dry_run: !cli_var.apply,
                    exclude: exclude_folders.clone().into_iter().collect(),
                    verbose: cli_var.output.verbose,
                    with_cst: true, // CST is always enabled by default
                };

                let (count, findings) = if cli_var.clones {
                    // Explicit run: print to stdout
                    crate::commands::run_clones(&effective_paths, &clone_options, &mut *writer)?
                } else {
                    // Implicit run for HTML: suppress output
                    let mut sink = std::io::sink();
                    crate::commands::run_clones(&effective_paths, &clone_options, &mut sink)?
                };

                clone_pairs_found = count;
                result.clones = findings;
            }

            // Print summary and time (only for non-JSON output)
            // Note: In quiet mode, print_report_quiet already prints the summary,
            // so we only print here if clone pairs were found (to add the clone count)
            if !cli_var.output.json {
                let total = result.unused_functions.len()
                    + result.unused_methods.len()
                    + result.unused_imports.len()
                    + result.unused_parameters.len()
                    + result.unused_classes.len()
                    + result.unused_variables.len();
                let security = result.danger.len() + result.secrets.len() + result.quality.len();

                // Only print summary if either:
                // 1. Not in quiet mode (print_report doesn't include summary)
                // 2. Clone pairs were found (need to add clone count to summary)
                if clone_pairs_found > 0 {
                    writeln!(writer,
                    "\n[SUMMARY] {total} unused code issues, {security} security/quality issues, {clone_pairs_found} clone pairs"
                )?;
                } else if !cli_var.output.quiet {
                    writeln!(writer,
                    "\n[SUMMARY] {total} unused code issues, {security} security/quality issues"
                )?;
                }

                let elapsed = start_time.elapsed();
                writeln!(
                    writer,
                    "\n[TIME] Completed in {:.2}s",
                    elapsed.as_secs_f64()
                )?;
            }
        }

        #[cfg(feature = "html_report")]
        if cli_var.output.html {
            writeln!(writer, "Generating HTML report...")?;
            let report_dir = std::path::Path::new(".cytoscnpy/report");
            if let Err(e) =
                crate::report::generator::generate_report(&result, &analysis_root, report_dir)
            {
                eprintln!("Failed to generate HTML report: {e}");
            } else {
                writeln!(writer, "HTML report generated at: {}", report_dir.display())?;
                // Try to open in browser
                if let Err(e) = open::that(report_dir.join("index.html")) {
                    eprintln!("Failed to open report in browser: {e}");
                }
            }
        }

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
                eprintln!("   Min confidence: 90%");
                eprintln!("   Targets: functions, classes, imports");
                eprintln!("   CST mode: enabled (precise byte ranges)");
                eprintln!();
            }
            let fix_options = crate::commands::DeadCodeFixOptions {
                min_confidence: 90, // Only fix high-confidence items
                dry_run: !cli_var.apply,
                fix_functions: true,
                fix_classes: true,
                fix_imports: true,
                verbose: cli_var.output.verbose,
                with_cst: true, // CST is always enabled by default
                analysis_root: analysis_root.clone(),
            };
            crate::commands::run_fix_deadcode(&result, &fix_options, &mut *writer)?;
        }

        // Check for fail threshold (CLI > config > env var > default)
        let fail_threshold = cli_var
            .fail_threshold
            .or(config.cytoscnpy.fail_threshold)
            .or_else(|| {
                std::env::var("CYTOSCNPY_FAIL_THRESHOLD")
                    .ok()
                    .and_then(|v| v.parse::<f64>().ok())
            })
            .unwrap_or(100.0); // Default to 100% (never fail unless explicitly set)

        let mut exit_code = 0;

        // Calculate unused percentage and show gate status
        if result.analysis_summary.total_definitions > 0 {
            let total_unused = result.unused_functions.len()
                + result.unused_methods.len()
                + result.unused_classes.len()
                + result.unused_imports.len()
                + result.unused_variables.len()
                + result.unused_parameters.len();

            #[allow(clippy::cast_precision_loss)] // Counts are far below 2^52
            let percentage =
                (total_unused as f64 / result.analysis_summary.total_definitions as f64) * 100.0;

            // Only show gate banner if threshold is configured (not default 100%)
            let show_gate = fail_threshold < 100.0;

            if percentage > fail_threshold {
                if !cli_var.output.json {
                    eprintln!(
                        "\n[GATE] Unused code: {percentage:.1}% (threshold: {fail_threshold:.1}%) - FAILED"
                    );
                }

                exit_code = 1;
            } else if show_gate && !cli_var.output.json {
                writeln!(writer,
                    "\n[GATE] Unused code: {percentage:.1}% (threshold: {fail_threshold:.1}%) - PASSED"
                )?;
            }
        }

        // Complexity gate check
        let max_complexity = cli_var.max_complexity.or(config.cytoscnpy.max_complexity);
        if let Some(threshold) = max_complexity {
            // Find the highest complexity violation
            let complexity_violations: Vec<usize> = result
                .quality
                .iter()
                .filter(|f| f.rule_id == "CSP-Q301")
                .filter_map(|f| {
                    // Extract complexity value from message like "Function is too complex (McCabe=15)"
                    f.message
                        .split("McCabe=")
                        .nth(1)
                        .and_then(|s| s.trim_end_matches(')').parse::<usize>().ok())
                })
                .collect();

            if let Some(&max_found) = complexity_violations.iter().max() {
                if max_found > threshold {
                    if !cli_var.output.json {
                        eprintln!(
                            "\n[GATE] Max complexity: {max_found} (threshold: {threshold}) - FAILED"
                        );
                    }
                    exit_code = 1;
                } else if !cli_var.output.json {
                    writeln!(
                        writer,
                        "\n[GATE] Max complexity: {max_found} (threshold: {threshold}) - PASSED"
                    )?;
                }
            } else if !cli_var.output.json && !result.quality.is_empty() {
                // No complexity violations found, all functions are below threshold
                writeln!(
                    writer,
                    "\n[GATE] Max complexity: OK (threshold: {threshold}) - PASSED"
                )?;
            }
        }

        // Maintainability Index gate check
        let min_mi = cli_var.min_mi.or(config.cytoscnpy.min_mi);
        if let Some(threshold) = min_mi {
            let mi = result.analysis_summary.average_mi;
            if mi > 0.0 {
                if mi < threshold {
                    if !cli_var.output.json {
                        eprintln!(
                            "\n[GATE] Maintainability Index: {mi:.1} (threshold: {threshold:.1}) - FAILED"
                        );
                    }
                    exit_code = 1;
                } else if !cli_var.output.json {
                    writeln!(writer,
                        "\n[GATE] Maintainability Index: {mi:.1} (threshold: {threshold:.1}) - PASSED"
                    )?;
                }
            }
        }

        // Quality gate check (--fail-on-quality)
        if cli_var.output.fail_on_quality && !result.quality.is_empty() {
            if !cli_var.output.json {
                eprintln!(
                    "\n[GATE] Quality issues: {} found - FAILED",
                    result.quality.len()
                );
            }
            exit_code = 1;
        }

        Ok(exit_code)
    }
}

/// Run clone detection and return findings for JSON output.
/// This is used to include `clone_findings` in the combined JSON output.
fn run_clone_detection_for_json(
    paths: &[std::path::PathBuf],
    similarity: f64,
    excludes: &[String],
    verbose: bool,
) -> Vec<crate::clones::CloneFinding> {
    use crate::clones::{CloneConfig, CloneDetector};

    // Collect file paths (not content) for OOM-safe processing
    let file_paths: Vec<std::path::PathBuf> = paths
        .iter()
        .flat_map(|path| {
            if path.is_file() {
                vec![path.clone()]
            } else if path.is_dir() {
                crate::utils::collect_python_files_gitignore(path, excludes, &[], false, verbose).0
            } else {
                vec![]
            }
        })
        .collect();

    // Use OOM-safe detection - processes files in chunks
    let config = CloneConfig::default().with_min_similarity(similarity);
    let detector = CloneDetector::with_config(config);
    let result = detector.detect_from_paths(&file_paths);

    // Lazy load only matched files for findings generation
    let matched_files: Vec<(std::path::PathBuf, String)> = {
        use std::collections::HashSet;
        let unique_paths: HashSet<std::path::PathBuf> = result
            .pairs
            .iter()
            .flat_map(|p| [p.instance_a.file.clone(), p.instance_b.file.clone()])
            .collect();
        unique_paths
            .into_iter()
            .filter_map(|p| std::fs::read_to_string(&p).ok().map(|c| (p, c)))
            .collect()
    };

    // Generate findings
    crate::commands::generate_clone_findings(&result.pairs, &matched_files, true)
}
