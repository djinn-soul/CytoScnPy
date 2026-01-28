use super::{collect_name_targets, create_finding, LoopDepth, ScopedNames, META_GLOBAL_LOOP};
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{Expr, ExprContext, Stmt};
use ruff_text_size::Ranged;
use std::collections::HashSet;

pub(in crate::rules::quality) struct GlobalUsageInLoopRule {
    loop_depth: LoopDepth,
    module_globals: HashSet<String>,
    local_scopes: ScopedNames,
    scope_depth: usize,
}
impl GlobalUsageInLoopRule {
    pub fn new() -> Self {
        Self {
            loop_depth: LoopDepth::new(),
            module_globals: HashSet::new(),
            local_scopes: ScopedNames::new(),
            scope_depth: 0,
        }
    }
}
impl Rule for GlobalUsageInLoopRule {
    fn name(&self) -> &'static str {
        "GlobalUsageInLoopRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_GLOBAL_LOOP
    }
    fn enter_stmt(&mut self, stmt: &Stmt, _context: &Context) -> Option<Vec<Finding>> {
        match stmt {
            Stmt::FunctionDef(func) => {
                self.record_name(&func.name);
                self.scope_depth += 1;
                self.local_scopes.push_scope();
            }
            Stmt::ClassDef(class_def) => {
                self.record_name(&class_def.name);
                self.scope_depth += 1;
                self.local_scopes.push_scope();
            }
            Stmt::Assign(assign) => {
                self.record_targets(&assign.targets);
            }
            Stmt::AnnAssign(assign) => {
                self.record_targets(std::slice::from_ref(&assign.target));
            }
            Stmt::AugAssign(assign) => {
                self.record_targets(std::slice::from_ref(&assign.target));
            }
            Stmt::For(for_stmt) => {
                self.record_targets(std::slice::from_ref(&for_stmt.target));
            }
            _ => {}
        }

        self.loop_depth.enter_stmt(stmt);
        None
    }
    fn leave_stmt(&mut self, stmt: &Stmt, _context: &Context) -> Option<Vec<Finding>> {
        self.loop_depth.leave_stmt(stmt);
        if matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_)) {
            self.scope_depth = self.scope_depth.saturating_sub(1);
            self.local_scopes.pop_scope();
        }
        None
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if self.loop_depth.in_loop() {
            if let Expr::Name(name) = expr {
                let name_id = name.id.as_str();
                if matches!(name.ctx, ExprContext::Load)
                    && name_id.chars().all(|c| c.is_uppercase() || c == '_')
                    && name_id.len() > 1
                    && self.module_globals.contains(name_id)
                    && !self.local_scopes.contains(name_id)
                {
                    return Some(vec![create_finding(
                        &format!(
                            "Usage of global/constant '{name_id}' in loop (hoist to local for performance)"
                        ),
                        META_GLOBAL_LOOP,
                        context,
                        expr.range().start(),
                        "LOW",
                    )]);
                }
            }
        }
        None
    }
}

impl GlobalUsageInLoopRule {
    fn record_name(&mut self, name: &str) {
        if self.scope_depth == 0 {
            self.module_globals.insert(name.to_owned());
        } else {
            self.local_scopes.insert(name.to_owned());
        }
    }

    fn record_targets(&mut self, targets: &[Expr]) {
        let mut names = Vec::new();
        for target in targets {
            collect_name_targets(target, &mut names);
        }
        for name in names {
            self.record_name(&name);
        }
    }
}
