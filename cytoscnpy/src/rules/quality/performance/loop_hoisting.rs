use super::{
    collect_name_targets, create_finding, is_loop_stmt, is_scope_boundary, LoopDepth,
    META_ATTRIBUTE_HOIST, META_PURE_CALL_HOIST,
};
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{Expr, Stmt};
use ruff_text_size::Ranged;
use std::collections::HashSet;

const ATTRIBUTE_CHAIN_DEPTH_THRESHOLD: usize = 3;

pub(in crate::rules::quality) struct AttributeChainHoistingRule {
    loop_depth: LoopDepth,
}
impl AttributeChainHoistingRule {
    pub fn new() -> Self {
        Self {
            loop_depth: LoopDepth::new(),
        }
    }
}
impl Rule for AttributeChainHoistingRule {
    fn name(&self) -> &'static str {
        "AttributeChainHoistingRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_ATTRIBUTE_HOIST
    }
    fn enter_stmt(&mut self, stmt: &Stmt, _context: &Context) -> Option<Vec<Finding>> {
        self.loop_depth.enter_stmt(stmt);
        None
    }
    fn leave_stmt(&mut self, stmt: &Stmt, _context: &Context) -> Option<Vec<Finding>> {
        self.loop_depth.leave_stmt(stmt);
        None
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if self.loop_depth.in_loop()
            && matches!(expr, Expr::Attribute(_))
            && attribute_depth(expr) >= ATTRIBUTE_CHAIN_DEPTH_THRESHOLD
        {
            return Some(vec![create_finding(
                "Deep attribute access in loop (hoist to a local variable for performance)",
                META_ATTRIBUTE_HOIST,
                context,
                expr.range().start(),
                "LOW",
            )]);
        }
        None
    }
}

pub(in crate::rules::quality) struct PureCallHoistingRule {
    depth: LoopDepth,
    targets: LoopTargets,
    mutations: LoopMutations,
}
impl PureCallHoistingRule {
    pub fn new() -> Self {
        Self {
            depth: LoopDepth::new(),
            targets: LoopTargets::new(),
            mutations: LoopMutations::new(),
        }
    }
}
impl Rule for PureCallHoistingRule {
    fn name(&self) -> &'static str {
        "PureCallHoistingRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_PURE_CALL_HOIST
    }
    fn enter_stmt(&mut self, stmt: &Stmt, _context: &Context) -> Option<Vec<Finding>> {
        if is_scope_boundary(stmt) {
            self.targets.enter_scope();
            self.mutations.enter_scope();
        }

        match stmt {
            Stmt::For(for_stmt) => {
                self.targets.enter_loop(Some(&for_stmt.target));
                self.mutations.enter_loop();
                self.mutations.add_target_expr(&for_stmt.target);
            }
            Stmt::While(_) => {
                self.targets.enter_loop(None);
                self.mutations.enter_loop();
            }
            _ => {}
        }

        self.depth.enter_stmt(stmt);

        if self.depth.in_loop() {
            match stmt {
                Stmt::Assign(assign) => {
                    for target in &assign.targets {
                        self.mutations.add_target_expr(target);
                    }
                }
                Stmt::AnnAssign(assign) => {
                    self.mutations.add_target_expr(&assign.target);
                }
                Stmt::AugAssign(assign) => {
                    self.mutations.add_target_expr(&assign.target);
                }
                _ => {}
            }
        }
        None
    }
    fn leave_stmt(&mut self, stmt: &Stmt, _context: &Context) -> Option<Vec<Finding>> {
        self.depth.leave_stmt(stmt);

        if is_loop_stmt(stmt) {
            self.targets.leave_loop();
            self.mutations.leave_loop();
        }
        if is_scope_boundary(stmt) {
            self.targets.leave_scope();
            self.mutations.leave_scope();
        }
        None
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if self.depth.in_loop() {
            if let Expr::Call(call) = expr {
                if let Expr::Name(name) = &*call.func {
                    let fname = name.id.as_str();
                    // Pure builtins that are often called in loops on invariant data
                    if matches!(
                        fname,
                        "len"
                            | "abs"
                            | "min"
                            | "max"
                            | "sum"
                            | "round"
                            | "bool"
                            | "int"
                            | "float"
                            | "str"
                    ) {
                        // Heuristic: if arguments are all literals or Simple Names, it's likely hoistable
                        // (Full invariance check would require dataflow)
                        let all_simple = call.arguments.args.iter().all(|arg| {
                            matches!(
                                arg,
                                Expr::Name(_) | Expr::StringLiteral(_) | Expr::NumberLiteral(_)
                            )
                        });
                        if all_simple && !call.arguments.args.is_empty() {
                            let uses_loop_target = call.arguments.args.iter().any(|arg| {
                                matches!(arg, Expr::Name(name) if self.targets.contains(name.id.as_str()))
                            });
                            let uses_mutated_name = call.arguments.args.iter().any(|arg| {
                                matches!(arg, Expr::Name(name) if self.mutations.contains(name.id.as_str()))
                            });

                            if !uses_loop_target && !uses_mutated_name {
                                return Some(vec![create_finding(
                                    &format!("Pure builtin call '{fname}()' in loop with simple arguments (hoist if arguments are invariant)"),
                                    META_PURE_CALL_HOIST,
                                    context,
                                    expr.range().start(),
                                    "LOW",
                                )]);
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

struct LoopTargets {
    stack: Vec<HashSet<String>>,
    suspended: Vec<Vec<HashSet<String>>>,
}

impl LoopTargets {
    fn new() -> Self {
        Self {
            stack: Vec::new(),
            suspended: Vec::new(),
        }
    }

    fn enter_scope(&mut self) {
        self.suspended.push(std::mem::take(&mut self.stack));
    }

    fn leave_scope(&mut self) {
        if let Some(prev) = self.suspended.pop() {
            self.stack = prev;
        }
    }

    fn enter_loop(&mut self, target: Option<&Expr>) {
        let mut set = HashSet::new();
        if let Some(target) = target {
            let mut names = Vec::new();
            collect_name_targets(target, &mut names);
            for name in names {
                set.insert(name);
            }
        }
        self.stack.push(set);
    }

    fn leave_loop(&mut self) {
        self.stack.pop();
    }

    fn contains(&self, name: &str) -> bool {
        self.stack.iter().any(|set| set.contains(name))
    }
}

struct LoopMutations {
    stack: Vec<HashSet<String>>,
    suspended: Vec<Vec<HashSet<String>>>,
}

impl LoopMutations {
    fn new() -> Self {
        Self {
            stack: Vec::new(),
            suspended: Vec::new(),
        }
    }

    fn enter_scope(&mut self) {
        self.suspended.push(std::mem::take(&mut self.stack));
    }

    fn leave_scope(&mut self) {
        if let Some(prev) = self.suspended.pop() {
            self.stack = prev;
        }
    }

    fn enter_loop(&mut self) {
        self.stack.push(HashSet::new());
    }

    fn leave_loop(&mut self) {
        self.stack.pop();
    }

    fn add_target_expr(&mut self, target: &Expr) {
        if self.stack.is_empty() {
            return;
        }
        let mut names = Vec::new();
        collect_name_targets(target, &mut names);
        if let Some(current) = self.stack.last_mut() {
            for name in names {
                current.insert(name);
            }
        }
    }

    fn contains(&self, name: &str) -> bool {
        self.stack.iter().any(|set| set.contains(name))
    }
}

fn attribute_depth(expr: &Expr) -> usize {
    let mut depth = 0;
    let mut current = expr;
    while let Expr::Attribute(attr) = current {
        depth += 1;
        current = &attr.value;
    }
    depth
}
