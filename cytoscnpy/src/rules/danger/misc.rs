use super::crypto::check_ciphers_and_modes;
use super::network::check_network_and_ssl;
use super::utils::{contains_sensitive_names, create_finding, get_call_name};
use crate::rules::ids;
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

/// Rule for detecting the use of `assert` in production code.
pub const META_ASSERT: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_ASSERT,
    category: super::CAT_BEST_PRACTICES,
};
/// Rule for detecting insecure module imports (e.g., telnetlib, ftplib).
pub const META_INSECURE_IMPORT: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_INSECURE_IMPORT,
    category: super::CAT_BEST_PRACTICES,
};
/// Rule for detecting disabled Jinja2 autoescaping.
pub const META_JINJA_AUTOESCAPE: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_JINJA_AUTOESCAPE,
    category: super::CAT_BEST_PRACTICES,
};
/// Rule for detecting calls to blacklisted functions.
pub const META_BLACKLIST: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_BLACKLIST,
    category: super::CAT_BEST_PRACTICES,
};
/// Rule for detecting logging of sensitive data.
pub const META_LOGGING_SENSITIVE: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_LOGGING_SENSITIVE,
    category: super::CAT_PRIVACY,
};
/// Rule for detecting use of `input()` (vulnerable in Py2, dangerous in Py3).
pub const META_INPUT: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_INPUT,
    category: super::CAT_CODE_EXEC,
};

/// Rule for detecting the use of `assert` in production code.
pub struct AssertUsedRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl AssertUsedRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for AssertUsedRule {
    fn name(&self) -> &'static str {
        "AssertUsedRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn enter_stmt(&mut self, stmt: &ast::Stmt, context: &Context) -> Option<Vec<Finding>> {
        if matches!(stmt, ast::Stmt::Assert(_)) {
            return Some(vec![create_finding(
                "Use of assert detected. The enclosed code will be removed when compiling to optimised byte code.",
                self.metadata,
                context,
                stmt.range().start(),
                "LOW",
            )]);
        }
        None
    }
}

