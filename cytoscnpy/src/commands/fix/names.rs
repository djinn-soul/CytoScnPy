//! Allocate discard bindings without shadowing names already present in the module.

use ruff_python_ast::visitor::source_order::{self, SourceOrderVisitor};
use ruff_python_ast::{Expr, Identifier, ModModule};
use std::collections::HashSet;

#[derive(Default)]
pub(super) struct DiscardNames {
    reserved: HashSet<String>,
}

impl DiscardNames {
    pub(super) fn new(module: &ModModule) -> Self {
        let mut names = Self::default();
        for stmt in &module.body {
            names.visit_stmt(stmt);
        }
        names
    }

    pub(super) fn allocate(&mut self) -> String {
        let mut name = "_".to_owned();
        let mut suffix = 0;
        while self.reserved.contains(&name) {
            name = if suffix == 0 {
                "_unused".to_owned()
            } else {
                format!("_unused_{suffix}")
            };
            suffix += 1;
        }
        self.reserved.insert(name.clone());
        name
    }
}

impl<'a> SourceOrderVisitor<'a> for DiscardNames {
    fn visit_identifier(&mut self, identifier: &'a Identifier) {
        self.reserved.insert(identifier.to_string());
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Name(name) = expr {
            self.reserved.insert(name.id.to_string());
        }
        source_order::walk_expr(self, expr);
    }
}
