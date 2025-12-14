//! Dangerous sink detection.
//!
//! Identifies where tainted data can cause security vulnerabilities.

use super::types::{Severity, VulnType};
use ruff_python_ast::{self as ast, Expr};

/// Information about a detected sink.
#[derive(Debug, Clone)]
pub struct SinkInfo {
    /// Name of the sink function/pattern
    pub name: String,
    /// Type of vulnerability this sink can cause
    pub vuln_type: VulnType,
    /// Severity level
    pub severity: Severity,
    /// Which argument positions are dangerous (0-indexed)
    pub dangerous_args: Vec<usize>,
    /// Suggested remediation
    pub remediation: String,
}

/// Checks if a call expression is a dangerous sink.
pub fn check_sink(call: &ast::ExprCall) -> Option<SinkInfo> {
    let name = get_call_name(&call.func)?;

    // Code injection sinks
    if name == "eval" {
        return Some(SinkInfo {
            name: "eval".to_owned(),
            vuln_type: VulnType::CodeInjection,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            remediation: "Avoid eval() with user input. Use ast.literal_eval() for safe parsing."
                .to_owned(),
        });
    }

    if name == "exec" {
        return Some(SinkInfo {
            name: "exec".to_owned(),
            vuln_type: VulnType::CodeInjection,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            remediation: "Avoid exec() with user input. Consider safer alternatives.".to_owned(),
        });
    }

    if name == "compile" {
        return Some(SinkInfo {
            name: "compile".to_owned(),
            vuln_type: VulnType::CodeInjection,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            remediation: "Avoid compile() with user input.".to_owned(),
        });
    }

    // SQL injection sinks
    if name.ends_with(".execute") || name.ends_with(".executemany") {
        return Some(SinkInfo {
            name,
            vuln_type: VulnType::SqlInjection,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            remediation: "Use parameterized queries: cursor.execute(sql, (param,))".to_owned(),
        });
    }

    // This is not actually a file extension comparison - we're checking method name suffixes
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    if name == "sqlalchemy.text" || name.ends_with(".text") {
        return Some(SinkInfo {
            name,
            vuln_type: VulnType::SqlInjection,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            remediation: "Use bound parameters: text('SELECT * WHERE id=:id').bindparams(id=val)"
                .to_owned(),
        });
    }

    if name.ends_with(".objects.raw") {
        return Some(SinkInfo {
            name,
            vuln_type: VulnType::SqlInjection,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            remediation: "Use Django ORM methods instead of raw SQL.".to_owned(),
        });
    }

    if name == "pandas.read_sql" || name == "pd.read_sql" {
        return Some(SinkInfo {
            name,
            vuln_type: VulnType::SqlInjection,
            severity: Severity::High,
            dangerous_args: vec![0],
            remediation: "Use parameterized queries with pd.read_sql(sql, con, params=[...])"
                .to_owned(),
        });
    }

    // Command injection sinks
    if name == "os.system" {
        return Some(SinkInfo {
            name: "os.system".to_owned(),
            vuln_type: VulnType::CommandInjection,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            remediation: "Use subprocess.run() with shell=False and a list of arguments."
                .to_owned(),
        });
    }

    if name == "os.popen" {
        return Some(SinkInfo {
            name: "os.popen".to_owned(),
            vuln_type: VulnType::CommandInjection,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            remediation: "Use subprocess.run() with shell=False.".to_owned(),
        });
    }

    // subprocess with shell=True
    if name.starts_with("subprocess.") && has_shell_true(call) {
        return Some(SinkInfo {
            name,
            vuln_type: VulnType::CommandInjection,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            remediation: "Use shell=False and pass arguments as a list.".to_owned(),
        });
    }

    // Path traversal sinks
    if name == "open" {
        return Some(SinkInfo {
            name: "open".to_owned(),
            vuln_type: VulnType::PathTraversal,
            severity: Severity::High,
            dangerous_args: vec![0],
            remediation: "Validate and sanitize file paths. Use os.path.basename() or pathlib."
                .to_owned(),
        });
    }

    if name.starts_with("shutil.") {
        return Some(SinkInfo {
            name,
            vuln_type: VulnType::PathTraversal,
            severity: Severity::High,
            dangerous_args: vec![0, 1],
            remediation: "Validate file paths before file operations.".to_owned(),
        });
    }

    // SSRF sinks
    if name.starts_with("requests.")
        || name.starts_with("httpx.")
        || name == "urllib.request.urlopen"
        || name == "urlopen"
    {
        return Some(SinkInfo {
            name,
            vuln_type: VulnType::Ssrf,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            remediation: "Validate URLs against an allowlist. Block internal/private IP ranges."
                .to_owned(),
        });
    }

    // XSS sinks
    if name == "flask.render_template_string" || name == "render_template_string" {
        return Some(SinkInfo {
            name,
            vuln_type: VulnType::Xss,
            severity: Severity::High,
            dangerous_args: vec![0],
            remediation: "Use render_template() with template files instead.".to_owned(),
        });
    }

    if name == "jinja2.Markup" || name == "Markup" || name == "mark_safe" {
        return Some(SinkInfo {
            name,
            vuln_type: VulnType::Xss,
            severity: Severity::High,
            dangerous_args: vec![0],
            remediation: "Escape user input before marking as safe.".to_owned(),
        });
    }

    // Deserialization sinks
    if name == "pickle.load" || name == "pickle.loads" {
        return Some(SinkInfo {
            name,
            vuln_type: VulnType::Deserialization,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            remediation: "Avoid unpickling untrusted data. Use JSON or other safe formats."
                .to_owned(),
        });
    }

    if name == "yaml.load" || name == "yaml.unsafe_load" {
        return Some(SinkInfo {
            name,
            vuln_type: VulnType::Deserialization,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            remediation: "Use yaml.safe_load() instead.".to_owned(),
        });
    }

    // Open redirect
    if name == "redirect" || name == "flask.redirect" || name == "django.shortcuts.redirect" {
        return Some(SinkInfo {
            name,
            vuln_type: VulnType::OpenRedirect,
            severity: Severity::Medium,
            dangerous_args: vec![0],
            remediation: "Validate redirect URLs against an allowlist.".to_owned(),
        });
    }

    None
}

