//! Allocate discard bindings without shadowing names already present in the module.

use ruff_python_ast::visitor::source_order::{self, SourceOrderVisitor};
use ruff_python_ast::{Expr, Identifier, ModModule};
use rustc_hash::FxHashSet;

/// Every name that appears in a module. Collected once per file and shared
/// by every fix planned against it.
#[derive(Default)]
pub(super) struct ModuleNames {
    reserved: FxHashSet<String>,
}

impl ModuleNames {
    pub(super) fn new(module: &ModModule) -> Self {
        let mut names = Self::default();
        for stmt in &module.body {
            names.visit_stmt(stmt);
        }
        names
    }
}

/// Allocates discard bindings for one fix plan.
pub(super) struct DiscardNames<'a> {
    module: &'a ModuleNames,
    allocated: FxHashSet<String>,
}

impl<'a> DiscardNames<'a> {
    pub(super) fn new(module: &'a ModuleNames) -> Self {
        Self {
            module,
            allocated: FxHashSet::default(),
        }
    }

    pub(super) fn allocate(&mut self) -> String {
        let mut name = "_".to_owned();
        let mut suffix = 0;
        while self.module.reserved.contains(&name) || self.allocated.contains(&name) {
            name = if suffix == 0 {
                "_unused".to_owned()
            } else {
                format!("_unused_{suffix}")
            };
            suffix += 1;
        }
        self.allocated.insert(name.clone());
        name
    }
}

impl<'a> SourceOrderVisitor<'a> for ModuleNames {
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
