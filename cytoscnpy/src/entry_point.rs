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

/// Runs the analyzer (or other commands) with the given arguments.
///
/// # Errors
///
/// Returns an error if argument parsing fails, or if the command execution fails.
#[allow(clippy::too_many_lines)]
pub fn run_with_args(args: Vec<String>) -> Result<i32> {
    let mut program_args = vec!["cytoscnpy".to_owned()];
    program_args.extend(args);
    let cli_var = Cli::parse_from(program_args);

    if let Some(command) = cli_var.command {
        let mut stdout = std::io::stdout();
        match command {
            Commands::Raw {
                path,
                json,
                exclude,
                ..
            } => {
                if !path.exists() {
                    eprintln!(
                        "Error: The file or directory '{}' does not exist.",
                        path.display()
                    );
                    return Ok(1);
                }
                crate::commands::run_raw(
                    &path,
                    json,
                    exclude,
                    Vec::new(),
                    false,
                    None,
                    &mut stdout,
                )?;
            }
            Commands::Cc {
                path,
                json,
                exclude,
                ..
            } => {
                if !path.exists() {
                    eprintln!(
                        "Error: The file or directory '{}' does not exist.",
                        path.display()
                    );
                    return Ok(1);
                }
                crate::commands::run_cc(
                    &path,
                    crate::commands::CcOptions {
                        json,
                        exclude,
                        ignore: Vec::new(),
                        min_rank: None,
                        max_rank: None,
                        average: false,
                        total_average: false,
                        show_complexity: false,
                        order: None,
                        no_assert: false,
                        xml: false,
                        fail_threshold: None,
                        output_file: None,
                    },
                    &mut stdout,
                )?;
            }
            Commands::Hal {
                path,
                json,
                exclude,
                ..
            } => {
                if !path.exists() {
                    eprintln!(
                        "Error: The file or directory '{}' does not exist.",
                        path.display()
                    );
                    return Ok(1);
                }
                crate::commands::run_hal(
                    &path,
                    json,
                    exclude,
                    Vec::new(),
                    false,
                    None,
                    &mut stdout,
                )?;
            }
            Commands::Mi {
                path,
                json,
                exclude,
                ignore,
                min_rank,
                max_rank,
                multi,
                show,
                average,
                fail_threshold,
                output_file,
            } => {
                if !path.exists() {
                    eprintln!(
                        "Error: The file or directory '{}' does not exist.",
                        path.display()
                    );
                    return Ok(1);
                }
                crate::commands::run_mi(
                    &path,
                    crate::commands::MiOptions {
                        json,
                        exclude,
                        ignore,
                        min_rank,
                        max_rank,
                        multi,
                        show,
                        average,
                        fail_threshold,
                        output_file,
                    },
                    &mut stdout,
                )?;
            }
        }
        Ok(0)
    } else {
        for path in &cli_var.paths {
            if !path.exists() {
                eprintln!(
                    "Error: The file or directory '{}' does not exist.",
                    path.display()
                );
                return Ok(1);
            }
        }
        let config_path = cli_var
            .paths
            .first()
            .map_or(std::path::Path::new("."), std::path::PathBuf::as_path);
        let config = crate::config::Config::load_from_path(config_path);
        let confidence = cli_var
            .confidence
            .or(config.cytoscnpy.confidence)
            .unwrap_or(60);
        let secrets = cli_var.scan.secrets || config.cytoscnpy.secrets.unwrap_or(false);
        let danger = cli_var.scan.danger || config.cytoscnpy.danger.unwrap_or(false);

        // Auto-enable quality mode when --min-mi or --max-complexity is set
        let quality = cli_var.scan.quality
            || config.cytoscnpy.quality.unwrap_or(false)
            || cli_var.min_mi.is_some()
            || cli_var.max_complexity.is_some()
            || config.cytoscnpy.min_mi.is_some()
            || config.cytoscnpy.complexity.is_some();

        let include_tests =
            cli_var.include.include_tests || config.cytoscnpy.include_tests.unwrap_or(false);

        let mut exclude_folders = config.cytoscnpy.exclude_folders.clone().unwrap_or_default();
        exclude_folders.extend(cli_var.exclude_folders);

        let mut include_folders = config.cytoscnpy.include_folders.clone().unwrap_or_default();
        include_folders.extend(cli_var.include_folders);

        if !cli_var.output.json {
            let mut stdout = std::io::stdout();
            crate::output::print_exclusion_list(&mut stdout, &exclude_folders).ok();
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
            eprintln!("   Paths: {:?}", cli_var.paths);
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
            exclude_folders,
            include_folders,
            cli_var.include.include_ipynb,
            cli_var.include.ipynb_cells,
            danger, // taint is now automatically enabled with --danger
            config.clone(),
        );

        // Count files first to create progress bar with accurate total
        let total_files = analyzer.count_files(&cli_var.paths);

        // Create progress bar with file count for visual feedback
        let progress = if cli_var.output.json {
            None
        } else if total_files > 0 {
            Some(crate::output::create_progress_bar(total_files as u64))
        } else {
            Some(crate::output::create_spinner())
        };

        let start_time = std::time::Instant::now();

        // Debug: Simulate progress for testing progress bar visibility
        if let Some(delay_ms) = cli_var.debug_delay {
            eprintln!("[DEBUG] delay_ms = {delay_ms}, total_files = {total_files}");
            if let Some(ref pb) = progress {
                for i in 0..total_files {
                    pb.set_position(i as u64);
                    pb.set_message(format!("file {}/{}", i + 1, total_files));
                    pb.tick();
                    std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                }
                pb.set_position(total_files as u64);
            }
        }

        let result = analyzer.analyze_paths(&cli_var.paths);

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

        if cli_var.output.json {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            let mut stdout = std::io::stdout();
            if cli_var.output.quiet {
                crate::output::print_report_quiet(&mut stdout, &result)?;
            } else {
                crate::output::print_report(&mut stdout, &result)?;
            }
            // Show processing time
            let elapsed = start_time.elapsed();
            println!("\n[TIME] Completed in {:.2}s", elapsed.as_secs_f64());
        }

        // Handle --clones flag
        if cli_var.clones {
            if cli_var.output.verbose && !cli_var.output.json {
                eprintln!("[VERBOSE] Clone detection enabled");
                eprintln!(
                    "   Similarity threshold: {:.0}%",
                    cli_var.clone_similarity * 100.0
                );
                if cli_var.fix {
                    eprintln!(
                        "   Fix mode: {} (confidence >= 90%)",
                        if cli_var.dry_run { "dry-run" } else { "apply" }
                    );
                }
                eprintln!();
            }
            let mut stdout = std::io::stdout();
            let clone_options = crate::commands::CloneOptions {
                similarity: cli_var.clone_similarity,
                json: cli_var.output.json,
                fix: cli_var.fix,
                dry_run: cli_var.dry_run,
                exclude: vec![], // Use empty - files already filtered by analyzer
                verbose: cli_var.output.verbose,
            };
            crate::commands::run_clones(&cli_var.paths, clone_options, &mut stdout)?;
        }

        // Handle --fix flag for dead code removal
        if cli_var.fix && !cli_var.clones {
            if cli_var.output.verbose && !cli_var.output.json {
                eprintln!("[VERBOSE] Dead code fix mode enabled");
                eprintln!(
                    "   Mode: {}",
                    if cli_var.dry_run {
                        "dry-run (preview)"
                    } else {
                        "apply changes"
                    }
                );
                eprintln!("   Min confidence: 90%");
                eprintln!("   Targets: functions, classes, imports");
                eprintln!();
            }
            let mut stdout = std::io::stdout();
            let fix_options = crate::commands::DeadCodeFixOptions {
                min_confidence: 90, // Only fix high-confidence items
                dry_run: cli_var.dry_run,
                fix_functions: true,
                fix_classes: true,
                fix_imports: true,
                verbose: cli_var.output.verbose,
            };
            crate::commands::run_fix_deadcode(&result, fix_options, &mut stdout)?;
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
                return Ok(1);
            } else if show_gate && !cli_var.output.json {
                println!(
                    "\n[GATE] Unused code: {percentage:.1}% (threshold: {fail_threshold:.1}%) - PASSED"
                );
            }
        }

        // Complexity gate check
        let max_complexity = cli_var.max_complexity.or(config.cytoscnpy.complexity);
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
                    return Ok(1);
                } else if !cli_var.output.json {
                    println!(
                        "\n[GATE] Max complexity: {max_found} (threshold: {threshold}) - PASSED"
                    );
                }
            } else if !cli_var.output.json && !result.quality.is_empty() {
                // No complexity violations found, all functions are below threshold
                println!("\n[GATE] Max complexity: OK (threshold: {threshold}) - PASSED");
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
                    return Ok(1);
                } else if !cli_var.output.json {
                    println!(
                        "\n[GATE] Maintainability Index: {mi:.1} (threshold: {threshold:.1}) - PASSED"
                    );
                }
            }
        }

        Ok(0)
    }
}
