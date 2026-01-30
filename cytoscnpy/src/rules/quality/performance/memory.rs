use super::{
    collect_name_targets, create_finding, is_scope_boundary, LoopDepth, ScopedNames,
    META_MEMORYVIEW_BYTES,
};
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

pub(in crate::rules::quality) struct MemoryviewOverBytesRule {
    loop_depth: LoopDepth,
    bytes_vars: ScopedNames,
}
impl MemoryviewOverBytesRule {
    pub fn new() -> Self {
        Self {
            loop_depth: LoopDepth::new(),
            bytes_vars: ScopedNames::new(),
        }
    }

    fn update_bytes_vars(&mut self, target: &Expr, value: &Expr) {
        let mut names = Vec::new();
        collect_name_targets(target, &mut names);
        if names.is_empty() {
            return;
        }

        let is_bytes = is_bytes_expr(value);
        for name in names {
            if is_bytes {
                self.bytes_vars.insert(name);
            } else {
                self.bytes_vars.remove(&name);
            }
        }
    }
}
impl Rule for MemoryviewOverBytesRule {
    fn name(&self) -> &'static str {
        "MemoryviewOverBytesRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_MEMORYVIEW_BYTES
    }
    fn enter_stmt(&mut self, stmt: &Stmt, _context: &Context) -> Option<Vec<Finding>> {
        if is_scope_boundary(stmt) {
            self.bytes_vars.push_scope();
        }

        self.loop_depth.enter_stmt(stmt);

        match stmt {
            Stmt::Assign(assign) => {
                for target in &assign.targets {
                    self.update_bytes_vars(target, &assign.value);
                }
            }
            Stmt::AnnAssign(assign) => {
                if let Some(value) = &assign.value {
                    self.update_bytes_vars(&assign.target, value);
                }
            }
            _ => {}
        }
        None
    }
    fn leave_stmt(&mut self, stmt: &Stmt, _context: &Context) -> Option<Vec<Finding>> {
        self.loop_depth.leave_stmt(stmt);

        if is_scope_boundary(stmt) {
            self.bytes_vars.pop_scope();
        }
        None
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if self.loop_depth.in_loop() {
            if let Expr::Subscript(sub) = expr {
                if let ast::Expr::Slice(_) = &*sub.slice {
                    if is_bytes_value(&sub.value, &self.bytes_vars) {
                        return Some(vec![create_finding(
                            "Looped slicing of bytes-like data (use memoryview() for zero-copy)",
                            META_MEMORYVIEW_BYTES,
                            context,
                            expr.range().start(),
                            "LOW",
                        )]);
                    }
                }
            }
        }
        None
    }
}

fn is_bytes_expr(expr: &Expr) -> bool {
    match expr {
        Expr::BytesLiteral(_) => true,
        Expr::Call(call) => match &*call.func {
            Expr::Name(name) => matches!(name.id.as_str(), "bytes" | "bytearray" | "memoryview"),
            _ => false,
        },
        _ => false,
    }
}

fn is_bytes_value(expr: &Expr, bytes_vars: &ScopedNames) -> bool {
    match expr {
        Expr::BytesLiteral(_) => true,
        Expr::Name(name) => bytes_vars.contains(name.id.as_str()),
        Expr::Call(call) => match &*call.func {
            Expr::Name(name) => matches!(name.id.as_str(), "bytes" | "bytearray" | "memoryview"),
            _ => false,
        },
        _ => false,
    }
}
