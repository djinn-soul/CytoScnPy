use ruff_python_ast::{self as ast, Stmt};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug)]
enum MethodKind {
    Instance,
    Class,
    Static,
}

/// Calculates LCOM4 (Lack of Cohesion of Methods 4).
///
/// LCOM4 measures the number of "connected components" in a class.
/// Nodes are methods. Edges exist if:
/// 1. A method accesses the same instance variable as another method.
/// 2. A method calls another method.
///
/// Score 1 = Cohesive (Good).
/// Score > 1 = The class performs > 1 unrelated responsibilities (God Class).
/// Score 0 = Empty class or no methods.
///
/// # Panics
///
/// Panics if internal data structures are inconsistent (methods in `method_list`
/// but not in `method_usage` or `method_calls` maps, or adjacency list missing entries).
pub fn calculate_lcom4(class_body: &[Stmt]) -> usize {
    let mut methods = HashSet::new();
    let mut method_usage: HashMap<String, HashSet<String>> = HashMap::new();
    let mut method_calls: HashMap<String, HashSet<String>> = HashMap::new();

    // 1. Identify methods and their field usages / internal calls
    for stmt in class_body {
        if let Stmt::FunctionDef(func) = stmt {
            let method_name = func.name.id.as_str();
            // Skip dunder methods (constructor, str, etc usually touch everything)
            if method_name.starts_with("__") && method_name.ends_with("__") {
                continue;
            }

            let method_kind = classify_method(&func.decorator_list);
            if matches!(method_kind, MethodKind::Static) {
                continue;
            }

            let method_name = method_name.to_owned();
            methods.insert(method_name.clone());

            let receiver_name = match method_kind {
                MethodKind::Instance | MethodKind::Class => first_parameter_name(&func.parameters),
                MethodKind::Static => None,
            };
            let mut visitor = LcomVisitor::new(receiver_name);
            for s in &func.body {
                visitor.visit_stmt(s);
            }

            method_usage.insert(method_name.clone(), visitor.used_fields);
            method_calls.insert(method_name, visitor.called_methods);
        }
    }

    if methods.is_empty() {
        return 0;
    }

    // 2. Build Graph (Adjacency List)
    // Node: Method Name
    // Edge: if intersection of fields > 0 OR calls exists
    let method_list: Vec<String> = methods.iter().cloned().collect();
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();

    for m in &method_list {
        adj.insert(m.clone(), Vec::new());
    }

    for i in 0..method_list.len() {
        for j in (i + 1)..method_list.len() {
            let m1 = &method_list[i];
            let m2 = &method_list[j];

            let Some(fields1) = method_usage.get(m1) else {
                continue;
            };
            let Some(fields2) = method_usage.get(m2) else {
                continue;
            };

            // Connected if share a field
            let share_fields = fields1.intersection(fields2).next().is_some();

            // Connected if m1 calls m2 OR m2 calls m1
            let Some(calls1) = method_calls.get(m1) else {
                continue;
            };
            let Some(calls2) = method_calls.get(m2) else {
                continue;
            };
            let calls = calls1.contains(m2) || calls2.contains(m1);

            if share_fields || calls {
                if let Some(neighbors) = adj.get_mut(m1) {
                    neighbors.push(m2.clone());
                }
                if let Some(neighbors) = adj.get_mut(m2) {
                    neighbors.push(m1.clone());
                }
            }
        }
    }

    // 3. Count Connected Components
    let mut visited = HashSet::new();
    let mut components = 0;

    for m in &method_list {
        if !visited.contains(m) {
            components += 1;
            // BFS/DFS
            let mut stack = vec![m.clone()];
            visited.insert(m.clone());
            while let Some(current) = stack.pop() {
                if let Some(neighbors) = adj.get(&current) {
                    for neighbor in neighbors {
                        if !visited.contains(neighbor) {
                            visited.insert(neighbor.clone());
                            stack.push(neighbor.clone());
                        }
                    }
                }
            }
        }
    }

    components
}

fn classify_method(decorators: &[ast::Decorator]) -> MethodKind {
    let mut is_static = false;
    let mut is_class = false;
    for decorator in decorators {
        if decorator_matches(&decorator.expression, "staticmethod") {
            is_static = true;
        }
        if decorator_matches(&decorator.expression, "classmethod") {
            is_class = true;
        }
    }
    if is_static {
        MethodKind::Static
    } else if is_class {
        MethodKind::Class
    } else {
        MethodKind::Instance
    }
}

fn decorator_matches(expr: &ast::Expr, expected: &str) -> bool {
    match expr {
        ast::Expr::Name(name) => name.id == expected,
        ast::Expr::Attribute(attr) => attr.attr.id == expected,
        ast::Expr::Call(call) => decorator_matches(&call.func, expected),
        _ => false,
    }
}

fn first_parameter_name(parameters: &ast::Parameters) -> Option<String> {
    if let Some(arg) = parameters.posonlyargs.first() {
        return Some(arg.parameter.name.to_string());
    }
    parameters
        .args
        .first()
        .map(|arg| arg.parameter.name.to_string())
}

