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
    /// Which keyword arguments are dangerous
    pub dangerous_keywords: Vec<String>,
    /// Suggested remediation
    pub remediation: String,
}

/// Checks if a call expression is a dangerous sink.
#[allow(clippy::too_many_lines)]
pub fn check_sink(call: &ast::ExprCall) -> Option<SinkInfo> {
    let name = get_call_name(&call.func)?;

    // Code injection sinks
    if name == "eval" {
        return Some(SinkInfo {
            name: "eval".to_owned(),
            vuln_type: VulnType::CodeInjection,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            dangerous_keywords: Vec::new(),
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
            dangerous_keywords: Vec::new(),
            remediation: "Avoid exec() with user input. Consider safer alternatives.".to_owned(),
        });
    }

    if name == "compile" {
        return Some(SinkInfo {
            name: "compile".to_owned(),
            vuln_type: VulnType::CodeInjection,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            dangerous_keywords: Vec::new(),
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
            dangerous_keywords: Vec::new(),
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
            dangerous_keywords: Vec::new(),
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
            dangerous_keywords: Vec::new(),
            remediation: "Use Django ORM methods instead of raw SQL.".to_owned(),
        });
    }

    if name == "pandas.read_sql" || name == "pd.read_sql" {
        return Some(SinkInfo {
            name,
            vuln_type: VulnType::SqlInjection,
            severity: Severity::High,
            dangerous_args: vec![0],
            dangerous_keywords: Vec::new(),
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
            dangerous_keywords: Vec::new(),
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
            dangerous_keywords: Vec::new(),
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
            dangerous_keywords: Vec::new(),
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
            dangerous_keywords: Vec::new(),
            remediation: "Validate and sanitize file paths. Use os.path.basename() or pathlib."
                .to_owned(),
        });
    }

    if name.starts_with("shutil.") || name.starts_with("os.path.") {
        return Some(SinkInfo {
            name,
            vuln_type: VulnType::PathTraversal,
            severity: Severity::High,
            dangerous_args: vec![], // Sentinel: check all positional args
            dangerous_keywords: vec!["path".to_owned(), "src".to_owned(), "dst".to_owned()],
            remediation: "Validate file paths before file operations.".to_owned(),
        });
    }

    if name == "pathlib.Path"
        || name == "pathlib.PurePath"
        || name == "pathlib.PosixPath"
        || name == "pathlib.WindowsPath"
        || name == "Path"
        || name == "PurePath"
        || name == "PosixPath"
        || name == "WindowsPath"
        || name == "zipfile.Path"
    {
        return Some(SinkInfo {
            name,
            vuln_type: VulnType::PathTraversal,
            severity: Severity::High,
            dangerous_args: vec![0],
            dangerous_keywords: vec![
                "path".to_owned(),
                "at".to_owned(),
                "file".to_owned(),
                "filename".to_owned(),
                "filepath".to_owned(),
            ],
            remediation: "Validate and sanitize file paths. Use os.path.basename() or pathlib."
                .to_owned(),
        });
    }

    // SSRF sinks
    if name.starts_with("requests.")
        || name.starts_with("httpx.")
        || name == "urllib.request.urlopen"
        || name == "urlopen"
    {
        let dangerous_args = if name.ends_with(".request") {
            vec![1]
        } else {
            vec![0]
        };
        return Some(SinkInfo {
            name,
            vuln_type: VulnType::Ssrf,
            severity: Severity::Critical,
            dangerous_args,
            dangerous_keywords: vec!["url".to_owned(), "uri".to_owned(), "address".to_owned()],
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
            dangerous_keywords: vec!["source".to_owned()],
            remediation: "Use render_template() with template files instead.".to_owned(),
        });
    }

    if name == "jinja2.Markup" || name == "Markup" || name == "mark_safe" {
        return Some(SinkInfo {
            name,
            vuln_type: VulnType::Xss,
            severity: Severity::High,
            dangerous_args: vec![0],
            dangerous_keywords: Vec::new(),
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
            dangerous_keywords: Vec::new(),
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
            dangerous_keywords: Vec::new(),
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
            dangerous_keywords: Vec::new(),
            remediation: "Validate redirect URLs against an allowlist.".to_owned(),
        });
    }

    // SQL Injection for Template and JinjaSQL
    if name == "Template.substitute" || name == "JinjaSql.prepare_query" {
        return Some(SinkInfo {
            name,
            vuln_type: VulnType::SqlInjection,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            dangerous_keywords: Vec::new(),
            remediation: "Avoid building raw SQL strings. Use parameterized queries.".to_owned(),
        });
    }

    // JinjaSql instance calls (Comment 1)
    if let Expr::Attribute(attr) = &*call.func {
        if attr.attr.as_str() == "prepare_query" {
            let is_jinja = match &*attr.value {
                Expr::Name(n) => {
                    let id = n.id.as_str().to_lowercase();
                    id == "j" || id.contains("jinjasql")
                }
                _ => false,
            };
            if is_jinja {
                return Some(SinkInfo {
                    name: "JinjaSql.prepare_query".to_owned(),
                    vuln_type: VulnType::SqlInjection,
                    severity: Severity::Critical,
                    dangerous_args: vec![0],
                    dangerous_keywords: Vec::new(),
                    remediation: "Avoid building raw SQL strings. Use parameterized queries."
                        .to_owned(),
                });
            }
        }
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
            } else if let Expr::Call(inner_call) = &*node.value {
                // Handling Template(...).substitute() and JinjaSql(...).prepare_query()
                if let Some(inner_name) = get_call_name(&inner_call.func) {
                    if (inner_name == "Template" || inner_name == "string.Template")
                        && (node.attr.as_str() == "substitute"
                            || node.attr.as_str() == "safe_substitute")
                    {
                        return Some("Template.substitute".to_owned());
                    }
                    if (inner_name == "JinjaSql" || inner_name == "jinjasql.JinjaSql")
                        && node.attr.as_str() == "prepare_query"
                    {
                        return Some("JinjaSql.prepare_query".to_owned());
                    }
                }
                None
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
    "os.path.",
    "subprocess.",
    "open",
    "shutil.",
    "pathlib.Path",
    "pathlib.PurePath",
    "pathlib.PosixPath",
    "pathlib.WindowsPath",
    "Path",
    "PurePath",
    "PosixPath",
    "WindowsPath",
    "zipfile.Path",
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
    "Template.substitute",
    "JinjaSql.prepare_query",
];
