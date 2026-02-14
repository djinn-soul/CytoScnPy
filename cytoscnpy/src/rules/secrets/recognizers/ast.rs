use super::types::{is_test_name, RawFinding, SecretRecognizer};
use crate::utils::LineIndex;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;
use std::path::PathBuf;

/// Suspicious variable name patterns.
const SUSPICIOUS_NAMES: &[&str] = &[
    "password",
    "passwd",
    "pwd",
    "secret",
    "key",
    "token",
    "auth",
    "credential",
    "api_key",
    "apikey",
    "private_key",
    "access_token",
    "secret_key",
    "auth_token",
    "bearer",
    "client_secret",
    "app_secret",
    "encryption_key",
    "signing_key",
    "master_key",
];

/// AST-based suspicious variable name detection recognizer.
pub struct AstRecognizer {
    /// Additional suspicious names from config.
    custom_names: Vec<String>,
}

impl Default for AstRecognizer {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl AstRecognizer {
    /// Creates a new AST recognizer with custom suspicious names.
    #[must_use]
    pub fn new(custom_names: Vec<String>) -> Self {
        Self { custom_names }
    }

    /// Check if a name matches suspicious patterns.
    fn matches_suspicious_name(&self, name: &str) -> bool {
        let lower = name.to_lowercase();

        // 1. Check exclusions (Safe patterns)
        const SAFE_NAME_SUBSTRINGS: &[&str] = &[
            "keyboard",
            "keyword",
            "monkey",
            "donkey",
            "tracking_id",
            "uuid",
            "public",
            "example",
            "sample",
        ];

        if SAFE_NAME_SUBSTRINGS.iter().any(|&s| lower.contains(s))
            || is_test_name(&lower)
            || lower.ends_with("_regex")
            || lower.ends_with("_pattern")
            || lower.ends_with("_re")
            || lower.ends_with("_fmt")
            || lower.ends_with("_format")
        {
            return false;
        }

        if lower.contains("jwt") && lower.contains("token") {
            return false;
        }

        // 2. Check built-in patterns with word boundary awareness
        for &pattern in SUSPICIOUS_NAMES {
            // Use match_indices for more idiomatic and efficient matching
            for (absolute_idx, _) in lower.match_indices(pattern) {
                // For short or common keywords, enforce word boundaries
                // to avoid matching 'keyboard', 'monkey', 'donkey', etc.
                if matches!(pattern, "key" | "pwd" | "auth" | "token") {
                    // Check if it's a standalone word or part of a snake_case/camelCase identifier
                    let before = if absolute_idx > 0 {
                        lower.as_bytes().get(absolute_idx - 1).map(|&b| b as char)
                    } else {
                        None
                    };
                    let after = lower
                        .as_bytes()
                        .get(absolute_idx + pattern.len())
                        .map(|&b| b as char);

                    let boundary_before = before.map_or(true, |c| !c.is_alphanumeric());
                    let boundary_after = after.map_or(true, |c| !c.is_alphanumeric());
                    let camel_boundary_before = Self::is_camel_boundary_before(name, absolute_idx);
                    let camel_boundary_after =
                        Self::is_camel_boundary_after(name, absolute_idx + pattern.len());

                    if (boundary_before || camel_boundary_before)
                        && (boundary_after || camel_boundary_after)
                    {
                        return true;
                    }
                } else {
                    // Not a length-sensitive keyword, any match is suspicious
                    return true;
                }
            }
        }

        // 3. Check custom patterns
        self.custom_names
            .iter()
            .any(|s| lower.contains(&s.to_lowercase()))
    }

    fn is_camel_boundary_before(name: &str, start: usize) -> bool {
        if !name.is_ascii() || start == 0 {
            return false;
        }
        let bytes = name.as_bytes();
        let current = bytes[start] as char;
        current.is_ascii_uppercase()
    }

    fn is_camel_boundary_after(name: &str, end: usize) -> bool {
        if !name.is_ascii() || end >= name.len() || end == 0 {
            return false;
        }
        let bytes = name.as_bytes();
        let before = bytes[end - 1] as char;
        let after = bytes[end] as char;
        before.is_ascii_lowercase() && after.is_ascii_uppercase()
    }

    /// Extract string value from an expression if it's a literal string.
    fn extract_string_value(expr: &Expr) -> Option<String> {
        match expr {
            Expr::StringLiteral(s) => Some(s.value.to_string()),
            _ => None,
        }
    }

    /// Check if the value is from an environment variable access.
    fn is_env_var_access(expr: &Expr) -> bool {
        match expr {
            Expr::Call(call) => {
                // Check for os.environ.get(...) or os.getenv(...)
                match &*call.func {
                    Expr::Attribute(attr) => {
                        let attr_name = attr.attr.as_str();
                        if attr_name == "get" {
                            // Check if it's environ.get
                            if let Expr::Attribute(inner) = &*attr.value {
                                return inner.attr.as_str() == "environ";
                            }
                        }
                        if attr_name == "getenv" {
                            // Check if it's os.getenv
                            if let Expr::Name(name) = &*attr.value {
                                return name.id.as_str() == "os";
                            }
                        }
                        false
                    }
                    Expr::Name(name) => {
                        // Direct getenv call (from os import getenv)
                        name.id.as_str() == "getenv"
                    }
                    _ => false,
                }
            }
            Expr::Subscript(sub) => {
                // Check for os.environ[...]
                if let Expr::Attribute(attr) = &*sub.value {
                    return attr.attr.as_str() == "environ";
                }
                false
            }
            _ => false,
        }
    }

