//! Dependency declarations from the pre-PEP-621 setuptools files: `setup.py`
//! and `setup.cfg`.
//!
//! Only literal declarations can be recovered. A `setup.py` that computes its
//! requirements at runtime (reading a file, building a list in a loop) yields
//! nothing here, because evaluating it would mean executing arbitrary code.

use super::declared::{extract_pep508_parts, normalize_package_name, DeclaredDependency};
use super::DependencySource;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr};
use ruff_python_parser::parse_module;
use std::path::Path;

/// Keyword arguments of `setup()` that hold requirement lists, and whether the
/// requirements they hold are development-only.
const SETUP_LIST_KEYWORDS: &[(&str, bool)] = &[
    ("install_requires", false),
    ("setup_requires", true),
    ("tests_require", true),
];

fn make_dep(
    spec: &str,
    source: &DependencySource,
    is_dev: bool,
    is_optional: bool,
) -> Option<DeclaredDependency> {
    let (package_name, marker) = extract_pep508_parts(spec)?;
    Some(DeclaredDependency {
        normalized_name: normalize_package_name(&package_name),
        package_name,
        is_dev,
        is_optional,
        marker,
        source: source.clone(),
    })
}

// ──────────────────────────────────────────────────────────────
// setup.py
// ──────────────────────────────────────────────────────────────

/// Extracts requirements from the literal keyword arguments of a `setup()` call.
pub fn parse_setup_py(path: &Path) -> Vec<DeclaredDependency> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(parsed) = parse_module(&content) else {
        return Vec::new();
    };

    let mut collector = SetupCallCollector { deps: Vec::new() };
    for stmt in &parsed.into_syntax().body {
        collector.visit_stmt(stmt);
    }
    collector.deps
}

struct SetupCallCollector {
    deps: Vec<DeclaredDependency>,
}

impl Visitor<'_> for SetupCallCollector {
    fn visit_expr(&mut self, expr: &Expr) {
        if let Expr::Call(call) = expr {
            if is_setup_call(&call.func) {
                self.collect_setup_arguments(&call.arguments);
            }
        }
        visitor::walk_expr(self, expr);
    }
}

/// Matches both `setup(...)` and `setuptools.setup(...)` / `distutils.core.setup(...)`.
fn is_setup_call(func: &Expr) -> bool {
    match func {
        Expr::Name(name) => name.id.as_str() == "setup",
        Expr::Attribute(attr) => attr.attr.as_str() == "setup",
        _ => false,
    }
}

impl SetupCallCollector {
    fn collect_setup_arguments(&mut self, arguments: &ast::Arguments) {
        let source = DependencySource::Setup("setup.py".to_owned());
        for keyword in &arguments.keywords {
            let Some(name) = keyword.arg.as_ref().map(ast::Identifier::as_str) else {
                continue;
            };

            if let Some((_, is_dev)) = SETUP_LIST_KEYWORDS.iter().find(|(kw, _)| *kw == name) {
                self.deps.extend(
                    string_list(&keyword.value)
                        .filter_map(|spec| make_dep(spec, &source, *is_dev, false)),
                );
            } else if name == "extras_require" {
                let Expr::Dict(dict) = &keyword.value else {
                    continue;
                };
                for item in &dict.items {
                    self.deps.extend(
                        string_list(&item.value)
                            .filter_map(|spec| make_dep(spec, &source, false, true)),
                    );
                }
            }
        }
    }
}

/// The string literals of a list/tuple expression; empty for anything else.
fn string_list(expr: &Expr) -> impl Iterator<Item = &str> {
    let elements: &[Expr] = match expr {
        Expr::List(list) => &list.elts,
        Expr::Tuple(tuple) => &tuple.elts,
        _ => &[],
    };
    elements.iter().filter_map(|element| match element {
        Expr::StringLiteral(literal) => Some(literal.value.to_str()),
        _ => None,
    })
}

// ──────────────────────────────────────────────────────────────
// setup.cfg
// ──────────────────────────────────────────────────────────────

/// Whether a `section = key` pair in `setup.cfg` holds requirements, and if so
/// whether they are optional (an extra). In `[options]` only `install_requires`
/// does; in `[options.extras_require]` every key names an extra.
fn requirement_key_is_optional(section: &str, key: &str) -> Option<bool> {
    match section {
        "options" if key == "install_requires" => Some(false),
        "options.extras_require" => Some(true),
        _ => None,
    }
}

/// Extracts requirements from `[options] install_requires` and
/// `[options.extras_require]` in a `setup.cfg`.
pub fn parse_setup_cfg(path: &Path) -> Vec<DeclaredDependency> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let source = DependencySource::Setup("setup.cfg".to_owned());

    let mut deps = Vec::new();
    let mut section = String::new();
    // Set while reading the indented continuation lines of a requirement key;
    // holds whether those requirements are optional.
    let mut continuing: Option<bool> = None;

    for raw_line in content.lines() {
        let line = raw_line.split('#').next().unwrap_or(raw_line);
        let trimmed = line.trim();

        if let Some(name) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            name.trim().clone_into(&mut section);
            continuing = None;
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }

        // An indented line continues the value of the preceding key.
        if raw_line.starts_with([' ', '\t']) {
            if let Some(is_optional) = continuing {
                deps.extend(make_dep(trimmed, &source, false, is_optional));
            }
            continue;
        }

        continuing = None;
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let Some(is_optional) = requirement_key_is_optional(&section, key.trim()) else {
            continue;
        };

        // Either `key = a, b` on one line, or `key =` with indented entries below.
        let value = value.trim();
        if value.is_empty() {
            continuing = Some(is_optional);
        } else {
            deps.extend(
                value
                    .split(',')
                    .filter_map(|spec| make_dep(spec, &source, false, is_optional)),
            );
        }
    }

    deps
}

#[cfg(test)]
#[path = "setup_tests.rs"]
mod tests;
