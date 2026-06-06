//! Function call graph construction.
//!
//! Builds a call graph for interprocedural analysis.

mod expression_traversal;
mod pattern_traversal;
mod statement_traversal;

use ruff_python_ast::{self as ast, Expr, Stmt};
use rustc_hash::FxHashSet;
use std::collections::HashMap;

/// A node in the call graph.
#[derive(Debug, Clone)]
pub struct CallGraphNode {
    /// Function name (qualified)
    pub name: String,
    /// Line where function is defined
    pub line: usize,
    /// Functions called by this function
    pub calls: FxHashSet<String>,
    /// Functions that call this function
    pub called_by: FxHashSet<String>,
    /// Parameter names
    pub params: Vec<String>,
    /// Whether this is a program entry point
    pub is_root: bool,
}

/// Call graph for a module.
#[derive(Debug, Default)]
pub struct CallGraph {
    /// Map from function name to node
    pub nodes: HashMap<String, CallGraphNode>,
    /// Current class context for method qualification
    class_stack: Vec<String>,
}

impl CallGraph {
    /// Creates a new empty call graph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds call graph from module statements.
    pub fn build_from_module(&mut self, stmts: &[Stmt], module_name: &str) {
        let module_node_name = if module_name.is_empty() {
            String::from("<module>")
        } else {
            format!("{module_name}.<module>")
        };

        self.nodes
            .entry(module_node_name.clone())
            .or_insert_with(|| CallGraphNode {
                name: module_node_name.clone(),
                line: 0,
                calls: FxHashSet::default(),
                called_by: FxHashSet::default(),
                params: Vec::new(),
                is_root: true,
            })
            .is_root = true;

        for stmt in stmts {
            self.visit_stmt(stmt, Some(&module_node_name), module_name);
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt, current_func: Option<&str>, module_name: &str) {
        statement_traversal::visit_stmt(self, stmt, current_func, module_name);
    }

    fn visit_expr_for_calls(&mut self, expr: &Expr, caller: &str, module_name: &str) {
        expression_traversal::visit_expr_for_calls(self, expr, caller, module_name);
    }

    fn visit_pattern_for_calls(&mut self, pattern: &ast::Pattern, caller: &str, module_name: &str) {
        pattern_traversal::visit_pattern_for_calls(self, pattern, caller, module_name);
    }

    fn add_call_edge(&mut self, caller: &str, callee: &str, module_name: &str) {
        if let Some(caller_node) = self.nodes.get_mut(caller) {
            caller_node.calls.insert(callee.to_owned());
            Self::add_qualified_local_call(caller_node, callee, module_name);
            Self::add_loose_method_call(caller_node, callee);
        }
        if let Some(callee_node) = self.nodes.get_mut(callee) {
            callee_node.called_by.insert(caller.to_owned());
        }
    }

    fn add_reflection_edge(&mut self, caller: &str, attr_name: &str) {
        if let Some(caller_node) = self.nodes.get_mut(caller) {
            caller_node.calls.insert(format!(".{attr_name}"));
        }
    }

    fn add_qualified_local_call(node: &mut CallGraphNode, callee: &str, module_name: &str) {
        if !callee.contains('.') && !module_name.is_empty() {
            node.calls.insert(format!("{module_name}.{callee}"));
        }
    }

    fn add_loose_method_call(node: &mut CallGraphNode, callee: &str) {
        if let Some(dot_idx) = callee.find('.') {
            if dot_idx > 0 {
                node.calls.insert(format!(".{}", &callee[dot_idx + 1..]));
            }
        }
    }

    fn get_qualified_name(&self, name: &str, module_name: &str) -> String {
        let mut qualified = if module_name.is_empty() {
            String::new()
        } else {
            format!("{module_name}.")
        };

        for class_name in &self.class_stack {
            qualified.push_str(class_name);
            qualified.push('.');
        }

        qualified.push_str(name);
        qualified
    }

    fn extract_params(args: &ast::Parameters) -> Vec<String> {
        let mut params = Vec::new();

        for arg in &args.posonlyargs {
            params.push(arg.parameter.name.to_string());
        }
        for arg in &args.args {
            params.push(arg.parameter.name.to_string());
        }

        if let Some(vararg) = &args.vararg {
            params.push(format!("*{}", vararg.name));
        }

        for arg in &args.kwonlyargs {
            params.push(arg.parameter.name.to_string());
        }

        if let Some(kwarg) = &args.kwarg {
            params.push(format!("**{}", kwarg.name));
        }

        params
    }

    fn get_call_name(func: &Expr) -> Option<String> {
        match func {
            Expr::Name(node) => Some(node.id.to_string()),
            Expr::Attribute(node) => match &*node.value {
                Expr::Name(value) => Some(format!("{}.{}", value.id, node.attr)),
                _ => Some(format!(".{}", node.attr)),
            },
            _ => None,
        }
    }

    /// Merges another call graph into this one.
    pub fn merge(&mut self, other: Self) {
        for (name, node) in other.nodes {
            let entry = self.nodes.entry(name).or_insert_with(|| CallGraphNode {
                name: node.name.clone(),
                line: node.line,
                calls: FxHashSet::default(),
                called_by: FxHashSet::default(),
                params: node.params.clone(),
                is_root: node.is_root,
            });

            entry.calls.extend(node.calls);
            entry.called_by.extend(node.called_by);
            entry.is_root |= node.is_root;
        }
    }

    /// Gets all functions that a given function can reach.
    #[must_use]
    pub fn get_reachable(&self, func_name: &str) -> FxHashSet<String> {
        let mut visited = FxHashSet::default();
        let mut stack = vec![func_name.to_owned()];
        self.collect_reachable(&mut stack, &mut visited);
        visited
    }

    fn collect_reachable(&self, stack: &mut Vec<String>, visited: &mut FxHashSet<String>) {
        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            self.push_unvisited_callees(&current, stack, visited);
        }
    }

    fn push_unvisited_callees(
        &self,
        current: &str,
        stack: &mut Vec<String>,
        visited: &FxHashSet<String>,
    ) {
        let Some(node) = self.nodes.get(current) else {
            return;
        };

        for callee in &node.calls {
            if !visited.contains(callee) {
                stack.push(callee.clone());
            }
        }
    }

    /// Gets topological order for analysis (reverse post-order).
    #[must_use]
    pub fn get_analysis_order(&self) -> Vec<String> {
        let mut visited = FxHashSet::default();
        let mut order = Vec::new();

        for name in self.nodes.keys() {
            self.dfs_post_order(name, &mut visited, &mut order);
        }

        order.reverse();
        order
    }

    fn dfs_post_order(&self, node: &str, visited: &mut FxHashSet<String>, order: &mut Vec<String>) {
        if visited.contains(node) {
            return;
        }
        visited.insert(node.to_owned());

        if let Some(n) = self.nodes.get(node) {
            for callee in &n.calls {
                self.dfs_post_order(callee, visited, order);
            }
        }

        order.push(node.to_owned());
    }
}
