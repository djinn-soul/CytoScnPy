//! Filters conventional constants out of module-level mutable collection findings.

use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{Expr, Stmt};
use rustc_hash::FxHashSet;

use super::super::types::{GlobalKind, GlobalMatch};

const MUTATING_METHODS: &[&str] = &[
    "append",
    "appendleft",
    "extend",
    "extendleft",
    "insert",
    "pop",
    "popleft",
    "popitem",
    "remove",
    "clear",
    "update",
    "setdefault",
    "add",
    "discard",
    "sort",
    "reverse",
    "rotate",
];

/// Returns `true` for dunder names such as `__all__`, which describe the module itself.
pub(super) fn is_dunder(name: &str) -> bool {
    name.len() > 4 && name.starts_with("__") && name.ends_with("__")
}

fn is_constant_name(name: &str) -> bool {
    name.chars().any(|c| c.is_ascii_uppercase())
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Drops dunder collections and `UPPER_CASE` collections the module never mutates.
pub(super) fn retain_mutable_collections(body: &[Stmt], matches: &mut Vec<GlobalMatch>) {
    let mut collector = MutationCollector::default();
    collector.visit_body(body);
    matches.retain(|m| match m.kind {
        GlobalKind::ModuleCollection => {
            !is_dunder(&m.name)
                && (!is_constant_name(&m.name) || collector.mutated.contains(&m.name))
        }
        GlobalKind::ClassVariable => !m.name.rsplit("::").next().is_some_and(is_dunder),
        _ => true,
    });
}

#[derive(Default)]
struct MutationCollector {
    mutated: FxHashSet<String>,
}

impl MutationCollector {
    fn record_container(&mut self, expr: &Expr) {
        match expr {
            Expr::Name(name) => {
                self.mutated.insert(name.id.to_string());
            }
            Expr::Subscript(sub) => self.record_container(&sub.value),
            _ => {}
        }
    }

    fn record_target(&mut self, target: &Expr) {
        match target {
            Expr::Subscript(sub) => self.record_container(&sub.value),
            Expr::Tuple(t) => t.elts.iter().for_each(|e| self.record_target(e)),
            Expr::List(l) => l.elts.iter().for_each(|e| self.record_target(e)),
            _ => {}
        }
    }
}

impl<'a> Visitor<'a> for MutationCollector {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::Assign(assign) => assign.targets.iter().for_each(|t| self.record_target(t)),
            Stmt::AnnAssign(ann) if ann.value.is_some() => self.record_target(&ann.target),
            Stmt::AugAssign(aug) => self.record_container(&aug.target),
            Stmt::Delete(del) => del.targets.iter().for_each(|t| self.record_target(t)),
            Stmt::Global(global) => {
                self.mutated
                    .extend(global.names.iter().map(|n| n.id.to_string()));
            }
            _ => {}
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(call) = expr {
            if let Expr::Attribute(attr) = call.func.as_ref() {
                if MUTATING_METHODS.contains(&attr.attr.as_str()) {
                    self.record_container(&attr.value);
                }
            }
        }
        visitor::walk_expr(self, expr);
    }
}
