//! Time-of-check/time-of-use race condition detection.

use super::super::utils::{create_finding, get_call_name};
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;
use rustc_hash::FxHashSet;

fn existence_check_path<'a>(
    expr: &'a Expr,
    pathlib_object_names: &FxHashSet<String>,
) -> Option<(&'a Expr, bool)> {
    if let Expr::Call(call) = expr {
        if let Some(name) = get_call_name(&call.func) {
            // Only match os.path.* and pathlib.Path.* qualified forms to avoid
            // false positives on ORM queryset.exists(), dict.get(), redis.exists(), etc.
            if matches!(
                name.as_str(),
                "os.path.exists" | "os.path.isfile" | "os.path.isdir" | "os.access"
            ) {
                return call.arguments.args.first().map(|path| (path, false));
            }
            if name.starts_with("pathlib.")
                && matches!(
                    name.rsplit('.').next().unwrap_or(""),
                    "exists" | "is_file" | "is_dir"
                )
            {
                return call.arguments.args.first().map(|path| (path, false));
            }
        }

        if let Expr::Attribute(attr) = &*call.func {
            if matches!(attr.attr.as_str(), "exists" | "is_file" | "is_dir")
                && is_tracked_pathlib_object(&attr.value, pathlib_object_names)
            {
                return Some((attr.value.as_ref(), false));
            }
        }
    }
    // Recurse into UnaryOp (e.g. `not os.path.exists(...)`)
    if let Expr::UnaryOp(u) = expr {
        if matches!(u.op, ast::UnaryOp::Not) {
            return existence_check_path(&u.operand, pathlib_object_names)
                .map(|(path, negated)| (path, !negated));
        }
    }
    None
}

/// Returns true if a statement (or its body) opens the checked file path.
fn body_contains_open_for_path(
    stmts: &[Stmt],
    checked_path: &Expr,
    pathlib_object_names: &FxHashSet<String>,
) -> bool {
    stmts
        .iter()
        .any(|stmt| stmt_contains_open_for_path(stmt, checked_path, pathlib_object_names))
}

fn stmt_contains_open_for_path(
    stmt: &Stmt,
    checked_path: &Expr,
    pathlib_object_names: &FxHashSet<String>,
) -> bool {
    match stmt {
        Stmt::Assign(a) => {
            expr_contains_open_for_path(&a.value, checked_path, pathlib_object_names)
        }
        Stmt::AugAssign(a) => {
            expr_contains_open_for_path(&a.value, checked_path, pathlib_object_names)
        }
        Stmt::AnnAssign(a) => a
            .value
            .as_ref()
            .is_some_and(|v| expr_contains_open_for_path(v, checked_path, pathlib_object_names)),
        Stmt::Expr(e) => expr_contains_open_for_path(&e.value, checked_path, pathlib_object_names),
        Stmt::With(w) => {
            w.items.iter().any(|item| {
                expr_contains_open_for_path(&item.context_expr, checked_path, pathlib_object_names)
            }) || body_contains_open_for_path(&w.body, checked_path, pathlib_object_names)
        }
        Stmt::If(i) => {
            body_contains_open_for_path(&i.body, checked_path, pathlib_object_names)
                || i.elif_else_clauses.iter().any(|c| {
                    body_contains_open_for_path(&c.body, checked_path, pathlib_object_names)
                })
        }
        _ => false,
    }
}

#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn expr_contains_open_for_path(
    expr: &Expr,
    checked_path: &Expr,
    pathlib_object_names: &FxHashSet<String>,
) -> bool {
    if let Expr::Call(call) = expr {
        if let Some(name) = get_call_name(&call.func) {
            if matches!(
                name.as_str(),
                "open" | "io.open" | "builtins.open" | "os.open"
            ) {
                if let Some(open_path) = call.arguments.args.first() {
                    return expr_same_path(open_path, checked_path);
                }
            }
        }
        if let Expr::Attribute(attr) = &*call.func {
            if attr.attr.as_str() == "open"
                && expr_same_path(&attr.value, checked_path)
                && is_tracked_pathlib_object(&attr.value, pathlib_object_names)
            {
                return true;
            }
        }
        // Recurse into chained callee receiver: `open(path).read()` → check attr.value
        if let Expr::Attribute(attr) = &*call.func {
            if expr_contains_open_for_path(&attr.value, checked_path, pathlib_object_names) {
                return true;
            }
        }
        // Check args/kwargs recursively
        return call
            .arguments
            .args
            .iter()
            .any(|arg| expr_contains_open_for_path(arg, checked_path, pathlib_object_names))
            || call.arguments.keywords.iter().any(|kw| {
                expr_contains_open_for_path(&kw.value, checked_path, pathlib_object_names)
            });
    }
    false
}

