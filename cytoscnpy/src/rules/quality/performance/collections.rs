use super::usage_scan::uses_name_in_body;
use super::{
    create_finding, LoopDepth, META_COMPREHENSION, META_DICT_ITERATOR, META_MEMBERSHIP_LIST,
    META_USELESS_CAST,
};
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

const MIN_MEMBERSHIP_LIST_LEN: usize = 4;

pub(in crate::rules::quality) struct MembershipInListRule {
    loop_depth: LoopDepth,
}
impl MembershipInListRule {
    pub fn new() -> Self {
        Self {
            loop_depth: LoopDepth::new(),
        }
    }
}
impl Rule for MembershipInListRule {
    fn name(&self) -> &'static str {
        "MembershipInListRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_MEMBERSHIP_LIST
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
        let Expr::Compare(comp) = expr else {
            return None;
        };

        let mut findings = Vec::new();

        for (op, right) in comp.ops.iter().zip(comp.comparators.iter()) {
            if matches!(op, ast::CmpOp::In | ast::CmpOp::NotIn) {
                // Check if right side is a list literal
                if let Expr::List(list) = right {
                    if self.loop_depth.in_loop() || list.elts.len() >= MIN_MEMBERSHIP_LIST_LEN {
                        findings.push(create_finding(
                            "Membership test in list literal (use a set {...} for O(1) lookup)",
                            META_MEMBERSHIP_LIST,
                            context,
                            right.range().start(),
                            "MEDIUM",
                        ));
                    }
                }
                // Also check list comprehension [x for x in ...] which constructs full list
                if matches!(right, Expr::ListComp(_)) && self.loop_depth.in_loop() {
                    findings.push(create_finding(
                        "Membership test in list comprehension (use generator expression or set)",
                        META_MEMBERSHIP_LIST,
                        context,
                        right.range().start(),
                        "MEDIUM",
                    ));
                }
            }
        }

        if findings.is_empty() {
            None
        } else {
            Some(findings)
        }
    }
}