    /// Redact a secret value.
    fn redact_value(s: &str) -> String {
        if s.len() <= 8 {
            return "*".repeat(s.len());
        }
        let start: String = s.chars().take(4).collect();
        let end: String = s
            .chars()
            .rev()
            .take(4)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        format!("{start}...{end}")
    }

    /// Check if value looks like a placeholder.
    fn is_placeholder(value: &str) -> bool {
        let lower = value.to_lowercase();
        lower.starts_with("xxx")
            || lower.starts_with("your_")
            || lower.starts_with("changeme")
            || lower.starts_with("replace_")
            || lower.starts_with("example")
            || lower.starts_with('<')
            || lower.contains("${")
            || lower.contains("{{")
            || lower == "none"
            || lower == "null"
            || lower.is_empty()
    }

    fn process_assign(
        &self,
        targets: &[Expr],
        value: &Expr,
        line: usize,
        findings: &mut Vec<RawFinding>,
    ) {
        if Self::is_env_var_access(value) {
            return;
        }

        let Some(string_value) = Self::extract_string_value(value) else {
            return;
        };

        if Self::is_placeholder(&string_value) {
            return;
        }

        for target in targets {
            let name = match target {
                Expr::Name(name) => name.id.to_string(),
                Expr::Attribute(attr) => attr.attr.to_string(),
                Expr::Subscript(sub) => {
                    if let Expr::StringLiteral(key) = &*sub.slice {
                        key.value.to_string()
                    } else {
                        continue;
                    }
                }
                _ => continue,
            };

            if self.matches_suspicious_name(&name) {
                findings.push(RawFinding {
                    message: format!("Suspicious assignment to '{name}'"),
                    rule_id: "CSP-S300".to_owned(),
                    line,
                    base_score: 70,
                    matched_value: Some(Self::redact_value(&string_value)),
                    entropy: None,
                    severity: "MEDIUM".to_owned(),
                });
            }
        }
    }

    fn visit_stmts(&self, stmts: &[Stmt], line_index: &LineIndex, findings: &mut Vec<RawFinding>) {
        for stmt in stmts {
            self.visit_stmt(stmt, line_index, findings);
        }
    }

    fn visit_stmt(&self, stmt: &Stmt, line_index: &LineIndex, findings: &mut Vec<RawFinding>) {
        match stmt {
            Stmt::Assign(node) => {
                let line = line_index.line_index(node.start());
                self.process_assign(&node.targets, &node.value, line, findings);
            }
            Stmt::AnnAssign(node) => {
                if let Some(value) = &node.value {
                    let line = line_index.line_index(node.start());
                    self.process_assign(&[(*node.target).clone()], value, line, findings);
                }
            }
            // Recurse into compound statements
            Stmt::FunctionDef(node) => {
                self.visit_stmts(&node.body, line_index, findings);
            }
            Stmt::ClassDef(node) => {
                self.visit_stmts(&node.body, line_index, findings);
            }
            Stmt::If(node) => {
                self.visit_stmts(&node.body, line_index, findings);
                for clause in &node.elif_else_clauses {
                    self.visit_stmts(&clause.body, line_index, findings);
                }
            }
            Stmt::For(node) => {
                self.visit_stmts(&node.body, line_index, findings);
                self.visit_stmts(&node.orelse, line_index, findings);
            }
            Stmt::While(node) => {
                self.visit_stmts(&node.body, line_index, findings);
                self.visit_stmts(&node.orelse, line_index, findings);
            }
            Stmt::With(node) => {
                self.visit_stmts(&node.body, line_index, findings);
            }
            Stmt::Try(node) => {
                self.visit_stmts(&node.body, line_index, findings);
                for ast::ExceptHandler::ExceptHandler(h) in &node.handlers {
                    self.visit_stmts(&h.body, line_index, findings);
                }
                self.visit_stmts(&node.orelse, line_index, findings);
                self.visit_stmts(&node.finalbody, line_index, findings);
            }
            Stmt::Match(node) => {
                for case in &node.cases {
                    self.visit_stmts(&case.body, line_index, findings);
                }
            }
            _ => {}
        }
    }
}

impl SecretRecognizer for AstRecognizer {
    fn name(&self) -> &'static str {
        "AstRecognizer"
    }

    fn base_score(&self) -> u8 {
        70 // Medium-high confidence
    }

    fn scan_text(&self, _content: &str, _file_path: &PathBuf) -> Vec<RawFinding> {
        // AST recognizer doesn't use text scanning
        Vec::new()
    }

    fn scan_ast(
        &self,
        stmts: &[Stmt],
        _file_path: &PathBuf,
        line_index: &LineIndex,
    ) -> Vec<RawFinding> {
        let mut findings = Vec::new();
        self.visit_stmts(stmts, line_index, &mut findings);
        findings
    }
}