fn is_pathlib_constructor_call(expr: &Expr) -> bool {
    let Expr::Call(call) = expr else {
        return false;
    };

    get_call_name(&call.func).is_some_and(|name| {
        matches!(
            name.as_str(),
            "pathlib.Path"
                | "pathlib.PurePath"
                | "pathlib.PosixPath"
                | "pathlib.WindowsPath"
                | "Path"
                | "PurePath"
                | "PosixPath"
                | "WindowsPath"
        )
    })
}

fn is_tracked_pathlib_object(expr: &Expr, pathlib_object_names: &FxHashSet<String>) -> bool {
    match expr {
        Expr::Name(name) => pathlib_object_names.contains(name.id.as_str()),
        Expr::Call(_) => is_pathlib_constructor_call(expr),
        _ => false,
    }
}

fn expr_same_path(left: &Expr, right: &Expr) -> bool {
    match (left, right) {
        (Expr::Name(left_name), Expr::Name(right_name)) => left_name.id == right_name.id,
        (Expr::Attribute(_), Expr::Attribute(_)) => get_call_name(left) == get_call_name(right),
        (Expr::StringLiteral(left_string), Expr::StringLiteral(right_string)) => {
            left_string.value == right_string.value
        }
        _ => false,
    }
}

/// Rule for detecting TOCTOU (time-of-check/time-of-use) race conditions.
///
/// Flags `if os.path.exists(path): open(path, ...)` — the file may be replaced
/// between the check and the open. Use `try/except` instead.
/// RACE729 / CWE-362.
pub struct RaceConditionRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
    pathlib_object_names: FxHashSet<String>,
}

impl RaceConditionRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self {
            metadata,
            pathlib_object_names: FxHashSet::default(),
        }
    }

    fn record_pathlib_assignment(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Assign(assign) if is_pathlib_constructor_call(&assign.value) => {
                for target in &assign.targets {
                    if let Expr::Name(name) = target {
                        self.pathlib_object_names.insert(name.id.to_string());
                    }
                }
            }
            Stmt::AnnAssign(assign)
                if assign
                    .value
                    .as_ref()
                    .is_some_and(|value| is_pathlib_constructor_call(value)) =>
            {
                if let Expr::Name(name) = &*assign.target {
                    self.pathlib_object_names.insert(name.id.to_string());
                }
            }
            _ => {}
        }
    }
}

impl Rule for RaceConditionRule {
    fn name(&self) -> &'static str {
        "RaceConditionRule"
    }

    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }

    fn enter_stmt(&mut self, stmt: &ast::Stmt, context: &Context) -> Option<Vec<Finding>> {
        self.record_pathlib_assignment(stmt);

        if context.is_test_file {
            return None;
        }
        let ast::Stmt::If(if_stmt) = stmt else {
            return None;
        };

        // Build the full list of (test, body) pairs that participate in this
        // chain: the leading `if` plus every `elif`. Trailing `else` clauses
        // are recorded separately because they apply to whichever sibling
        // test matched the existence check.
        let mut tests_with_bodies: Vec<(&Expr, &[Stmt])> = Vec::new();
        tests_with_bodies.push((&if_stmt.test, &if_stmt.body));
        let mut else_bodies: Vec<&[Stmt]> = Vec::new();
        for clause in &if_stmt.elif_else_clauses {
            match clause.test.as_ref() {
                Some(test) => tests_with_bodies.push((test, &clause.body)),
                None => else_bodies.push(&clause.body),
            }
        }

        // For each branch whose test is an existence check, the TOCTOU window
        // covers both its own body (check-then-act) and any sibling `else`
        // body — the negated complement opens the same path. Polarity of the
        // check does not matter: either branch can be the offending one.
        let guarded_branch_has_open = tests_with_bodies.iter().any(|(test, body)| {
            let Some((checked_path, _negated)) =
                existence_check_path(test, &self.pathlib_object_names)
            else {
                return false;
            };
            if body_contains_open_for_path(body, checked_path, &self.pathlib_object_names) {
                return true;
            }
            else_bodies.iter().any(|else_body| {
                body_contains_open_for_path(else_body, checked_path, &self.pathlib_object_names)
            })
        });

        if guarded_branch_has_open {
            return Some(vec![create_finding(
                "TOCTOU race condition: file existence check followed by open(). The file may be modified between check and use. Use try/except instead.",
                self.metadata,
                context,
                stmt.range().start(),
                "MEDIUM",
            )]);
        }

        None
    }
}
