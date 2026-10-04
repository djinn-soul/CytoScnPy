//! Loaded names identify references without counting the definition or comments.
use crate::utils::LineIndex;
use ruff_python_ast::{
    visitor::{self, Visitor},
    Expr, ExprContext, Stmt,
};

pub(super) fn collect_references(body: &[Stmt], lines: &LineIndex) -> Vec<(String, usize)> {
    let mut visitor = References {
        lines,
        names: Vec::new(),
    };
    visitor.visit_body(body);
    visitor.names
}

struct References<'a> {
    lines: &'a LineIndex,
    names: Vec<(String, usize)>,
}
impl<'a> Visitor<'a> for References<'_> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Name(name) = expr {
            if name.ctx == ExprContext::Load {
                self.names.push((
                    name.id.to_string(),
                    self.lines.line_index(name.range.start()),
                ));
            }
        }
        visitor::walk_expr(self, expr);
    }
}
