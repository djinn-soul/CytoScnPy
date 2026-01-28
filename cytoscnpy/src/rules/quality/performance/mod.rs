use crate::rules::ids;
use crate::rules::RuleMetadata;
use ruff_python_ast::{Expr, Stmt};
use std::collections::HashSet;

pub(super) use super::finding::create_finding;
use super::CAT_PERFORMANCE;

mod collections;
mod exception_flow;
mod global_usage;
mod io;
mod loop_hoisting;
mod memory;
mod regex;
mod string_concat;
mod tuple_over_list;
mod usage_scan;

pub(super) use collections::{
    ComprehensionSuggestionRule, IncorrectDictIteratorRule, MembershipInListRule, UselessCastRule,
};
pub(super) use exception_flow::ExceptionFlowInLoopRule;
pub(super) use global_usage::GlobalUsageInLoopRule;
pub(super) use io::{FileReadMemoryRiskRule, PandasChunksizeRiskRule};
pub(super) use loop_hoisting::{AttributeChainHoistingRule, PureCallHoistingRule};
pub(super) use memory::MemoryviewOverBytesRule;
pub(super) use regex::RegexLoopRule;
pub(super) use string_concat::StringConcatInLoopRule;
pub(super) use tuple_over_list::UseTupleOverListRule;

pub(super) const META_MEMBERSHIP_LIST: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_MEMBERSHIP_LIST,
    category: CAT_PERFORMANCE,
};

pub(super) const META_FILE_READ_RISK: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_FILE_READ_RISK,
    category: CAT_PERFORMANCE,
};

pub(super) const META_STRING_CONCAT: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_STRING_CONCAT,
    category: CAT_PERFORMANCE,
};

pub(super) const META_USELESS_CAST: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_USELESS_CAST,
    category: CAT_PERFORMANCE,
};

pub(super) const META_REGEX_LOOP: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_REGEX_LOOP,
    category: CAT_PERFORMANCE,
};

pub(super) const META_ATTRIBUTE_HOIST: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_ATTRIBUTE_HOIST,
    category: CAT_PERFORMANCE,
};

pub(super) const META_PURE_CALL_HOIST: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_PURE_CALL_HOIST,
    category: CAT_PERFORMANCE,
};

pub(super) const META_EXCEPTION_FLOW_LOOP: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_EXCEPTION_FLOW_LOOP,
    category: CAT_PERFORMANCE,
};

pub(super) const META_DICT_ITERATOR: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_DICT_ITERATOR,
    category: CAT_PERFORMANCE,
};

pub(super) const META_GLOBAL_LOOP: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_GLOBAL_LOOP,
    category: CAT_PERFORMANCE,
};

pub(super) const META_MEMORYVIEW_BYTES: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_MEMORYVIEW_BYTES,
    category: CAT_PERFORMANCE,
};

pub(super) const META_COMPREHENSION: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_COMPREHENSION,
    category: CAT_PERFORMANCE,
};

pub(super) const META_PANDAS_CHUNK_RISK: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_PANDAS_CHUNK_RISK,
    category: CAT_PERFORMANCE,
};

// Note: META_TUPLE_OVER_LIST is defined but UseTupleOverListRule is not yet fully implemented
// This is intentional for future enhancement.
#[allow(dead_code)]
pub(super) const META_TUPLE_OVER_LIST: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_TUPLE_OVER_LIST,
    category: CAT_PERFORMANCE,
};

pub(super) fn is_loop_stmt(stmt: &Stmt) -> bool {
    matches!(stmt, Stmt::For(_) | Stmt::While(_))
}

pub(super) fn is_scope_boundary(stmt: &Stmt) -> bool {
    matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_))
}

pub(super) fn collect_name_targets(expr: &Expr, names: &mut Vec<String>) {
    match expr {
        Expr::Name(name) => names.push(name.id.to_string()),
        Expr::Tuple(tuple) => {
            for elt in &tuple.elts {
                collect_name_targets(elt, names);
            }
        }
        Expr::List(list) => {
            for elt in &list.elts {
                collect_name_targets(elt, names);
            }
        }
        Expr::Starred(starred) => collect_name_targets(&starred.value, names),
        _ => {}
    }
}

pub(super) struct LoopDepth {
    depth: usize,
    suspended: Vec<usize>,
}

impl LoopDepth {
    pub fn new() -> Self {
        Self {
            depth: 0,
            suspended: Vec::new(),
        }
    }

    pub fn enter_stmt(&mut self, stmt: &Stmt) {
        if is_scope_boundary(stmt) {
            self.suspended.push(self.depth);
            self.depth = 0;
            return;
        }

        if is_loop_stmt(stmt) {
            self.depth += 1;
        }
    }

    pub fn leave_stmt(&mut self, stmt: &Stmt) {
        if is_scope_boundary(stmt) {
            if let Some(prev) = self.suspended.pop() {
                self.depth = prev;
            }
            return;
        }

        if is_loop_stmt(stmt) {
            self.depth = self.depth.saturating_sub(1);
        }
    }

    pub fn in_loop(&self) -> bool {
        self.depth > 0
    }
}

pub(super) struct ScopedNames {
    stack: Vec<HashSet<String>>,
}

impl ScopedNames {
    pub fn new() -> Self {
        Self {
            stack: vec![HashSet::new()],
        }
    }

    pub fn push_scope(&mut self) {
        self.stack.push(HashSet::new());
    }

    pub fn pop_scope(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
        }
    }

    pub fn insert(&mut self, name: String) {
        if let Some(scope) = self.stack.last_mut() {
            scope.insert(name);
        }
    }

    pub fn remove(&mut self, name: &str) {
        if let Some(scope) = self.stack.last_mut() {
            scope.remove(name);
        }
    }

    pub fn contains(&self, name: &str) -> bool {
        self.stack.iter().rev().any(|set| set.contains(name))
    }
}
