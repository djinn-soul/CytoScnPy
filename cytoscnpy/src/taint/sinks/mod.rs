use super::types::{Severity, VulnType};
use ruff_python_ast::{self as ast, Expr};

mod command_path;
mod framework_network;
mod misc;
mod patterns;
mod sql_code;

pub use patterns::SINK_PATTERNS;

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
pub fn check_sink(call: &ast::ExprCall) -> Option<SinkInfo> {
    let name = get_call_name(&call.func)?;

    sql_code::check_code_injection_sinks(&name)
        .or_else(|| sql_code::check_sql_injection_sinks(&name))
        .or_else(|| command_path::check_command_injection_sinks(&name, call))
        .or_else(|| command_path::check_path_traversal_sinks(&name))
        .or_else(|| framework_network::check_network_sinks(&name))
        .or_else(|| framework_network::check_framework_sink_packs(&name))
        .or_else(|| misc::check_misc_sinks(&name))
        .or_else(|| misc::check_dynamic_attribute_sinks(call))
}

/// Checks if a subprocess call has shell=True.
pub(super) fn has_shell_true(call: &ast::ExprCall) -> bool {
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
pub(super) fn get_call_name(func: &Expr) -> Option<String> {
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
