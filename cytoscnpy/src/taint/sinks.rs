use super::types::{Severity, VulnType};
use crate::rules::ids;
use ruff_python_ast::{self as ast, Expr};

/// Information about a detected sink.
#[derive(Debug, Clone)]
pub struct SinkInfo {
    /// Name of the sink function/pattern
    pub name: String,
    /// Rule ID
    pub rule_id: String,
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
/// Checks if a call expression is a dangerous sink.
pub fn check_sink(call: &ast::ExprCall) -> Option<SinkInfo> {
    let name = get_call_name(&call.func)?;

    check_code_injection_sinks(&name)
        .or_else(|| check_sql_injection_sinks(&name))
        .or_else(|| check_command_injection_sinks(&name, call))
        .or_else(|| check_path_traversal_sinks(&name))
        .or_else(|| check_network_sinks(&name))
        .or_else(|| check_misc_sinks(&name))
        .or_else(|| check_dynamic_attribute_sinks(call))
}

fn check_code_injection_sinks(name: &str) -> Option<SinkInfo> {
    match name {
        "eval" => Some(SinkInfo {
            name: "eval".to_owned(),
            rule_id: ids::RULE_ID_EVAL.to_owned(),
            vuln_type: VulnType::CodeInjection,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            dangerous_keywords: Vec::new(),
            remediation: "Avoid eval() with user input. Use ast.literal_eval() for safe parsing."
                .to_owned(),
        }),
        "exec" | "compile" => {
            let actual_name = if name == "exec" { "exec" } else { "compile" };
            Some(SinkInfo {
                name: actual_name.to_owned(),
                rule_id: ids::RULE_ID_EXEC.to_owned(),
                vuln_type: VulnType::CodeInjection,
                severity: Severity::Critical,
                dangerous_args: vec![0],
                dangerous_keywords: Vec::new(),
                remediation: format!(
                    "Avoid {actual_name}() with user input. Consider safer alternatives."
                ),
            })
        }
        _ => None,
    }
}

fn check_sql_injection_sinks(name: &str) -> Option<SinkInfo> {
    if name.ends_with(".execute") || name.ends_with(".executemany") {
        return Some(SinkInfo {
            name: name.to_owned(),
            rule_id: ids::RULE_ID_SQL_RAW.to_owned(),
            vuln_type: VulnType::SqlInjection,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            dangerous_keywords: Vec::new(),
            remediation: "Use parameterized queries: cursor.execute(sql, (param,))".to_owned(),
        });
    }

    // This is not actually a file extension comparison - we're checking method name suffixes
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    if name == "sqlalchemy.text" || name.ends_with(".text") || name.ends_with(".objects.raw") {
        let rule_id = if name.ends_with(".objects.raw")
            || name == "sqlalchemy.text"
            || name.ends_with(".text")
        {
            ids::RULE_ID_SQL_INJECTION.to_owned()
        } else {
            ids::RULE_ID_SQL_RAW.to_owned()
        };
        return Some(SinkInfo {
            name: name.to_owned(),
            rule_id,
            vuln_type: VulnType::SqlInjection,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            dangerous_keywords: Vec::new(),
            remediation: if name.ends_with(".objects.raw") {
                "Use Django ORM methods instead of raw SQL.".to_owned()
            } else {
                "Use bound parameters: text('SELECT * WHERE id=:id').bindparams(id=val)".to_owned()
            },
        });
    }

    if name == "pandas.read_sql"
        || name == "pd.read_sql"
        || name == "Template.substitute"
        || name == "JinjaSql.prepare_query"
    {
        return Some(SinkInfo {
            name: name.to_owned(),
            rule_id: ids::RULE_ID_SQL_RAW.to_owned(),
            vuln_type: VulnType::SqlInjection,
            severity: if name.starts_with("pandas") || name.starts_with("pd") {
                Severity::High
            } else {
                Severity::Critical
            },
            dangerous_args: vec![0],
            dangerous_keywords: Vec::new(),
            remediation: if name.contains("pandas") || name.contains("pd") {
                "Use parameterized queries with pd.read_sql(sql, con, params=[...])".to_owned()
            } else {
                "Avoid building raw SQL strings. Use parameterized queries.".to_owned()
            },
        });
    }

    None
}

fn check_command_injection_sinks(name: &str, call: &ast::ExprCall) -> Option<SinkInfo> {
    if name == "os.system"
        || name == "os.popen"
        || (name.starts_with("subprocess.") && has_shell_true(call))
    {
        return Some(SinkInfo {
            name: name.to_owned(),
            rule_id: ids::RULE_ID_SUBPROCESS.to_owned(),
            vuln_type: VulnType::CommandInjection,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            dangerous_keywords: Vec::new(),
            remediation: if name.starts_with("subprocess") {
                "Use shell=False and pass arguments as a list.".to_owned()
            } else {
                "Use subprocess.run() with shell=False.".to_owned()
            },
        });
    }
    None
}

fn check_path_traversal_sinks(name: &str) -> Option<SinkInfo> {
    if name == "open" {
        return Some(SinkInfo {
            name: "open".to_owned(),
            rule_id: ids::RULE_ID_PATH_TRAVERSAL.to_owned(),
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
            name: name.to_owned(),
            rule_id: "CSP-D501".to_owned(),
            vuln_type: VulnType::PathTraversal,
            severity: Severity::High,
            dangerous_args: vec![], // Sentinel: check all positional args
            dangerous_keywords: vec!["path".to_owned(), "src".to_owned(), "dst".to_owned()],
            remediation: "Validate file paths before file operations.".to_owned(),
        });
    }

    let is_pathlib = name == "pathlib.Path"
        || name == "pathlib.PurePath"
        || name == "pathlib.PosixPath"
        || name == "pathlib.WindowsPath"
        || name == "Path"
        || name == "PurePath"
        || name == "PosixPath"
        || name == "WindowsPath"
        || name == "zipfile.Path";

    if is_pathlib {
        return Some(SinkInfo {
            name: name.to_owned(),
            rule_id: "CSP-D501".to_owned(),
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

    None
}

fn check_network_sinks(name: &str) -> Option<SinkInfo> {
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
            name: name.to_owned(),
            rule_id: ids::RULE_ID_SSRF.to_owned(),
            vuln_type: VulnType::Ssrf,
            severity: Severity::Critical,
            dangerous_args,
            dangerous_keywords: vec!["url".to_owned(), "uri".to_owned(), "address".to_owned()],
            remediation: "Validate URLs against an allowlist. Block internal/private IP ranges."
                .to_owned(),
        });
    }

    if name == "redirect" || name == "flask.redirect" || name == "django.shortcuts.redirect" {
        return Some(SinkInfo {
            name: name.to_owned(),
            rule_id: ids::RULE_ID_OPEN_REDIRECT.to_owned(),
            vuln_type: VulnType::OpenRedirect,
            severity: Severity::Medium,
            dangerous_args: vec![0],
            dangerous_keywords: Vec::new(),
            remediation: "Validate redirect URLs against an allowlist.".to_owned(),
        });
    }

    None
}

fn check_misc_sinks(name: &str) -> Option<SinkInfo> {
    match name {
        "flask.render_template_string"
        | "render_template_string"
        | "jinja2.Markup"
        | "Markup"
        | "mark_safe" => {
            let vuln_type = VulnType::Xss;
            let remediation = if name.contains("render_template") {
                "Use render_template() with template files instead.".to_owned()
            } else {
                "Escape user input before marking as safe.".to_owned()
            };
            Some(SinkInfo {
                name: name.to_owned(),
                rule_id: ids::RULE_ID_XSS_GENERIC.to_owned(),
                vuln_type,
                severity: Severity::High,
                dangerous_args: vec![0],
                dangerous_keywords: if name.contains("render_template") {
                    vec!["source".to_owned()]
                } else {
                    Vec::new()
                },
                remediation,
            })
        }
        "pickle.load" | "pickle.loads" | "yaml.load" | "yaml.unsafe_load" => Some(SinkInfo {
            name: name.to_owned(),
            rule_id: ids::RULE_ID_METHOD_MISUSE.to_owned(), // Note: Using D601 as generic deser fallback
            vuln_type: VulnType::Deserialization,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            dangerous_keywords: Vec::new(),
            remediation: if name.contains("pickle") {
                "Avoid unpickling untrusted data. Use JSON or other safe formats.".to_owned()
            } else {
                "Use yaml.safe_load() instead.".to_owned()
            },
        }),
        _ => None,
    }
}

fn check_dynamic_attribute_sinks(call: &ast::ExprCall) -> Option<SinkInfo> {
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
                    rule_id: "CSP-D102".to_owned(),
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
