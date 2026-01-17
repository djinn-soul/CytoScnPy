use super::crypto::{check_ciphers_and_modes, check_marshal_and_hashes};
use super::network::check_network_and_ssl;
use super::utils::{contains_sensitive_names, create_finding, get_call_name};
use crate::rules::{Context, Finding, Rule};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

pub struct AssertUsedRule;
impl Rule for AssertUsedRule {
    fn name(&self) -> &'static str {
        "AssertUsedRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D105"
    }
    fn enter_stmt(&mut self, stmt: &ast::Stmt, context: &Context) -> Option<Vec<Finding>> {
        if matches!(stmt, ast::Stmt::Assert(_)) {
            return Some(vec![create_finding(
                "Use of assert detected. The enclosed code will be removed when compiling to optimised byte code.",
                self.code(),
                context,
                stmt.range().start(),
                "LOW",
            )]);
        }
        None
    }
}

pub struct DebugModeRule;
impl Rule for DebugModeRule {
    fn name(&self) -> &'static str {
        "DebugModeRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D403"
    }
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
        }
        None
    }
}

pub struct Jinja2AutoescapeRule;
impl Rule for Jinja2AutoescapeRule {
    fn name(&self) -> &'static str {
        "Jinja2AutoescapeRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D106"
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
        }
        None
    }
}

pub struct BlacklistCallRule;
impl Rule for BlacklistCallRule {
    fn name(&self) -> &'static str {
        "BlacklistCallRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D800"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if let Some(finding) = check_marshal_and_hashes(&name, call, context) {
                    return Some(vec![finding]);
                }
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
    // B308: mark_safe
    if name == "mark_safe" || name == "django.utils.safestring.mark_safe" {
        return Some(create_finding(
            "Use of mark_safe() may expose XSS. Review carefully.",
            "CSP-D107",
            context,
            call.range().start(),
            "MEDIUM",
        ));
    }
    // B322: input (python 2 mainly, but bad practice)
    if name == "input" {
        return Some(create_finding(
            "Check for use of input() (vulnerable in Py2, unsafe in Py3 if not careful).",
            "CSP-D007",
            context,
            call.range().start(),
            "HIGH",
        ));
    }
    // B325: tempnam (vulnerable to symlink attacks)
    if name == "os.tempnam" || name == "os.tmpnam" {
        return Some(create_finding(
            "Use of os.tempnam/os.tmpnam is vulnerable to symlink attacks. Use tempfile module instead.",
            "CSP-D506",
            context,
            call.range().start(),
            "MEDIUM",
        ));
    }
    None
}

pub struct LoggingSensitiveDataRule;
impl Rule for LoggingSensitiveDataRule {
    fn name(&self) -> &'static str {
        "LoggingSensitiveDataRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D903"
    }
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
                                self.code(),
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

pub struct InsecureImportRule;
impl Rule for InsecureImportRule {
    fn name(&self) -> &'static str {
        "InsecureImportRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D004"
    }
    fn enter_stmt(&mut self, stmt: &ast::Stmt, context: &Context) -> Option<Vec<Finding>> {
        match stmt {
            ast::Stmt::Import(node) => {
                let mut findings = Vec::new();
                for name in &node.names {
                    if let Some((msg, severity)) = check_insecure_module(&name.name.id) {
                        findings.push(create_finding(
                            msg,
                            self.code(),
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
                        self.code(),
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
                            self.code(),
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
