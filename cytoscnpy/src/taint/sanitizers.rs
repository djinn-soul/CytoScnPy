use super::types::VulnType;
use ruff_python_ast::{self as ast, Expr};

/// How a sanitizer affects taint flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SanitizerKind {
    /// The call's returned value is sanitized.
    ReturnValue,
    /// The call sanitizes arguments only in its truthy branch.
    Guard,
    /// The call sanitizes arguments after it returns successfully.
    SideEffect,
}

/// A configured sanitizer and the vulnerability class it addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanitizerSpec {
    /// Fully qualified or local function name.
    pub pattern: String,
    /// Vulnerability class neutralized by the function.
    pub vuln_type: VulnType,
}

/// Rule-scoped sanitizer configuration used by taint analysis.
#[derive(Debug, Clone, Default)]
pub struct SanitizerConfig {
    /// Return-value sanitizer specifications.
    pub return_value: Vec<SanitizerSpec>,
    /// Guard sanitizer specifications.
    pub guard: Vec<SanitizerSpec>,
    /// Side-effect sanitizer specifications.
    pub side_effect: Vec<SanitizerSpec>,
}

impl SanitizerConfig {
    /// Converts user-facing grouped configuration into taint sanitizer specifications.
    #[must_use]
    pub fn from_project_config(config: &crate::config::SanitizerConfig) -> Self {
        let mut sanitizers = Self::default();
        sanitizers.add_group(
            &VulnType::Ssrf,
            &config.ssrf.return_value,
            &config.ssrf.guard,
            &config.ssrf.side_effect,
        );
        sanitizers.add_group(
            &VulnType::PathTraversal,
            &config.path_traversal.return_value,
            &config.path_traversal.guard,
            &config.path_traversal.side_effect,
        );
        sanitizers.add_group(
            &VulnType::SqlInjection,
            &config.sql_injection.return_value,
            &config.sql_injection.guard,
            &config.sql_injection.side_effect,
        );
        sanitizers.add_group(
            &VulnType::CommandInjection,
            &config.command_injection.return_value,
            &config.command_injection.guard,
            &config.command_injection.side_effect,
        );
        sanitizers.add_group(
            &VulnType::CodeInjection,
            &config.code_injection.return_value,
            &config.code_injection.guard,
            &config.code_injection.side_effect,
        );
        sanitizers
    }

    /// Converts danger configuration into taint sanitizer specifications.
    #[must_use]
    pub fn from_danger_config(config: &crate::config::DangerConfig) -> Self {
        let mut sanitizers = Self::from_project_config(&config.sanitizers);
        if let Some(legacy) = &config.custom_sanitizers {
            sanitizers.add_legacy_global(legacy);
        }
        sanitizers
    }

    /// Adds one vulnerability-scoped sanitizer group.
    pub fn add_group(
        &mut self,
        vuln_type: &VulnType,
        return_value: &[String],
        guard: &[String],
        side_effect: &[String],
    ) {
        extend_specs(&mut self.return_value, return_value, vuln_type);
        extend_specs(&mut self.guard, guard, vuln_type);
        extend_specs(&mut self.side_effect, side_effect, vuln_type);
    }

    fn add_legacy_global(&mut self, patterns: &[String]) {
        for vuln_type in legacy_global_vuln_types() {
            extend_specs(&mut self.return_value, patterns, &vuln_type);
        }
    }

    /// Returns the vulnerability classes matched by a call in the requested mode.
    #[must_use]
    pub fn matching_types(&self, call: &ast::ExprCall, kind: SanitizerKind) -> Vec<VulnType> {
        let specs = match kind {
            SanitizerKind::ReturnValue => &self.return_value,
            SanitizerKind::Guard => &self.guard,
            SanitizerKind::SideEffect => &self.side_effect,
        };
        let Some(name) = call_name(&call.func) else {
            return Vec::new();
        };

        unique_types(
            specs
                .iter()
                .filter(|spec| pattern_matches(&spec.pattern, &name))
                .map(|spec| spec.vuln_type.clone()),
        )
    }
}

/// Returns strict built-in return-value sanitizer classes for a call.
#[must_use]
pub fn builtin_return_types(call: &ast::ExprCall) -> Vec<VulnType> {
    let Some(name) = call_name(&call.func) else {
        return Vec::new();
    };

    match name.as_str() {
        "html.escape"
        | "escape"
        | "cgi.escape"
        | "markupsafe.escape"
        | "flask.escape"
        | "django.utils.html.escape"
        | "bleach.clean" => vec![VulnType::Xss],
        "shlex.quote" | "shlex.split" => vec![VulnType::CommandInjection],
        "urllib.parse.quote" | "quote" => vec![VulnType::Ssrf],
        "int" | "float" | "bool" => vec![
            VulnType::SqlInjection,
            VulnType::CommandInjection,
            VulnType::PathTraversal,
        ],
        _ => Vec::new(),
    }
}

/// Returns direct variable arguments passed to a sanitizer call.
#[must_use]
pub fn call_argument_names(call: &ast::ExprCall) -> Vec<String> {
    let positional = call.arguments.args.iter().filter_map(|arg| match arg {
        Expr::Name(name) => Some(name.id.to_string()),
        _ => None,
    });
    let keywords = call.arguments.keywords.iter().filter_map(|keyword| {
        if let Expr::Name(name) = &keyword.value {
            Some(name.id.to_string())
        } else {
            None
        }
    });
    positional.chain(keywords).collect()
}

/// Resolves a dotted Python call name.
#[must_use]
pub fn call_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Name(name) => Some(name.id.to_string()),
        Expr::Attribute(attr) => {
            let base = call_name(&attr.value)?;
            Some(format!("{base}.{}", attr.attr))
        }
        _ => None,
    }
}

fn extend_specs(target: &mut Vec<SanitizerSpec>, patterns: &[String], vuln_type: &VulnType) {
    target.extend(patterns.iter().map(|pattern| SanitizerSpec {
        pattern: pattern.clone(),
        vuln_type: vuln_type.clone(),
    }));
}

fn pattern_matches(pattern: &str, call_name: &str) -> bool {
    pattern == call_name
        || (!pattern.contains('.')
            && call_name
                .rsplit('.')
                .next()
                .is_some_and(|local| local == pattern))
}

pub(crate) fn unique_types(types: impl IntoIterator<Item = VulnType>) -> Vec<VulnType> {
    let mut unique = Vec::new();
    for vuln_type in types {
        if !unique.contains(&vuln_type) {
            unique.push(vuln_type);
        }
    }
    unique
}

pub(crate) fn legacy_global_vuln_types() -> Vec<VulnType> {
    vec![
        VulnType::Ssrf,
        VulnType::PathTraversal,
        VulnType::SqlInjection,
        VulnType::CommandInjection,
        VulnType::CodeInjection,
    ]
}

pub(crate) fn has_builtin_command_sanitizer(source: &str) -> bool {
    source.contains("shlex.quote(") || source.contains("shlex.split(")
}
