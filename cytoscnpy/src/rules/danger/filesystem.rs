use super::utils::{create_finding, get_call_name, is_literal, is_literal_expr};
use crate::rules::{Context, Finding, Rule};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

/// Rule for detecting potential path traversal vulnerabilities.
pub struct PathTraversalRule;
impl Rule for PathTraversalRule {
    fn name(&self) -> &'static str {
        "PathTraversalRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D501"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "open"
                    || name.starts_with("os.path.")
                    || name.starts_with("shutil.")
                    || name == "pathlib.Path"
                    || name == "pathlib.PurePath"
                    || name == "pathlib.PosixPath"
                    || name == "pathlib.WindowsPath"
                    || name == "Path"
                    || name == "PurePath"
                    || name == "PosixPath"
                    || name == "WindowsPath"
                    || name == "zipfile.Path"
                {
                    let is_dynamic_args = !is_literal(&call.arguments.args);
                    let is_dynamic_kwargs = call.arguments.keywords.iter().any(|kw| {
                        kw.arg.as_ref().is_some_and(|a| {
                            let s = a.as_str();
                            s == "path"
                                || s == "file"
                                || s == "at"
                                || s == "filename"
                                || s == "filepath"
                                || s == "member"
                        }) && !is_literal_expr(&kw.value)
                    });

                    if is_dynamic_args || is_dynamic_kwargs {
                        return Some(vec![create_finding(
                            "Potential path traversal (dynamic file path)",
                            self.code(),
                            context,
                            call.range().start(),
                            "HIGH",
                        )]);
                    }
                }
            }
        }
        None
    }
}

/// Rule for detecting insecure temporary file usage.
pub struct TempfileRule;
impl Rule for TempfileRule {
    fn name(&self) -> &'static str {
        "TempfileRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D504"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                // CSP-D504 / B306 / B316: mktemp usage
                if name == "tempfile.mktemp" || name == "mktemp" {
                    return Some(vec![create_finding(
                        "Insecure use of tempfile.mktemp (race condition risk). Use tempfile.mkstemp or tempfile.TemporaryFile.",
                        self.code(),
                        context,
                        call.range().start(),
                        "HIGH",
                    )]);
                }
            }
        }
        None
    }
}

/// Rule for detecting insecure file permission settings.
pub struct BadFilePermissionsRule;
impl Rule for BadFilePermissionsRule {
    fn name(&self) -> &'static str {
        "BadFilePermissionsRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D505"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "os.chmod" {
                    // Check 'mode' argument (usually 2nd arg)
                    let mode_arg = if call.arguments.args.len() >= 2 {
                        Some(&call.arguments.args[1])
                    } else {
                        // Check keywords
                        call.arguments
                            .keywords
                            .iter()
                            .find(|k| k.arg.as_ref().is_some_and(|a| a == "mode"))
                            .map(|k| &k.value)
                    };

                    if let Some(mode) = mode_arg {
                        // Check for stat.S_IWOTH
                        if let Expr::Attribute(attr) = mode {
                            if attr.attr.as_str() == "S_IWOTH" {
                                return Some(vec![create_finding(
                                    "Setting file permissions to world-writable (S_IWOTH) is insecure.",
                                    self.code(),
                                    context,
                                    call.range().start(),
                                    "HIGH",
                                )]);
                            }
                        } else if let Expr::NumberLiteral(n) = mode {
                            if let ast::Number::Int(i) = &n.value {
                                if i.to_string() == "511" {
                                    return Some(vec![create_finding(
                                        "Setting file permissions to world-writable (0o777) is insecure.",
                                        self.code(),
                                        context,
                                        call.range().start(),
                                        "HIGH",
                                    )]);
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }
}
