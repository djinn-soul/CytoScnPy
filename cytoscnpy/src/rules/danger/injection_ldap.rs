//! LDAP injection detection for python-ldap and ldap3.

use super::super::utils::{create_finding, get_call_name, is_literal_expr};
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;
use rustc_hash::{FxHashMap, FxHashSet};

/// Rule for detecting LDAP injection via non-literal filter strings.
///
/// Detects calls to python-ldap (`ldap.search*`) and ldap3 (`Connection.search`)
/// where the filter argument is not a string literal, enabling LDAP injection.
/// PY104 / OWASP A03:2021.
pub struct LdapInjectionRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
    python_ldap_module_aliases: FxHashSet<String>,
    ldap3_module_aliases: FxHashSet<String>,
    ldap_connection_class_aliases: FxHashSet<String>,
    ldap_connection_kinds: FxHashMap<String, LdapApiKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LdapApiKind {
    PythonLdap,
    Ldap3,
}

impl LdapInjectionRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self {
            metadata,
            python_ldap_module_aliases: FxHashSet::default(),
            ldap3_module_aliases: FxHashSet::default(),
            ldap_connection_class_aliases: FxHashSet::default(),
            ldap_connection_kinds: FxHashMap::default(),
        }
    }

    fn alias_name(alias: &ast::Alias) -> String {
        alias
            .asname
            .as_ref()
            .map_or_else(|| alias.name.to_string(), ToString::to_string)
    }

    fn record_imports(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Import(import_stmt) => {
                for alias in &import_stmt.names {
                    let name = alias.name.as_str();
                    if name == "ldap" {
                        self.python_ldap_module_aliases
                            .insert(Self::alias_name(alias));
                    } else if name == "ldap3" {
                        self.ldap3_module_aliases.insert(Self::alias_name(alias));
                    }
                }
            }
            Stmt::ImportFrom(import_from) => {
                if import_from.level > 0 {
                    return;
                }
                let Some(module) = &import_from.module else {
                    return;
                };
                if module.as_str() == "ldap3" {
                    for alias in &import_from.names {
                        if alias.name.as_str() == "Connection" {
                            self.ldap_connection_class_aliases
                                .insert(Self::alias_name(alias));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn record_connection_assignment(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Assign(assign) => {
                if let Some(kind) = self.connection_factory_kind(&assign.value) {
                    for target in &assign.targets {
                        if let Expr::Name(name) = target {
                            self.ldap_connection_kinds.insert(name.id.to_string(), kind);
                        }
                    }
                }
            }
            Stmt::AnnAssign(assign) => {
                if let Some(kind) = assign
                    .value
                    .as_ref()
                    .and_then(|value| self.connection_factory_kind(value))
                {
                    if let Expr::Name(name) = &*assign.target {
                        self.ldap_connection_kinds.insert(name.id.to_string(), kind);
                    }
                }
            }
            _ => {}
        }
    }

    fn connection_factory_kind(&self, expr: &Expr) -> Option<LdapApiKind> {
        let Expr::Call(call) = expr else {
            return None;
        };

        if let Some(name) = get_call_name(&call.func) {
            if self.ldap_connection_class_aliases.contains(&name) {
                return Some(LdapApiKind::Ldap3);
            }
            if name.rsplit_once('.').is_some_and(|(module, attr)| {
                attr == "Connection" && self.ldap3_module_aliases.contains(module)
            }) {
                return Some(LdapApiKind::Ldap3);
            }
            if name.rsplit_once('.').is_some_and(|(module, attr)| {
                attr == "initialize" && self.python_ldap_module_aliases.contains(module)
            }) {
                return Some(LdapApiKind::PythonLdap);
            }
        }

        None
    }

    fn connection_receiver_kind(&self, expr: &Expr) -> Option<LdapApiKind> {
        match expr {
            Expr::Name(name) => self.ldap_connection_kinds.get(name.id.as_str()).copied(),
            Expr::Call(_) => self.connection_factory_kind(expr),
            _ => None,
        }
    }

    /// Resolves an attribute call to (kind, filter-positional-index) when it
    /// matches an LDAP search API. `python-ldap` puts the filter at index 2
    /// (`search_s(base, scope, filterstr, ...)`) while `ldap3` puts it at
    /// index 1 (`Connection.search(search_base, search_filter, ...)`).
    fn classify_ldap_search_call(&self, func: &Expr) -> Option<usize> {
        let Expr::Attribute(attr) = func else {
            return None;
        };
        let method = attr.attr.as_str();
        let receiver = &attr.value;

        // python-ldap module-level or connection-bound search APIs.
        if matches!(
            method,
            "search_s" | "search_st" | "search_ext" | "search_ext_s"
        ) {
            if let Expr::Name(name) = &**receiver {
                if self.python_ldap_module_aliases.contains(name.id.as_str()) {
                    return Some(2);
                }
            }
            if self.connection_receiver_kind(receiver).is_some() {
                return Some(2);
            }
            return None;
        }

        // python-ldap uses conn.search(base, scope, filterstr, ...), while
        // ldap3 uses Connection.search(search_base, search_filter, ...).
        if method == "search" {
            return match self.connection_receiver_kind(receiver)? {
                LdapApiKind::PythonLdap => Some(2),
                LdapApiKind::Ldap3 => Some(1),
            };
        }

        None
    }
}

impl Rule for LdapInjectionRule {
    fn name(&self) -> &'static str {
        "LdapInjectionRule"
    }

    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }

    fn enter_stmt(&mut self, stmt: &Stmt, _context: &Context) -> Option<Vec<Finding>> {
        self.record_imports(stmt);
        self.record_connection_assignment(stmt);
        None
    }

    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        let Expr::Call(call) = expr else {
            return None;
        };

        let filter_index = self.classify_ldap_search_call(&call.func)?;

        // Check kwarg filterstr / search_filter for non-literal value
        for kw in &call.arguments.keywords {
            if let Some(arg) = &kw.arg {
                if matches!(arg.as_str(), "filterstr" | "search_filter")
                    && !is_literal_expr(&kw.value)
                {
                    return Some(vec![create_finding(
                        "Potential LDAP injection: non-literal filter string passed to LDAP search. Use ldap.filter.escape_filter_chars() to sanitize.",
                        self.metadata,
                        context,
                        call.range().start(),
                        "HIGH",
                    )]);
                }
            }
        }

        // Positional filter index depends on the API family — python-ldap uses
        // index 2, ldap3 uses index 1. `classify_ldap_search_call` already
        // disambiguated and handed us the right slot.
        if let Some(filter_arg) = call.arguments.args.get(filter_index) {
            if !is_literal_expr(filter_arg) {
                return Some(vec![create_finding(
                    "Potential LDAP injection: non-literal filter string passed to LDAP search. Use ldap.filter.escape_filter_chars() to sanitize.",
                    self.metadata,
                    context,
                    call.range().start(),
                    "HIGH",
                )]);
            }
        }

        None
    }
}
