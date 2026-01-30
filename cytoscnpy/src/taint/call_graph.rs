//! Function call graph construction.
//!
//! Builds a call graph for interprocedural analysis.

use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;
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

        // Ensure module node exists and is root
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

    /// Visits a statement to build the call graph.
    fn visit_stmt(&mut self, stmt: &Stmt, current_func: Option<&str>, module_name: &str) {
        match stmt {
            Stmt::FunctionDef(func) => {
                let func_name = self.get_qualified_name(&func.name, module_name);
                let params = Self::extract_params(&func.parameters);

                let node = CallGraphNode {
                    name: func_name.clone(),
                    line: func.range().start().to_u32() as usize,
                    calls: FxHashSet::default(),
                    called_by: FxHashSet::default(),
                    params,
                    is_root: false,
                };

                self.nodes.insert(func_name.clone(), node);

                // Visit body
                for s in &func.body {
                    self.visit_stmt(s, Some(&func_name), module_name);
                }
            }

            Stmt::ClassDef(class) => {
                self.class_stack.push(class.name.to_string());
                for s in &class.body {
                    self.visit_stmt(s, current_func, module_name);
                }
                self.class_stack.pop();
            }

            Stmt::Expr(expr_stmt) => {
                if let Some(caller) = current_func {
                    self.visit_expr_for_calls(&expr_stmt.value, caller, module_name);
                }
            }

            Stmt::Assign(assign) => {
                if let Some(caller) = current_func {
                    self.visit_expr_for_calls(&assign.value, caller, module_name);
                }
            }

            Stmt::Return(ret) => {
                if let Some(caller) = current_func {
                    if let Some(value) = &ret.value {
                        self.visit_expr_for_calls(value, caller, module_name);
                    }
                }
            }

            Stmt::If(if_stmt) => {
                if let Some(caller) = current_func {
                    self.visit_expr_for_calls(&if_stmt.test, caller, module_name);
                }
                for s in &if_stmt.body {
                    self.visit_stmt(s, current_func, module_name);
                }
                for clause in &if_stmt.elif_else_clauses {
                    for s in &clause.body {
                        self.visit_stmt(s, current_func, module_name);
                    }
                }
            }

            Stmt::For(for_stmt) => {
                if let Some(caller) = current_func {
                    self.visit_expr_for_calls(&for_stmt.iter, caller, module_name);
                }
                for s in &for_stmt.body {
                    self.visit_stmt(s, current_func, module_name);
                }
                for s in &for_stmt.orelse {
                    self.visit_stmt(s, current_func, module_name);
                }
            }

            Stmt::While(while_stmt) => {
                if let Some(caller) = current_func {
                    self.visit_expr_for_calls(&while_stmt.test, caller, module_name);
                }
                for s in &while_stmt.body {
                    self.visit_stmt(s, current_func, module_name);
                }
            }

            Stmt::With(with_stmt) => {
                for s in &with_stmt.body {
                    self.visit_stmt(s, current_func, module_name);
                }
            }

            Stmt::Try(try_stmt) => {
                for s in &try_stmt.body {
                    self.visit_stmt(s, current_func, module_name);
                }
                for handler in &try_stmt.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = handler;
                    for s in &h.body {
                        self.visit_stmt(s, current_func, module_name);
                    }
                }
                for s in &try_stmt.orelse {
                    self.visit_stmt(s, current_func, module_name);
                }
                for s in &try_stmt.finalbody {
                    self.visit_stmt(s, current_func, module_name);
                }
            }

            _ => {}
        }
    }

    /// Visits an expression to find function calls.
    fn visit_expr_for_calls(&mut self, expr: &Expr, caller: &str, module_name: &str) {
        match expr {
            Expr::Call(call) => {
                if let Some(callee) = Self::get_call_name(&call.func) {
                    // Add edge caller -> callee
                    if let Some(caller_node) = self.nodes.get_mut(caller) {
                        caller_node.calls.insert(callee.clone());

                        // If it's a simple name (no dots) and we have a module name,
                        // conservatively add a module-qualified version to handle local calls.
                        if !callee.contains('.') && !module_name.is_empty() {
                            let qualified = format!("{module_name}.{callee}");
                            caller_node.calls.insert(qualified);
                        }

                        // If it's an attribute call (contains '.'), also add a loose version ".attr"
                        // to help with reachability of methods in classes.
                        if let Some(dot_idx) = callee.find('.') {
                            if dot_idx > 0 {
                                // "obj.method" -> ".method"
                                let loose = format!(".{}", &callee[dot_idx + 1..]);
                                caller_node.calls.insert(loose);
                            }
                        }

                        // Special handling for hasattr/getattr/setattr
                        if callee == "hasattr" || callee == "getattr" || callee == "setattr" {
                            if let Some(Expr::StringLiteral(s)) = call.arguments.args.get(1) {
                                let attr_name = s.value.to_str();
                                caller_node.calls.insert(format!(".{attr_name}"));
                            }
                        }
                    }
                    if let Some(callee_node) = self.nodes.get_mut(&callee) {
                        callee_node.called_by.insert(caller.to_owned());
                    }
                }

                // Visit arguments
                for arg in &call.arguments.args {
                    self.visit_expr_for_calls(arg, caller, module_name);
                }
            }

            Expr::BinOp(binop) => {
                self.visit_expr_for_calls(&binop.left, caller, module_name);
                self.visit_expr_for_calls(&binop.right, caller, module_name);
            }

            Expr::If(ifexp) => {
                self.visit_expr_for_calls(&ifexp.test, caller, module_name);
                self.visit_expr_for_calls(&ifexp.body, caller, module_name);
                self.visit_expr_for_calls(&ifexp.orelse, caller, module_name);
            }

            Expr::List(list) => {
                for elt in &list.elts {
                    self.visit_expr_for_calls(elt, caller, module_name);
                }
            }

            Expr::Dict(dict) => {
                for item in &dict.items {
                    self.visit_expr_for_calls(&item.value, caller, module_name);
                }
            }

            _ => {}
        }
    }

    /// Gets qualified name for a function.
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

    /// Extracts parameter names from function arguments.
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

    /// Gets the call name from an expression.
    fn get_call_name(func: &Expr) -> Option<String> {
        match func {
            Expr::Name(node) => Some(node.id.to_string()),
            Expr::Attribute(node) => {
                // If it's a simple attribute call x.y(), return ".y" as a hint
                // if we can't resolve x accurately.
                if let Expr::Name(value) = &*node.value {
                    Some(format!("{}.{}", value.id, node.attr))
                } else {
                    Some(format!(".{}", node.attr))
                }
            }
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

        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            if let Some(node) = self.nodes.get(&current) {
                for callee in &node.calls {
                    if !visited.contains(callee) {
                        stack.push(callee.clone());
                    }
                }
            }
        }

        visited
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
