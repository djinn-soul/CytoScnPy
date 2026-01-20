use super::utils::{create_finding, get_call_name, is_literal_expr};
use crate::rules::ids;
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::Expr;
use ruff_text_size::Ranged;

/// Rule for detecting potential SQL injection vulnerabilities.
pub const META_SQL_INJECTION: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_SQL_INJECTION,
    category: super::CAT_INJECTION,
};
/// Rule for detecting potential raw SQL injection.
pub const META_SQL_RAW: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_SQL_RAW,
    category: super::CAT_INJECTION,
};
/// Rule for detecting potential reflected XSS.
pub const META_XSS: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_XSS,
    category: super::CAT_INJECTION,
};
/// Rule for detecting insecure XML parsing.
pub const META_XML: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_XML,
    category: super::CAT_INJECTION,
};
/// Rule for detecting use of `mark_safe` which bypasses autoescaping.
pub const META_MARK_SAFE: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_MARK_SAFE,
    category: super::CAT_INJECTION,
};

/// Rule for detecting potential SQL injection vulnerabilities in common ORMs and drivers.
pub struct SqlInjectionRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl SqlInjectionRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for SqlInjectionRule {
    fn name(&self) -> &'static str {
        "SqlInjectionRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                // Common ORM patterns (Django .execute, etc.)
                if (name.ends_with(".execute") || name.ends_with(".executemany"))
                    && !call.arguments.args.is_empty()
                    && !is_literal_expr(&call.arguments.args[0])
                {
                    return Some(vec![create_finding(
                        "Potential SQL injection (dynamic query string)",
                        self.metadata,
                        context,
                        call.range().start(),
                        "CRITICAL",
                    )]);
                }
            }
        }
        None
    }
}

/// Rule for detecting potential raw SQL injection.
pub struct SqlInjectionRawRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl SqlInjectionRawRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for SqlInjectionRawRule {
    fn name(&self) -> &'static str {
        "SqlInjectionRawRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            let name_opt = get_call_name(&call.func);
            let attr_name = if let Expr::Attribute(attr) = &*call.func {
                Some(attr.attr.as_str())
            } else {
                None
            };

            // Detect patterns by full name or attribute name for "fluent" APIs
            let is_sqli_pattern = if let Some(name) = &name_opt {
                name == "execute"
                    || name == "executemany"
                    || name == "raw"
                    || name == "sqlalchemy.text"
                    || name == "text"
                    || name == "pandas.read_sql"
                    || name == "pandas.read_sql_query"
                    || name == "read_sql"
                    || name == "read_sql_query"
                    || name.to_lowercase().ends_with(".execute")
                    || name.to_lowercase().ends_with(".raw")
                    || name.to_lowercase().ends_with(".prepare_query")
                    || name.to_lowercase().ends_with(".substitute")
            } else if let Some(attr) = attr_name {
                attr == "substitute"
                    || attr == "prepare_query"
                    || attr == "execute"
                    || attr == "raw"
            } else {
                false
            };

