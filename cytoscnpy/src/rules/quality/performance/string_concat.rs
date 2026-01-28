use super::{
    collect_name_targets, create_finding, is_scope_boundary, LoopDepth, ScopedNames,
    META_STRING_CONCAT,
};
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{Expr, Operator, Stmt};
use ruff_text_size::Ranged;

pub(in crate::rules::quality) struct StringConcatInLoopRule {
    loop_depth: LoopDepth,
    string_vars: ScopedNames,
}
impl StringConcatInLoopRule {
    pub fn new() -> Self {
        Self {
            loop_depth: LoopDepth::new(),
            string_vars: ScopedNames::new(),
        }
    }

    fn update_string_vars(&mut self, target: &Expr, value: &Expr) {
        let mut names = Vec::new();
        collect_name_targets(target, &mut names);
        if names.is_empty() {
            return;
        }

        let is_string = is_stringy_expr(value);
        for name in names {
            if is_string {
                self.string_vars.insert(name);
            } else {
                self.string_vars.remove(&name);
            }
        }
    }

    fn is_string_target(&self, target: &Expr) -> bool {
        match target {
            Expr::Name(name) => self.string_vars.contains(name.id.as_str()),
            _ => false,
        }
    }
}
impl Rule for StringConcatInLoopRule {
    fn name(&self) -> &'static str {
        "StringConcatInLoopRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_STRING_CONCAT
    }

    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        if is_scope_boundary(stmt) {
            self.string_vars.push_scope();
        }

        self.loop_depth.enter_stmt(stmt);

        match stmt {
            Stmt::Assign(assign) => {
                for target in &assign.targets {
                    self.update_string_vars(target, &assign.value);
                }
            }
            Stmt::AnnAssign(assign) => {
                if let Some(value) = &assign.value {
                    self.update_string_vars(&assign.target, value);
                }
            }
            _ => {}
        }

        if !self.loop_depth.in_loop() {
            return None;
        }

        if let Stmt::AugAssign(aug) = stmt {
            if matches!(aug.op, Operator::Add)
                && (is_stringy_expr(&aug.value) || self.is_string_target(&aug.target))
            {
                return Some(vec![create_finding(
                    "Potential accumulated '+' in loop (use join() for strings)",
                    META_STRING_CONCAT,
                    context,
                    aug.range().start(),
                    "LOW",
                )]);
            }
        }

        if let Stmt::Assign(assign) = stmt {
            if assign.targets.len() == 1 {
                if let Expr::Name(target_name) = &assign.targets[0] {
                    if let Expr::BinOp(bin) = &*assign.value {
                        if matches!(bin.op, Operator::Add) {
                            if let Expr::Name(left_name) = &*bin.left {
                                if left_name.id == target_name.id
                                    && (is_stringy_expr(&bin.right)
                                        || self.string_vars.contains(target_name.id.as_str()))
                                {
                                    return Some(vec![create_finding(
                                        "Potential accumulated '+' in loop (use join() for strings)",
                                        META_STRING_CONCAT,
                                        context,
                                        assign.range().start(),
                                        "LOW",
                                    )]);
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    fn leave_stmt(&mut self, stmt: &Stmt, _context: &Context) -> Option<Vec<Finding>> {
        self.loop_depth.leave_stmt(stmt);

        if is_scope_boundary(stmt) {
            self.string_vars.pop_scope();
        }
        None
    }
}

fn is_stringy_expr(expr: &Expr) -> bool {
    match expr {
        Expr::StringLiteral(_) | Expr::FString(_) => true,
        Expr::Call(call) => match &*call.func {
            Expr::Name(name) => matches!(name.id.as_str(), "str" | "repr" | "format"),
            Expr::Attribute(attr) => {
                attr.attr.as_str() == "format"
                    && matches!(&*attr.value, Expr::StringLiteral(_) | Expr::FString(_))
            }
            _ => false,
        },
        Expr::BinOp(bin) => {
            matches!(bin.op, Operator::Add)
                && (is_stringy_expr(&bin.left) || is_stringy_expr(&bin.right))
        }
        _ => false,
    }
}
