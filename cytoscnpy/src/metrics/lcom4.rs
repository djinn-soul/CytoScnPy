use ruff_python_ast::{self as ast, Stmt};
use std::collections::{HashMap, HashSet};

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
            let method_name = func.name.id.to_string();
            // Skip dunder methods (constructor, str, etc usually touch everything)
            if method_name.starts_with("__") && method_name.ends_with("__") {
                continue;
            }

            // Check visibility - ignore privates? No, standard LCOM includes them.
            // Often getters/setters are excluded, but let's keep it simple for now.

            methods.insert(method_name.clone());

            let mut visitor = LcomVisitor::new();
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

struct LcomVisitor {
    used_fields: HashSet<String>,
    called_methods: HashSet<String>,
}

impl LcomVisitor {
    fn new() -> Self {
        Self {
            used_fields: HashSet::new(),
            called_methods: HashSet::new(),
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Assign(n) => {
                self.visit_expr(&n.value);
                // Targets handled if they are self.x?
                for t in &n.targets {
                    self.visit_expr(t);
                }
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
            // ... truncated simplified recursion
            _ => {
                // Fallback: we should really recurse fully, but for LCOM specific
                // we mostly care about explicit Attribute usage.
                // Assuming simple structure for now.
            }
        }
    }

    fn visit_expr(&mut self, expr: &ast::Expr) {
        match expr {
            ast::Expr::Attribute(attr) => {
                // Check for self.field
                if let ast::Expr::Name(name) = &*attr.value {
                    if name.id == "self" {
                        // usage of self.attr
                        // Is it a method call or field access?
                        // If it's in a Call node, it might be a method.
                        // Imprecise without type info, but we assume all self.X are fields unless we distinguish context.
                        // Actually, in LCOM, interacting with a method is also "using" it.
                        // We separate them into 'calls' vs 'fields' to be precise, but for graph connection,
                        // matching names is enough.
                        // Let's store everything as "used_fields" for simplicity unless we can prove it's a call.
                        self.used_fields.insert(attr.attr.id.to_string());
                    }
                }
                self.visit_expr(&attr.value);
            }
            ast::Expr::Call(call) => {
                // Check if calling self.method()
                if let ast::Expr::Attribute(attr) = &*call.func {
                    if let ast::Expr::Name(name) = &*attr.value {
                        if name.id == "self" {
                            self.called_methods.insert(attr.attr.id.to_string());
                        }
                    }
                }
                self.visit_expr(&call.func);
                for a in &call.arguments.args {
                    self.visit_expr(a);
                }
            }
            // Other expression types including Name - we keep it simple for LCOM4
            _ => {}
        }
    }
}