            if is_sqli_pattern {
                let mut is_dangerous = false;

                // Check if FIRST argument is non-literal
                if let Some(arg) = call.arguments.args.first() {
                    if !is_literal_expr(arg) {
                        is_dangerous = true;
                    }
                }

                // Check if ANY keyword argument is non-literal
                if !is_dangerous {
                    for kw in &call.arguments.keywords {
                        if !is_literal_expr(&kw.value) {
                            is_dangerous = true;
                            break;
                        }
                    }
                }

                // Special case: Template(user_sql).substitute(...)
                if !is_dangerous {
                    if let Expr::Attribute(attr) = &*call.func {
                        if attr.attr.as_str() == "substitute" {
                            if let Expr::Call(inner_call) = &*attr.value {
                                if let Some(inner_name) = get_call_name(&inner_call.func) {
                                    if inner_name == "Template" || inner_name == "string.Template" {
                                        if let Some(arg) = inner_call.arguments.args.first() {
                                            if !is_literal_expr(arg) {
                                                is_dangerous = true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if is_dangerous {
                    return Some(vec![create_finding(
                        "Potential SQL injection or dynamic query execution.",
                        self.metadata,
                        context,
                        call.range().start(),
                        "CRITICAL",
                    )]);
                }
            }
        }
        None
    }
}

/// Rule for detecting potential reflected XSS in web frameworks.
pub struct XSSRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl XSSRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for XSSRule {
    fn name(&self) -> &'static str {
        "XSSRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            let name_opt = get_call_name(&call.func);
            let attr_name = if let Expr::Attribute(attr) = &*call.func {
                Some(attr.attr.as_str())
            } else {
                None
            };

            let is_xss_pattern = if let Some(name) = &name_opt {
                name == "flask.render_template_string"
                    || name == "render_template_string"
                    || name == "flask.render_template"
                    || name == "render_template"
                    || name == "jinja2.Template"
                    || name == "Template"
                    || name == "Markup"
                    || name == "flask.Markup"
                    || name == "jinja2.Markup"
                    || name == "jinja2.from_string"
                    || name == "fastapi.responses.HTMLResponse"
                    || name == "HTMLResponse"
            } else if let Some(attr) = attr_name {
                // Note: mark_safe and format_html are handled by BlacklistCallRule to avoid redundancy/ID mismatch
                attr == "render_template_string" || attr == "Markup"
            } else {
                false
            };

            if is_xss_pattern {
                let mut is_dynamic = false;

                // Check positional arguments
                if !call.arguments.args.is_empty() && !is_literal_expr(&call.arguments.args[0]) {
                    is_dynamic = true;
                }

                // Check relevant keywords
                if !is_dynamic {
                    for kw in &call.arguments.keywords {
                        if let Some(arg) = &kw.arg {
                            let s = arg.as_str();
                            if (s == "content"
                                || s == "template_string"
                                || s == "source"
                                || s == "html")
                                && !is_literal_expr(&kw.value)
                            {
                                is_dynamic = true;
                                break;
                            }
                        }
                    }
                }

                if is_dynamic {
                    return Some(vec![create_finding(
                        "Potential XSS vulnerability (unsafe HTML/template rendering with dynamic content)",
                        self.metadata,
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

/// Rule for detecting insecure XML parsing (XXE/DoS risk).
pub struct XmlRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl XmlRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for XmlRule {
    fn name(&self) -> &'static str {
        "XmlRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            let name_opt = get_call_name(&call.func);
            let attr_name = if let Expr::Attribute(attr) = &*call.func {
                Some(attr.attr.as_str())
            } else {
                None
            };

            // Detect patterns by full name or attribute name for aliases like "ET"
            let is_xml_pattern = if let Some(name) = &name_opt {
                name.contains("lxml.etree")
                    || name.contains("etree.")
                    || name.starts_with("xml.etree.ElementTree.")
                    || name.starts_with("ElementTree.")
                    || name.starts_with("xml.dom.minidom.")
                    || name.starts_with("xml.sax.")
                    || name.contains("minidom.")
                    || name.contains("sax.")
                    || name.contains("pulldom.")
                    || name.contains("expatbuilder.")
                    || name.starts_with("ET.")
                    || name == "ET.parse"
                    || name == "ET.fromstring"
                    || name == "ET.XML"
                    || name == "xml.sax.make_parser"
            } else if let Some(attr) = attr_name {
                attr == "parse"
                    || attr == "fromstring"
                    || attr == "XML"
                    || attr == "make_parser"
                    || attr == "RestrictedElement"
                    || attr == "GlobalParserTLS"
                    || attr == "getDefaultParser"
                    || attr == "check_docinfo"
            } else {
                false
            };

            if is_xml_pattern {
                let mut severity = "MEDIUM";
                let mut msg = "Insecure XML parsing (vulnerable to XXE or DoS).";

                if let Some(name) = &name_opt {
                    if name.contains("lxml") || name.contains("etree") {
                        severity = "HIGH";
                        msg = "Insecure XML parsing (resolve_entities is enabled by default in lxml). XXE risk.";
                    } else if name.contains("sax") {
                        msg = "Insecure XML parsing (SAX is vulnerable to XXE).";
                    } else if name.contains("minidom") {
                        msg = "Insecure XML parsing (minidom is vulnerable to XXE).";
                    }
                } else if let Some(attr) = attr_name {
                    if attr == "RestrictedElement"
                        || attr == "GlobalParserTLS"
                        || attr == "getDefaultParser"
                        || attr == "check_docinfo"
                    {
                        severity = "HIGH";
                        msg = "Insecure XML parsing (resolve_entities is enabled by default in lxml). XXE risk.";
                    }
                }

                // Check lxml resolve_entities specifically
                if let Some(name) = &name_opt {
                    if name.contains("lxml.etree") {
                        let mut resolve_entities = true;
                        for keyword in &call.arguments.keywords {
                            if let Some(arg) = &keyword.arg {
                                if arg == "resolve_entities" {
                                    if let Expr::BooleanLiteral(b) = &keyword.value {
                                        resolve_entities = b.value;
                                    }
                                }
                            }
                        }
                        if !resolve_entities {
                            return None; // Explicitly safe
                        }
                    }
                }

                return Some(vec![create_finding(
                    msg,
                    self.metadata,
                    context,
                    call.range().start(),
                    severity,
                )]);
            }
        }
        None
    }
}
