use super::utils::{create_finding, get_call_name};
use crate::rules::ids;
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{Expr, ExprCall};
use ruff_text_size::Ranged;

/// Rule for detecting insecure XML parsing.
pub const META_XML: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_XML,
    category: super::CAT_INJECTION,
};

/// Rule for detecting insecure XML parsing (XXE/DoS risk).
pub struct XmlRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}

struct XmlFindingDetails {
    message: &'static str,
    severity: &'static str,
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
        let Expr::Call(call) = expr else {
            return None;
        };

        let name_opt = get_call_name(&call.func);
        let attr_name = call_attr_name(call);
        if !is_xml_call(name_opt.as_deref(), attr_name)
            || is_explicitly_safe_lxml(call, name_opt.as_ref())
        {
            return None;
        }

        let details = xml_finding_details(name_opt.as_deref(), attr_name);
        Some(vec![create_finding(
            details.message,
            self.metadata,
            context,
            call.range().start(),
            details.severity,
        )])
    }
}

fn call_attr_name(call: &ExprCall) -> Option<&str> {
    match &*call.func {
        Expr::Attribute(attr) => Some(attr.attr.as_str()),
        _ => None,
    }
}

fn is_xml_call(name: Option<&str>, attr: Option<&str>) -> bool {
    name.map_or_else(|| attr.is_some_and(is_xml_attr), is_xml_full_name)
}

fn is_xml_full_name(name: &str) -> bool {
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
        || matches!(
            name,
            "ET.parse" | "ET.fromstring" | "ET.XML" | "xml.sax.make_parser"
        )
}

fn is_xml_attr(attr: &str) -> bool {
    matches!(
        attr,
        "parse"
            | "fromstring"
            | "XML"
            | "make_parser"
            | "RestrictedElement"
            | "GlobalParserTLS"
            | "getDefaultParser"
            | "check_docinfo"
    )
}

fn xml_finding_details(name: Option<&str>, attr: Option<&str>) -> XmlFindingDetails {
    if name.is_some_and(|name| name.contains("lxml") || name.contains("etree"))
        || attr.is_some_and(is_lxml_parser_attr)
    {
        return XmlFindingDetails {
            message:
                "Insecure XML parsing (resolve_entities is enabled by default in lxml). XXE risk.",
            severity: "HIGH",
        };
    }

    if name.is_some_and(|name| name.contains("sax")) {
        return XmlFindingDetails {
            message: "Insecure XML parsing (SAX is vulnerable to XXE).",
            severity: "MEDIUM",
        };
    }

    if name.is_some_and(|name| name.contains("minidom")) {
        return XmlFindingDetails {
            message: "Insecure XML parsing (minidom is vulnerable to XXE).",
            severity: "MEDIUM",
        };
    }

    XmlFindingDetails {
        message: "Insecure XML parsing (vulnerable to XXE or DoS).",
        severity: "MEDIUM",
    }
}

fn is_lxml_parser_attr(attr: &str) -> bool {
    matches!(
        attr,
        "RestrictedElement" | "GlobalParserTLS" | "getDefaultParser" | "check_docinfo"
    )
}

fn is_explicitly_safe_lxml(call: &ExprCall, name: Option<&String>) -> bool {
    name.is_some_and(|name| name.contains("lxml.etree"))
        && call
            .arguments
            .keywords
            .iter()
            .any(is_false_resolve_entities_keyword)
}

fn is_false_resolve_entities_keyword(keyword: &ruff_python_ast::Keyword) -> bool {
    keyword
        .arg
        .as_ref()
        .is_some_and(|arg| arg == "resolve_entities")
        && matches!(&keyword.value, Expr::BooleanLiteral(value) if !value.value)
}