pub(in crate::rules::quality) struct UselessCastRule;
impl Rule for UselessCastRule {
    fn name(&self) -> &'static str {
        "UselessCastRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_USELESS_CAST
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Expr::Name(name) = &*call.func {
                if name.id.as_str() == "list" || name.id.as_str() == "tuple" {
                    if let Some(Expr::Call(arg_call)) = call.arguments.args.first() {
                        if let Expr::Name(arg_func) = &*arg_call.func {
                            // list(range(...)), list(map(...)), list(filter(...))
                            if matches!(arg_func.id.as_str(), "range" | "map" | "filter") {
                                return Some(vec![create_finding(
                                    &format!(
                                        "Unnecessary call to {}() on {}() (iterate directly)",
                                        name.id.as_str(),
                                        arg_func.id.as_str()
                                    ),
                                    META_USELESS_CAST,
                                    context,
                                    call.range().start(),
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

pub(in crate::rules::quality) struct IncorrectDictIteratorRule;
impl Rule for IncorrectDictIteratorRule {
    fn name(&self) -> &'static str {
        "IncorrectDictIteratorRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_DICT_ITERATOR
    }
    fn visit_expr(&mut self, _expr: &Expr, _context: &Context) -> Option<Vec<Finding>> {
        // Look for `for _, v in d.items()` or `for k, _ in d.items()`
        // This is usually in a For loop. visit_expr will see the Call to .items()
        // but it doesn't know the target of the For loop.
        // We'll check For loops in enter_stmt.
        None
    }

    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        let Stmt::For(for_stmt) = stmt else {
            return None;
        };

        if let Expr::Call(call) = &*for_stmt.iter {
            if let Expr::Attribute(attr) = &*call.func {
                if attr.attr.as_str() == "items" {
                    // Check target
                    if let Expr::Tuple(tuple) = &*for_stmt.target {
                        if tuple.elts.len() == 2 {
                            let (k_name, v_name) = match (&tuple.elts[0], &tuple.elts[1]) {
                                (Expr::Name(k), Expr::Name(v)) => (k.id.as_str(), v.id.as_str()),
                                _ => return None,
                            };

                            let body_uses_k = uses_name_in_body(&for_stmt.body, k_name)
                                || uses_name_in_body(&for_stmt.orelse, k_name);
                            let body_uses_v = uses_name_in_body(&for_stmt.body, v_name)
                                || uses_name_in_body(&for_stmt.orelse, v_name);

                            let k_is_unused = k_name == "_" || !body_uses_k;
                            let v_is_unused = v_name == "_" || !body_uses_v;

                            if k_is_unused || v_is_unused {
                                let suggestion = if k_is_unused && v_is_unused {
                                    "iterate directly or use .keys()/.values()"
                                } else if k_is_unused {
                                    "use .values()"
                                } else {
                                    "use .keys()"
                                };

                                return Some(vec![create_finding(
                                    &format!(
                                        "Incorrect dictionary iterator: using .items() but discarding one field ({suggestion} instead)"
                                    ),
                                    META_DICT_ITERATOR,
                                    context,
                                    call.range().start(),
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

pub(in crate::rules::quality) struct ComprehensionSuggestionRule;
impl Rule for ComprehensionSuggestionRule {
    fn name(&self) -> &'static str {
        "ComprehensionSuggestionRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_COMPREHENSION
    }
    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        let Stmt::For(for_stmt) = stmt else {
            return None;
        };

        if for_stmt.body.len() == 1 {
            let inner = &for_stmt.body[0];
            if let Some(kind) = comprehension_kind(inner) {
                return Some(vec![create_finding(
                    comprehension_message(kind, false),
                    META_COMPREHENSION,
                    context,
                    stmt.range().start(),
                    "MEDIUM",
                )]);
            }

            if let Stmt::If(if_stmt) = inner {
                if if_stmt.body.len() == 1 && if_stmt.elif_else_clauses.is_empty() {
                    if let Some(kind) = comprehension_kind(&if_stmt.body[0]) {
                        return Some(vec![create_finding(
                            comprehension_message(kind, true),
                            META_COMPREHENSION,
                            context,
                            stmt.range().start(),
                            "MEDIUM",
                        )]);
                    }
                }
            }
        }
        None
    }
}

#[derive(Copy, Clone)]
enum ComprehensionKind {
    List,
    Set,
    Dict,
}

fn comprehension_kind(stmt: &Stmt) -> Option<ComprehensionKind> {
    match stmt {
        Stmt::Expr(expr_stmt) => {
            if let Expr::Call(call) = &*expr_stmt.value {
                if let Expr::Attribute(attr) = &*call.func {
                    return match attr.attr.as_str() {
                        "append" => Some(ComprehensionKind::List),
                        "add" => Some(ComprehensionKind::Set),
                        _ => None,
                    };
                }
            }
            None
        }
        Stmt::Assign(assign)
            if assign.targets.len() == 1 && matches!(&assign.targets[0], Expr::Subscript(_)) =>
        {
            Some(ComprehensionKind::Dict)
        }
        _ => None,
    }
}

fn comprehension_message(kind: ComprehensionKind, conditional: bool) -> &'static str {
    match (kind, conditional) {
        (ComprehensionKind::List, false) => {
            "Simple loop with .append() can be replaced by a list comprehension"
        }
        (ComprehensionKind::List, true) => {
            "Loop with conditional .append() can be replaced by a list comprehension"
        }
        (ComprehensionKind::Set, false) => {
            "Simple loop with .add() can be replaced by a set comprehension"
        }
        (ComprehensionKind::Set, true) => {
            "Loop with conditional .add() can be replaced by a set comprehension"
        }
        (ComprehensionKind::Dict, false) => {
            "Simple loop assigning to a dict can be replaced by a dict comprehension"
        }
        (ComprehensionKind::Dict, true) => {
            "Loop with conditional dict assignment can be replaced by a dict comprehension"
        }
    }
}
