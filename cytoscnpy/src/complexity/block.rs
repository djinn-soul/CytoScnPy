use ruff_python_ast::{self as ast, Expr, Stmt};

pub(super) fn calculate_complexity(body: &[Stmt], no_assert: bool) -> usize {
    let mut visitor = BlockComplexityVisitor {
        complexity: 1,
        no_assert,
    };
    visitor.visit_body(body);
    visitor.complexity
}

struct BlockComplexityVisitor {
    complexity: usize,
    no_assert: bool,
}

fn is_wildcard_case(pattern: &ast::Pattern) -> bool {
    match pattern {
        ast::Pattern::MatchAs(node) => node.pattern.is_none() && node.name.is_none(),
        _ => false,
    }
}

impl BlockComplexityVisitor {
    fn visit_body(&mut self, body: &[Stmt]) {
        for stmt in body {
            self.visit_stmt(stmt);
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::If(node) => {
                self.complexity += 1;
                self.visit_expr(&node.test);
                self.visit_body(&node.body);
                for clause in &node.elif_else_clauses {
                    if let Some(test) = &clause.test {
                        self.complexity += 1;
                        self.visit_expr(test);
                    }
                    self.visit_body(&clause.body);
                }
            }
            Stmt::For(node) => {
                self.complexity += 1;
                self.visit_expr(&node.target);
                self.visit_expr(&node.iter);
                self.visit_body(&node.body);
                if !node.orelse.is_empty() {
                    self.complexity += 1;
                }
                self.visit_body(&node.orelse);
            }
            Stmt::While(node) => {
                self.complexity += 1;
                self.visit_expr(&node.test);
                self.visit_body(&node.body);
                if !node.orelse.is_empty() {
                    self.complexity += 1;
                }
                self.visit_body(&node.orelse);
            }
            Stmt::Try(node) => {
                self.visit_body(&node.body);
                for handler in &node.handlers {
                    self.complexity += 1;
                    let ast::ExceptHandler::ExceptHandler(except_handler) = handler;
                    if let Some(type_) = &except_handler.type_ {
                        self.visit_expr(type_);
                    }
                    self.visit_body(&except_handler.body);
                }
                if !node.orelse.is_empty() {
                    self.complexity += 1;
                }
                self.visit_body(&node.orelse);
                self.visit_body(&node.finalbody);
            }
            Stmt::With(node) => {
                for item in &node.items {
                    self.visit_expr(&item.context_expr);
                    if let Some(optional_vars) = &item.optional_vars {
                        self.visit_expr(optional_vars);
                    }
                }
                self.visit_body(&node.body);
            }
            Stmt::Assert(node) => {
                if !self.no_assert {
                    self.complexity += 1;
                }
                self.visit_expr(&node.test);
                if let Some(msg) = &node.msg {
                    self.visit_expr(msg);
                }
            }
            Stmt::Match(node) => {
                self.visit_expr(&node.subject);
                for case in &node.cases {
                    if !is_wildcard_case(&case.pattern) {
                        self.complexity += 1;
                    }
                    if let Some(guard) = &case.guard {
                        self.visit_expr(guard);
                    }
                    self.visit_body(&case.body);
                }
            }
            Stmt::Expr(node) => self.visit_expr(&node.value),
            Stmt::Return(node) => {
                if let Some(value) = &node.value {
                    self.visit_expr(value);
                }
            }
            Stmt::Assign(node) => {
                for target in &node.targets {
                    self.visit_expr(target);
                }
                self.visit_expr(&node.value);
            }
            Stmt::AugAssign(node) => {
                self.visit_expr(&node.target);
                self.visit_expr(&node.value);
            }
            Stmt::AnnAssign(node) => {
                self.visit_expr(&node.target);
                self.visit_expr(&node.annotation);
                if let Some(value) = &node.value {
                    self.visit_expr(value);
                }
            }
            Stmt::Delete(node) => {
                for target in &node.targets {
                    self.visit_expr(target);
                }
            }
            Stmt::Raise(node) => {
                if let Some(exc) = &node.exc {
                    self.visit_expr(exc);
                }
                if let Some(cause) = &node.cause {
                    self.visit_expr(cause);
                }
            }
            _ => {}
        }
    }

    fn visit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::BoolOp(node) => {
                if node.values.len() > 1 {
                    self.complexity += node.values.len() - 1;
                }
                for value in &node.values {
                    self.visit_expr(value);
                }
            }
            Expr::If(node) => {
                self.complexity += 1;
                self.visit_expr(&node.test);
                self.visit_expr(&node.body);
                self.visit_expr(&node.orelse);
            }
            Expr::ListComp(node) => self.visit_generators(&node.generators, Some(&node.elt), None),
            Expr::SetComp(node) => self.visit_generators(&node.generators, Some(&node.elt), None),
            Expr::DictComp(node) => {
                self.visit_generators(&node.generators, Some(&node.key), Some(&node.value));
            }
            Expr::Generator(node) => self.visit_generators(&node.generators, Some(&node.elt), None),
            Expr::Lambda(node) => self.visit_expr(&node.body),
            Expr::BinOp(node) => {
                self.visit_expr(&node.left);
                self.visit_expr(&node.right);
            }
            Expr::UnaryOp(node) => self.visit_expr(&node.operand),
            Expr::Compare(node) => {
                self.visit_expr(&node.left);
                for cmp in &node.comparators {
                    self.visit_expr(cmp);
                }
            }
            Expr::Attribute(node) => self.visit_expr(&node.value),
            Expr::Subscript(node) => {
                self.visit_expr(&node.value);
                self.visit_expr(&node.slice);
            }
            Expr::Tuple(node) => self.visit_expr_list(&node.elts),
            Expr::List(node) => self.visit_expr_list(&node.elts),
            Expr::Set(node) => self.visit_expr_list(&node.elts),
            Expr::Dict(node) => {
                for item in &node.items {
                    if let Some(key) = &item.key {
                        self.visit_expr(key);
                    }
                    self.visit_expr(&item.value);
                }
            }
            Expr::Named(node) => {
                self.visit_expr(&node.target);
                self.visit_expr(&node.value);
            }
            Expr::Await(node) => self.visit_expr(&node.value),
            Expr::Yield(node) => {
                if let Some(value) = &node.value {
                    self.visit_expr(value);
                }
            }
            Expr::Call(node) => {
                self.visit_expr(&node.func);
                self.visit_expr_list(&node.arguments.args);
                for kw in &node.arguments.keywords {
                    self.visit_expr(&kw.value);
                }
            }
            _ => {}
        }
    }

    fn visit_expr_list(&mut self, exprs: &[Expr]) {
        for expr in exprs {
            self.visit_expr(expr);
        }
    }

    fn visit_generators(
        &mut self,
        generators: &[ast::Comprehension],
        first_expr: Option<&Expr>,
        second_expr: Option<&Expr>,
    ) {
        self.complexity += generators.len();
        for gen in generators {
            self.complexity += gen.ifs.len();
            self.visit_expr(&gen.target);
            self.visit_expr(&gen.iter);
            self.visit_expr_list(&gen.ifs);
        }
        if let Some(expr) = first_expr {
            self.visit_expr(expr);
        }
        if let Some(expr) = second_expr {
            self.visit_expr(expr);
        }
    }
}