struct LcomVisitor {
    used_fields: HashSet<String>,
    called_methods: HashSet<String>,
    receiver_name: Option<String>,
}

impl LcomVisitor {
    fn new(receiver_name: Option<String>) -> Self {
        Self {
            used_fields: HashSet::new(),
            called_methods: HashSet::new(),
            receiver_name,
        }
    }

    fn is_receiver_name(&self, name: &str) -> bool {
        self.receiver_name.as_deref() == Some(name)
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Assign(n) => {
                self.visit_expr(&n.value);
                for t in &n.targets {
                    self.visit_expr(t);
                }
            }
            Stmt::AugAssign(n) => {
                self.visit_expr(&n.target);
                self.visit_expr(&n.value);
            }
            Stmt::Expr(n) => self.visit_expr(&n.value),
            Stmt::If(n) => {
                self.visit_expr(&n.test);
                for s in &n.body {
                    self.visit_stmt(s);
                }
                for s in &n.elif_else_clauses {
                    if let Some(t) = &s.test {
                        self.visit_expr(t);
                    }
                    for b in &s.body {
                        self.visit_stmt(b);
                    }
                }
            }
            Stmt::Return(n) => {
                if let Some(v) = &n.value {
                    self.visit_expr(v);
                }
            }
            Stmt::For(n) => {
                self.visit_expr(&n.iter);
                for s in &n.body {
                    self.visit_stmt(s);
                }
                for s in &n.orelse {
                    self.visit_stmt(s);
                }
            }
            Stmt::While(n) => {
                self.visit_expr(&n.test);
                for s in &n.body {
                    self.visit_stmt(s);
                }
                for s in &n.orelse {
                    self.visit_stmt(s);
                }
            }
            Stmt::With(n) => {
                for item in &n.items {
                    self.visit_expr(&item.context_expr);
                    if let Some(optional_vars) = &item.optional_vars {
                        self.visit_expr(optional_vars);
                    }
                }
                for s in &n.body {
                    self.visit_stmt(s);
                }
            }
            Stmt::Try(n) => {
                for s in &n.body {
                    self.visit_stmt(s);
                }
                for handler in &n.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = handler;
                    if let Some(type_) = &h.type_ {
                        self.visit_expr(type_);
                    }
                    for s in &h.body {
                        self.visit_stmt(s);
                    }
                }
                for s in &n.orelse {
                    self.visit_stmt(s);
                }
                for s in &n.finalbody {
                    self.visit_stmt(s);
                }
            }
            _ => {
                // Ignore other statements
            }
        }
    }

    fn visit_expr(&mut self, expr: &ast::Expr) {
        match expr {
            ast::Expr::Attribute(attr) => {
                // Check for receiver.field
                if let ast::Expr::Name(name) = &*attr.value {
                    if self.is_receiver_name(name.id.as_str()) {
                        self.used_fields.insert(attr.attr.id.to_string());
                    }
                }
                self.visit_expr(&attr.value);
            }
            ast::Expr::Call(call) => {
                // Check if calling receiver.method()
                if let ast::Expr::Attribute(attr) = &*call.func {
                    if let ast::Expr::Name(name) = &*attr.value {
                        if self.is_receiver_name(name.id.as_str()) {
                            self.called_methods.insert(attr.attr.id.to_string());
                        }
                    }
                }
                self.visit_expr(&call.func);
                for a in &call.arguments.args {
                    self.visit_expr(a);
                }
                for k in &call.arguments.keywords {
                    self.visit_expr(&k.value);
                }
            }
            ast::Expr::BinOp(op) => {
                self.visit_expr(&op.left);
                self.visit_expr(&op.right);
            }
            ast::Expr::UnaryOp(op) => {
                self.visit_expr(&op.operand);
            }
            ast::Expr::BoolOp(op) => {
                for v in &op.values {
                    self.visit_expr(v);
                }
            }
            ast::Expr::Compare(op) => {
                self.visit_expr(&op.left);
                for c in &op.comparators {
                    self.visit_expr(c);
                }
            }
            ast::Expr::If(op) => {
                self.visit_expr(&op.test);
                self.visit_expr(&op.body);
                self.visit_expr(&op.orelse);
            }
            ast::Expr::List(l) => {
                for elt in &l.elts {
                    self.visit_expr(elt);
                }
            }
            ast::Expr::Tuple(t) => {
                for elt in &t.elts {
                    self.visit_expr(elt);
                }
            }
            ast::Expr::Dict(d) => {
                for item in &d.items {
                    if let Some(key) = &item.key {
                        self.visit_expr(key);
                    }
                    self.visit_expr(&item.value);
                }
            }
            ast::Expr::Set(s) => {
                for elt in &s.elts {
                    self.visit_expr(elt);
                }
            }
            ast::Expr::Subscript(s) => {
                self.visit_expr(&s.value);
                self.visit_expr(&s.slice);
            }
            // Other expression types include Name, Constant, etc.
            _ => {}
        }
    }
}
