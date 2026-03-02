use super::{Severity, SinkInfo, VulnType};
use ruff_python_ast::{self as ast, Expr};

pub(super) fn check_misc_sinks(name: &str) -> Option<SinkInfo> {
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
                rule_id: crate::rules::ids::RULE_ID_XSS_GENERIC.to_owned(),
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
            rule_id: crate::rules::ids::RULE_ID_METHOD_MISUSE.to_owned(),
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

pub(super) fn check_dynamic_attribute_sinks(call: &ast::ExprCall) -> Option<SinkInfo> {
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