/// Rule for detecting if debug mode is enabled in production.
pub struct DebugModeRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl DebugModeRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for DebugModeRule {
    fn name(&self) -> &'static str {
        "DebugModeRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    // This lint is a false positive - we're checking Python method names, not file extensions
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name.ends_with(".run") || name == "run_simple" {
                    for keyword in &call.arguments.keywords {
                        if let Some(arg) = &keyword.arg {
                            if arg == "debug" {
                                if let Expr::BooleanLiteral(b) = &keyword.value {
                                    if b.value {
                                        return Some(vec![create_finding(
                                            "Debug mode enabled (debug=True) in production",
                                            self.metadata,
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
        }
        None
    }
}

/// Rule for detecting disabled autoescaping in Jinja2 templates.
pub struct Jinja2AutoescapeRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl Jinja2AutoescapeRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for Jinja2AutoescapeRule {
    fn name(&self) -> &'static str {
        "Jinja2AutoescapeRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "jinja2.Environment" || name == "Environment" {
                    for keyword in &call.arguments.keywords {
                        if let Some(arg) = &keyword.arg {
                            if arg == "autoescape" {
                                if let Expr::BooleanLiteral(b) = &keyword.value {
                                    if !b.value {
                                        return Some(vec![create_finding(
                                            "jinja2.Environment created with autoescape=False. This enables XSS attacks.",
                                            self.metadata,
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
        }
        None
    }
}

/// Rule for detecting blacklisted function calls.
pub struct BlacklistCallRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl BlacklistCallRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for BlacklistCallRule {
    fn name(&self) -> &'static str {
        "BlacklistCallRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if let Some(finding) = check_ciphers_and_modes(&name, call, context) {
                    return Some(vec![finding]);
                }
                if let Some(finding) = check_network_and_ssl(&name, call, context) {
                    return Some(vec![finding]);
                }
                if let Some(finding) = check_misc_blacklist(&name, call, context) {
                    return Some(vec![finding]);
                }
            }
        }
        None
    }
}

/// Check for miscellaneous blacklisted calls (B308, B322, B325)
fn check_misc_blacklist(name: &str, call: &ast::ExprCall, context: &Context) -> Option<Finding> {
    use super::filesystem::META_TEMPNAM;
    use super::injection::META_MARK_SAFE;
    // use crate::rules::danger::{META_INPUT, META_MARK_SAFE, META_TEMPNAM};

    // B308: mark_safe
    if name == "mark_safe" || name == "django.utils.safestring.mark_safe" {
        return Some(create_finding(
            "Use of mark_safe() may expose XSS. Review carefully.",
            META_MARK_SAFE,
            context,
            call.range().start(),
            "MEDIUM",
        ));
    }
    // B322: input (python 2 mainly, but bad practice)
    if name == "input" {
        return Some(create_finding(
            "Check for use of input() (vulnerable in Py2, unsafe in Py3 if not careful).",
            META_INPUT,
            context,
            call.range().start(),
            "HIGH",
        ));
    }
    // B325: tempnam (vulnerable to symlink attacks)
    if name == "os.tempnam" || name == "os.tmpnam" {
        return Some(create_finding(
            "Use of os.tempnam/os.tmpnam is vulnerable to symlink attacks. Use tempfile module instead.",
            META_TEMPNAM,
            context,
            call.range().start(),
            "MEDIUM",
        ));
    }
    None
}

/// Rule for detecting logging of potentially sensitive data.
pub struct LoggingSensitiveDataRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl LoggingSensitiveDataRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for LoggingSensitiveDataRule {
    fn name(&self) -> &'static str {
        "LoggingSensitiveDataRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    // This lint is a false positive - we're checking Python method names, not file extensions
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name.starts_with("logging.")
                    || name.starts_with("logger.")
                    || name == "log"
                    || name.ends_with(".debug")
                    || name.ends_with(".info")
                    || name.ends_with(".warning")
                    || name.ends_with(".error")
                    || name.ends_with(".critical")
                {
                    for arg in &call.arguments.args {
                        if contains_sensitive_names(arg) {
                            return Some(vec![create_finding(
                                "Potential sensitive data in log statement. Avoid logging passwords, tokens, secrets, or API keys.",
                                self.metadata,
                                context,
                                call.range().start(),
                                "MEDIUM",
                            )]);
                        }
                    }
                }
            }
        }
        None
    }
}

/// Rule for detecting insecure module imports.
pub struct InsecureImportRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl InsecureImportRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for InsecureImportRule {
    fn name(&self) -> &'static str {
        "InsecureImportRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn enter_stmt(&mut self, stmt: &ast::Stmt, context: &Context) -> Option<Vec<Finding>> {
        match stmt {
            ast::Stmt::Import(node) => {
                let mut findings = Vec::new();
                for name in &node.names {
                    if let Some((msg, severity)) = check_insecure_module(&name.name.id) {
                        findings.push(create_finding(
                            msg,
                            self.metadata,
                            context,
                            name.range().start(),
                            severity,
                        ));
                    }
                }
                if !findings.is_empty() {
                    return Some(findings);
                }
            }
            ast::Stmt::ImportFrom(node) => {
                let module_name = node
                    .module
                    .as_ref()
                    .map(ruff_python_ast::Identifier::as_str)
                    .unwrap_or("");

                if let Some((msg, severity)) = check_insecure_module(module_name) {
                    return Some(vec![create_finding(
                        msg,
                        self.metadata,
                        context,
                        node.range().start(),
                        severity,
                    )]);
                }

                let mut findings = Vec::new();
                for name in &node.names {
                    let full_name = if module_name.is_empty() {
                        name.name.id.to_string()
                    } else {
                        format!("{}.{}", module_name, name.name.id)
                    };

                    if let Some((msg, severity)) = check_insecure_module(&full_name) {
                        findings.push(create_finding(
                            msg,
                            self.metadata,
                            context,
                            name.range().start(),
                            severity,
                        ));
                    }
                }
                if !findings.is_empty() {
                    return Some(findings);
                }
            }
            _ => {}
        }
        None
    }
}

fn check_insecure_module(name: &str) -> Option<(&'static str, &'static str)> {
    if name == "telnetlib" {
        return Some((
            "Insecure import (telnetlib). Telnet is unencrypted and considered insecure. Use SSH.",
            "HIGH",
        ));
    }
    if name == "ftplib" {
        return Some(("Insecure import (ftplib). FTP is unencrypted and considered insecure. Use SSH/SFTP/SCP.", "HIGH"));
    }
    if name == "pyghmi" {
        return Some((
            "Insecure import (pyghmi). IPMI is considered insecure. Use an encrypted protocol.",
            "HIGH",
        ));
    }
    if name.starts_with("Crypto.") || name == "Crypto" {
        return Some(("Insecure import (pycrypto). PyCrypto is unmaintained and contains vulnerabilities. Use pyca/cryptography.", "HIGH"));
    }
    if name == "xmlrpc" || name.starts_with("xmlrpc.") {
        return Some(("Insecure import (xmlrpc). XMLRPC is vulnerable to XML attacks. Use defusedxml.xmlrpc.monkey_patch().", "HIGH"));
    }
    if name == "wsgiref.handlers.CGIHandler" || name == "twisted.web.twcgi.CGIScript" {
        return Some((
            "Insecure import (httpoxy). CGI usage is vulnerable to httpoxy attacks.",
            "HIGH",
        ));
    }
    if name == "wsgiref" {
        return Some((
            "Insecure import (wsgiref). Ensure CGIHandler is not used (httpoxy vulnerability).",
            "LOW",
        ));
    }
    if name == "xmlrpclib" {
        return Some((
            "Insecure import (xmlrpclib). XMLRPC is vulnerable to XML attacks. Use defusedxml.xmlrpc.",
            "HIGH",
        ));
    }

    if matches!(name, "pickle" | "cPickle" | "dill" | "shelve") {
        return Some((
            "Consider possible security implications of pickle/deserialization modules.",
            "LOW",
        ));
    }

    if name == "subprocess" {
        return Some((
            "Consider possible security implications of subprocess module.",
            "LOW",
        ));
    }

    if matches!(
        name,
        "xml.etree.cElementTree"
            | "xml.etree.ElementTree"
            | "xml.sax"
            | "xml.dom.expatbuilder"
            | "xml.dom.minidom"
            | "xml.dom.pulldom"
            | "lxml"
    ) || name.starts_with("xml.etree")
        || name.starts_with("xml.sax")
        || name.starts_with("xml.dom")
        || name.starts_with("lxml")
    {
        return Some((
            "Using XML parsing modules may be vulnerable to XML attacks. Consider defusedxml.",
            "LOW",
        ));
    }

    None
}
