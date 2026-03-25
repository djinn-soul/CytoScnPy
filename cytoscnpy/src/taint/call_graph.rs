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

            Stmt::AugAssign(aug_assign) => {
                if let Some(caller) = current_func {
                    self.visit_expr_for_calls(&aug_assign.value, caller, module_name);
                }
            }

            Stmt::AnnAssign(ann_assign) => {
                if let Some(caller) = current_func {
                    if let Some(value) = &ann_assign.value {
                        self.visit_expr_for_calls(value, caller, module_name);
                    }
                }
            }

            Stmt::Assert(assert_stmt) => {
                if let Some(caller) = current_func {
                    self.visit_expr_for_calls(&assert_stmt.test, caller, module_name);
                    if let Some(msg) = &assert_stmt.msg {
                        self.visit_expr_for_calls(msg, caller, module_name);
                    }
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
                if let Some(caller) = current_func {
                    for item in &with_stmt.items {
                        self.visit_expr_for_calls(&item.context_expr, caller, module_name);
                    }
                }
                for s in &with_stmt.body {
                    self.visit_stmt(s, current_func, module_name);
                }
            }

            Stmt::Match(match_stmt) => {
                if let Some(caller) = current_func {
                    self.visit_expr_for_calls(&match_stmt.subject, caller, module_name);
                }
                for case in &match_stmt.cases {
                    if let Some(caller) = current_func {
                        // Guard expression: `case x if some_check(x):`
                        if let Some(guard) = &case.guard {
                            self.visit_expr_for_calls(guard, caller, module_name);
                        }
                        // Patterns can contain calls via MatchValue and MatchClass
                        self.visit_pattern_for_calls(&case.pattern, caller, module_name);
                    }
                    for s in &case.body {
                        self.visit_stmt(s, current_func, module_name);
                    }
                }
            }

            Stmt::Raise(raise_stmt) => {
                if let Some(caller) = current_func {
                    if let Some(exc) = &raise_stmt.exc {
                        self.visit_expr_for_calls(exc, caller, module_name);
                    }
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

                // Visit arguments (positional and keyword)
                for arg in &call.arguments.args {
                    self.visit_expr_for_calls(arg, caller, module_name);
                }
                for keyword in &call.arguments.keywords {
                    self.visit_expr_for_calls(&keyword.value, caller, module_name);
                }
            }

            Expr::BinOp(binop) => {
                self.visit_expr_for_calls(&binop.left, caller, module_name);
                self.visit_expr_for_calls(&binop.right, caller, module_name);
            }

            Expr::BoolOp(boolop) => {
                for value in &boolop.values {
                    self.visit_expr_for_calls(value, caller, module_name);
                }
            }

            Expr::UnaryOp(unary) => {
                self.visit_expr_for_calls(&unary.operand, caller, module_name);
            }

            Expr::If(ifexp) => {
                self.visit_expr_for_calls(&ifexp.test, caller, module_name);
                self.visit_expr_for_calls(&ifexp.body, caller, module_name);
                self.visit_expr_for_calls(&ifexp.orelse, caller, module_name);
            }

            Expr::Compare(cmp) => {
                self.visit_expr_for_calls(&cmp.left, caller, module_name);
                for comparator in &cmp.comparators {
                    self.visit_expr_for_calls(comparator, caller, module_name);
                }
            }

            Expr::Named(named) => {
                self.visit_expr_for_calls(&named.value, caller, module_name);
            }

            Expr::Await(await_expr) => {
                self.visit_expr_for_calls(&await_expr.value, caller, module_name);
            }

            Expr::Yield(yield_expr) => {
                if let Some(value) = &yield_expr.value {
                    self.visit_expr_for_calls(value, caller, module_name);
                }
            }

            Expr::YieldFrom(yield_from) => {
                self.visit_expr_for_calls(&yield_from.value, caller, module_name);
            }

            // Lambda body calls are attributed to the enclosing function (safe approximation).
            Expr::Lambda(lambda) => {
                self.visit_expr_for_calls(&lambda.body, caller, module_name);
            }

            Expr::List(list) => {
                for elt in &list.elts {
                    self.visit_expr_for_calls(elt, caller, module_name);
                }
            }

            Expr::Tuple(tuple) => {
                for elt in &tuple.elts {
                    self.visit_expr_for_calls(elt, caller, module_name);
                }
            }

            Expr::Set(set) => {
                for elt in &set.elts {
                    self.visit_expr_for_calls(elt, caller, module_name);
                }
            }

            Expr::Dict(dict) => {
                for item in &dict.items {
                    if let Some(key) = &item.key {
                        self.visit_expr_for_calls(key, caller, module_name);
                    }
                    self.visit_expr_for_calls(&item.value, caller, module_name);
                }
            }

            Expr::ListComp(comp) => {
                self.visit_expr_for_calls(&comp.elt, caller, module_name);
                for gen in &comp.generators {
                    self.visit_expr_for_calls(&gen.iter, caller, module_name);
                    for cond in &gen.ifs {
                        self.visit_expr_for_calls(cond, caller, module_name);
                    }
                }
            }

            Expr::SetComp(comp) => {
                self.visit_expr_for_calls(&comp.elt, caller, module_name);
                for gen in &comp.generators {
                    self.visit_expr_for_calls(&gen.iter, caller, module_name);
                    for cond in &gen.ifs {
                        self.visit_expr_for_calls(cond, caller, module_name);
                    }
                }
            }

            Expr::DictComp(comp) => {
                self.visit_expr_for_calls(&comp.key, caller, module_name);
                self.visit_expr_for_calls(&comp.value, caller, module_name);
                for gen in &comp.generators {
                    self.visit_expr_for_calls(&gen.iter, caller, module_name);
                    for cond in &gen.ifs {
                        self.visit_expr_for_calls(cond, caller, module_name);
                    }
                }
            }

            Expr::Generator(comp) => {
                self.visit_expr_for_calls(&comp.elt, caller, module_name);
                for gen in &comp.generators {
                    self.visit_expr_for_calls(&gen.iter, caller, module_name);
                    for cond in &gen.ifs {
                        self.visit_expr_for_calls(cond, caller, module_name);
                    }
                }
            }

            Expr::Subscript(subscript) => {
                self.visit_expr_for_calls(&subscript.value, caller, module_name);
                self.visit_expr_for_calls(&subscript.slice, caller, module_name);
            }

            Expr::Starred(starred) => {
                self.visit_expr_for_calls(&starred.value, caller, module_name);
            }

            Expr::Slice(slice) => {
                if let Some(lower) = &slice.lower {
                    self.visit_expr_for_calls(lower, caller, module_name);
                }
                if let Some(upper) = &slice.upper {
                    self.visit_expr_for_calls(upper, caller, module_name);
                }
                if let Some(step) = &slice.step {
                    self.visit_expr_for_calls(step, caller, module_name);
                }
            }

            Expr::FString(fstring) => {
                for part in &fstring.value {
                    if let ast::FStringPart::FString(f) = part {
                        for element in &f.elements {
                            if let ast::InterpolatedStringElement::Interpolation(interp) = element {
                                self.visit_expr_for_calls(&interp.expression, caller, module_name);
                            }
                        }
                    }
                }
            }

            _ => {}
        }
    }

    /// Visits a match pattern for function calls.
    ///
    /// Patterns that can contain calls:
    /// - `MatchValue` — the value expression (e.g. `case Status.ACTIVE:`)
    /// - `MatchClass` — the class expression and keyword patterns (e.g. `case Point(x=get_x()):`)
    /// - `MatchMapping` — key expressions (values are patterns, not expressions)
    /// - `MatchSequence` / `MatchOr` / `MatchAs` — recursed into
    fn visit_pattern_for_calls(&mut self, pattern: &ast::Pattern, caller: &str, module_name: &str) {
        match pattern {
            ast::Pattern::MatchValue(node) => {
                self.visit_expr_for_calls(&node.value, caller, module_name);
            }
            ast::Pattern::MatchClass(node) => {
                // The class expression itself (e.g. `MyModule.Point`)
                self.visit_expr_for_calls(&node.cls, caller, module_name);
                // Positional sub-patterns
                for p in &node.arguments.patterns {
                    self.visit_pattern_for_calls(p, caller, module_name);
                }
                // Keyword sub-patterns (e.g. `case Foo(x=expr_pattern)`)
                for kw in &node.arguments.keywords {
                    self.visit_pattern_for_calls(&kw.pattern, caller, module_name);
                }
            }
            ast::Pattern::MatchMapping(node) => {
                // Keys are expressions and can contain calls
                for key in &node.keys {
                    self.visit_expr_for_calls(key, caller, module_name);
                }
                for p in &node.patterns {
                    self.visit_pattern_for_calls(p, caller, module_name);
                }
            }
            ast::Pattern::MatchSequence(node) => {
                for p in &node.patterns {
                    self.visit_pattern_for_calls(p, caller, module_name);
                }
            }
            ast::Pattern::MatchOr(node) => {
                for p in &node.patterns {
                    self.visit_pattern_for_calls(p, caller, module_name);
                }
            }
            ast::Pattern::MatchAs(node) => {
                if let Some(p) = &node.pattern {
                    self.visit_pattern_for_calls(p, caller, module_name);
                }
            }
            // MatchSingleton (None/True/False) and MatchStar contain no calls
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
