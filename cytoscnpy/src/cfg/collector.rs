use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast};
use rustc_hash::FxHashSet;

/// Collector for variable definitions and usages in a block
pub(super) struct NameCollector<'a> {
    pub(super) defs: &'a mut FxHashSet<(String, usize)>,
    pub(super) uses: &'a mut FxHashSet<(String, usize)>,
    pub(super) current_line: usize,
}

impl<'a> Visitor<'a> for NameCollector<'a> {
    fn visit_expr(&mut self, expr: &'a ast::Expr) {
        match expr {
            ast::Expr::Name(name) => match name.ctx {
                ast::ExprContext::Load => {
                    self.uses.insert((name.id.to_string(), self.current_line));
                }
                ast::ExprContext::Store | ast::ExprContext::Del => {
                    self.defs.insert((name.id.to_string(), self.current_line));
                }
                ast::ExprContext::Invalid => {}
            },
            _ => visitor::walk_expr(self, expr),
        }
    }

    fn visit_stmt(&mut self, stmt: &'a ast::Stmt) {
        match stmt {
            ast::Stmt::FunctionDef(func) => {
                self.defs.insert((func.name.to_string(), self.current_line));
            }
            ast::Stmt::ClassDef(class) => {
                self.defs
                    .insert((class.name.to_string(), self.current_line));
            }
            ast::Stmt::Assign(assign) => {
                for target in &assign.targets {
                    self.visit_expr(target);
                }
                self.visit_expr(&assign.value);
            }
            ast::Stmt::AnnAssign(assign) => {
                self.visit_expr(&assign.target);
                if let Some(value) = &assign.value {
                    self.visit_expr(value);
                }
            }
            ast::Stmt::AugAssign(assign) => {
                self.visit_expr(&assign.target);
                self.visit_expr(&assign.value);
            }
            ast::Stmt::Expr(expr) => {
                self.visit_expr(&expr.value);
            }
            ast::Stmt::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.visit_expr(value);
                }
            }
            ast::Stmt::Raise(raise) => {
                if let Some(exc) = &raise.exc {
                    self.visit_expr(exc);
                }
                if let Some(cause) = &raise.cause {
                    self.visit_expr(cause);
                }
            }
            ast::Stmt::Assert(assert) => {
                self.visit_expr(&assert.test);
                if let Some(msg) = &assert.msg {
                    self.visit_expr(msg);
                }
            }
            ast::Stmt::Delete(delete) => {
                for target in &delete.targets {
                    self.visit_expr(target);
                }
            }
            _ => {}
        }
    }

    fn visit_pattern(&mut self, pattern: &'a ast::Pattern) {
        match pattern {
            ast::Pattern::MatchAs(p) => {
                if let Some(name) = &p.name {
                    self.defs.insert((name.to_string(), self.current_line));
                }
                if let Some(pattern) = &p.pattern {
                    self.visit_pattern(pattern);
                }
            }
            ast::Pattern::MatchMapping(p) => {
                if let Some(rest) = &p.rest {
                    self.defs.insert((rest.to_string(), self.current_line));
                }
                for key in &p.keys {
                    self.visit_expr(key);
                }
                for pattern in &p.patterns {
                    self.visit_pattern(pattern);
                }
            }
            ast::Pattern::MatchStar(p) => {
                if let Some(name) = &p.name {
                    self.defs.insert((name.to_string(), self.current_line));
                }
            }
            _ => visitor::walk_pattern(self, pattern),
        }
    }
}
