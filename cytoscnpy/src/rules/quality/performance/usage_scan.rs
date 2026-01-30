use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{Expr, ExprContext, Stmt};

pub(super) fn uses_name_in_body(body: &[Stmt], name: &str) -> bool {
    if name == "_" {
        return false;
    }
    let mut finder = NameUseFinder {
        target: name,
        found: false,
    };
    for stmt in body {
        finder.visit_stmt(stmt);
        if finder.found {
            return true;
        }
    }
    false
}

struct NameUseFinder<'a> {
    target: &'a str,
    found: bool,
}

impl<'a> Visitor<'a> for NameUseFinder<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if self.found {
            return;
        }
        match expr {
            Expr::Name(name)
                if name.id.as_str() == self.target && matches!(name.ctx, ExprContext::Load) =>
            {
                self.found = true;
            }
            _ => visitor::walk_expr(self, expr),
        }
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if self.found {
            return;
        }
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {}
            _ => visitor::walk_stmt(self, stmt),
        }
    }
}