/// Checks if a subprocess call has shell=True.
fn has_shell_true(call: &ast::ExprCall) -> bool {
    for keyword in &call.arguments.keywords {
        if let Some(arg) = &keyword.arg {
            if arg.as_str() == "shell" {
                if let Expr::BooleanLiteral(b) = &keyword.value {
                    if b.value {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Extracts the call name from a function expression.
fn get_call_name(func: &Expr) -> Option<String> {
    match func {
        Expr::Name(node) => Some(node.id.to_string()),
        Expr::Attribute(node) => {
            if let Expr::Name(value) = &*node.value {
                Some(format!("{}.{}", value.id, node.attr))
            } else if let Expr::Attribute(inner) = &*node.value {
                if let Expr::Name(name) = &*inner.value {
                    Some(format!("{}.{}.{}", name.id, inner.attr, node.attr))
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// List of all sink patterns for quick lookup.
pub static SINK_PATTERNS: &[&str] = &[
    "eval",
    "exec",
    "compile",
    ".execute",
    ".executemany",
    ".text",
    ".objects.raw",
    "os.system",
    "os.popen",
    "subprocess.",
    "open",
    "shutil.",
    "requests.",
    "httpx.",
    "urlopen",
    "render_template_string",
    "Markup",
    "mark_safe",
    "pickle.load",
    "pickle.loads",
    "yaml.load",
    "redirect",
];

#[cfg(test)]
mod tests {
    use super::*;
    use ruff_python_parser::{parse, Mode};

    fn parse_call(source: &str) -> ast::ExprCall {
        let tree = parse(source, Mode::Expression.into()).unwrap();
        if let ast::Mod::Expression(expr) = tree.into_syntax() {
            if let Expr::Call(call) = *expr.body {
                return call;
            }
        }
        panic!("Expected call expression")
    }

    #[test]
    fn test_eval_sink() {
        let call = parse_call("eval(x)");
        let sink = check_sink(&call);
        assert!(sink.is_some());
        assert!(matches!(sink.unwrap().vuln_type, VulnType::CodeInjection));
    }

    #[test]
    fn test_execute_sink() {
        let call = parse_call("cursor.execute(query)");
        let sink = check_sink(&call);
        assert!(sink.is_some());
        assert!(matches!(sink.unwrap().vuln_type, VulnType::SqlInjection));
    }

    #[test]
    fn test_subprocess_shell_true() {
        let call = parse_call("subprocess.run(cmd, shell=True)");
        let sink = check_sink(&call);
        assert!(sink.is_some());
        assert!(matches!(
            sink.unwrap().vuln_type,
            VulnType::CommandInjection
        ));
    }
}
