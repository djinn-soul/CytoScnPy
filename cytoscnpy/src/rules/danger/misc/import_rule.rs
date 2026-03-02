use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast as ast;
use ruff_text_size::Ranged;

use super::super::utils::create_finding;

/// Rule for detecting insecure module imports.
pub struct InsecureImportRule {
    /// Rule metadata.
    pub metadata: RuleMetadata,
}

impl InsecureImportRule {
    /// Creates a new insecure-import rule instance.
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
        return Some(("Insecure import (xmlrpclib). XMLRPC is vulnerable to XML attacks. Use defusedxml.xmlrpc.", "HIGH"));
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
