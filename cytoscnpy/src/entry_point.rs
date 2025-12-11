use crate::cli::{Cli, Commands};
use anyhow::Result;
use clap::Parser;
use rustpython_ast::{Constant, Expr, Stmt};
use std::collections::HashSet; // allows parse_from
/// Detects if `__name__ == "__main__"` blocks exist and extracts function calls from them.
///
/// This is crucial for correctly identifying entry points in Python scripts.
/// Functions called within this block should be considered "used" because they are the starting points of execution.
pub fn detect_entry_point_calls(stmts: &[Stmt]) -> HashSet<String> {
    let mut entry_point_calls = HashSet::new();

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
    if let Expr::Constant(const_expr) = expr {
        if let Constant::Str(s) = &const_expr.value {
            return s.as_str() == "__main__";
        }
    }
    false
}

/// Recursively collects all function calls from a statement.
///
/// This function traverses nested statements (like loops and nested ifs)
/// to find where functions are being called.
fn collect_function_calls(stmt: &Stmt, calls: &mut HashSet<String>) {
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
            for else_stmt in &if_stmt.orelse {
                collect_function_calls(else_stmt, calls);
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
fn collect_calls_from_expr(expr: &Expr, calls: &mut HashSet<String>) {
    match expr {
        // Found a call: func(...)
        Expr::Call(call) => {
            // Get the name of the function being called
            if let Some(name) = get_call_name(&call.func) {
                calls.insert(name);
            }
            // Recursively check arguments, they might contain calls too: func(other_func())
            for arg in &call.args {
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

#[cfg(test)]
mod tests {
    use super::*;
    use rustpython_parser::{parse, Mode};

    #[test]
    fn test_entry_point_detection() {
        let source = r#"
def my_function():
    pass

if __name__ == "__main__":
    my_function()
    another_call()
"#;

        let tree = parse(source, Mode::Module, "test.py").expect("Failed to parse");
        if let rustpython_ast::Mod::Module(module) = tree {
            let calls = detect_entry_point_calls(&module.body);

            assert!(
                calls.contains("my_function"),
                "Should detect my_function call"
            );
            assert!(calls.contains("another_call"), "Should detect another_call");
        }
    }

    #[test]
    fn test_no_entry_point() {
        let source = r#"
def my_function():
    pass
"#;

        let tree = parse(source, Mode::Module, "test.py").expect("Failed to parse");
        if let rustpython_ast::Mod::Module(module) = tree {
            let calls = detect_entry_point_calls(&module.body);
            assert_eq!(calls.len(), 0, "Should detect no entry point calls");
        }
    }

    #[test]
    fn test_reversed_main_guard() {
        let source = r#"
def func():
    pass

if "__main__" == __name__:
    func()
"#;

        let tree = parse(source, Mode::Module, "test.py").expect("Failed to parse");
        if let rustpython_ast::Mod::Module(module) = tree {
            let calls = detect_entry_point_calls(&module.body);
            assert!(calls.contains("func"), "Should handle reversed comparison");
        }
    }
}

/// Runs the analyzer (or other commands) with the given arguments.
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
            } => {
                if !path.exists() {
                    eprintln!(
                        "Error: The file or directory '{}' does not exist.",
                        path.display()
                    );
                    return Ok(1);
                }
                crate::commands::run_raw(
                    path,
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
            } => {
                if !path.exists() {
                    eprintln!(
                        "Error: The file or directory '{}' does not exist.",
                        path.display()
                    );
                    return Ok(1);
                }
                crate::commands::run_cc(
                    path,
                    json,
                    exclude,
                    Vec::new(),
                    None,
                    None,
                    false,
                    false,
                    false,
                    None,
                    false,
                    false,
                    None, // fail_threshold
                    None, // output_file
                    &mut stdout,
                )?
            }
            Commands::Hal {
                path,
                json,
                exclude,
            } => {
                if !path.exists() {
                    eprintln!(
                        "Error: The file or directory '{}' does not exist.",
                        path.display()
                    );
                    return Ok(1);
                }
                crate::commands::run_hal(
                    path,
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
            } => {
                if !path.exists() {
                    eprintln!(
                        "Error: The file or directory '{}' does not exist.",
                        path.display()
                    );
                    return Ok(1);
                }
                crate::commands::run_mi(
                    path,
                    json,
                    exclude,
                    Vec::new(),
                    None,
                    None,
                    false,
                    false,
                    false, // average
                    None,  // fail_under
                    None,  // output_file
                    &mut stdout,
                )?
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
        let secrets = cli_var.secrets || config.cytoscnpy.secrets.unwrap_or(false);
        let danger = cli_var.danger || config.cytoscnpy.danger.unwrap_or(false);
        let quality = cli_var.quality || config.cytoscnpy.quality.unwrap_or(false);
        let include_tests =
            cli_var.include_tests || config.cytoscnpy.include_tests.unwrap_or(false);

        let mut exclude_folders = config.cytoscnpy.exclude_folders.clone().unwrap_or_default();
        exclude_folders.extend(cli_var.exclude_folders);

        let mut include_folders = config.cytoscnpy.include_folders.clone().unwrap_or_default();
        include_folders.extend(cli_var.include_folders);

        if !cli_var.json {
            let mut stdout = std::io::stdout();
            crate::output::print_exclusion_list(&mut stdout, &exclude_folders).ok();
        }

        let spinner = if cli_var.json {
            None
        } else {
            Some(crate::output::create_spinner())
        };

        let mut analyzer = crate::analyzer::CytoScnPy::new(
            confidence,
            secrets,
            danger,
            quality,
            include_tests,
            exclude_folders,
            include_folders,
            cli_var.include_ipynb,
            cli_var.ipynb_cells,
            danger || cli_var.taint, // taint enabled with --danger or --taint
            config.clone(),
        );
        let result = analyzer.analyze_paths(&cli_var.paths)?;

        if let Some(s) = spinner {
            s.finish_and_clear();
        }

        if cli_var.json {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            let mut stdout = std::io::stdout();
            crate::output::print_report(&mut stdout, &result)?;
        }

        // Check for fail threshold
        let fail_threshold = config
            .cytoscnpy
            .fail_threshold
            .or_else(|| {
                std::env::var("CYTOSCNPY_FAIL_THRESHOLD")
                    .ok()
                    .and_then(|v| v.parse::<f64>().ok())
            })
            .unwrap_or(100.0); // Default to 100% (never fail unless 0.0 is passed explicitly)

        // Calculate unused percentage
        if result.analysis_summary.total_definitions > 0 {
            let total_unused = result.unused_functions.len()
                + result.unused_methods.len()
                + result.unused_classes.len()
                + result.unused_imports.len()
                + result.unused_variables.len()
                + result.unused_parameters.len();

            let percentage =
                (total_unused as f64 / result.analysis_summary.total_definitions as f64) * 100.0;

            if percentage > fail_threshold {
                eprintln!(
                    "Error: Unused code percentage ({:.2}%) exceeds threshold ({:.2}%).",
                    percentage, fail_threshold
                );
                return Ok(1);
            }
        }

        Ok(0)
    }
}
