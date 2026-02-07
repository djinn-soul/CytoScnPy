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
