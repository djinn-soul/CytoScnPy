use super::LinterVisitor;
use ruff_python_ast::Expr;

impl LinterVisitor {
    /// Visits an expression node and applies rules.
    ///
    /// This function implements comprehensive recursion for all expression types
    /// (Calls, `BinOps`, Comprehensions, etc.) to ensure that linter rules
    /// can inspect strictly nested nodes.
    /// Verified by `analyzer_test` and `quality_test` suites.
    pub fn visit_expr(&mut self, expr: &Expr) {
        self.apply_visit_expr_rules(expr);
        self.visit_expr_children(expr);
        self.apply_leave_expr_rules(expr);
    }

    fn apply_visit_expr_rules(&mut self, expr: &Expr) {
        for rule in &mut self.rules {
            if let Some(mut findings) = rule.visit_expr(expr, &self.context) {
                self.findings.append(&mut findings);
            }
        }
    }

    fn apply_leave_expr_rules(&mut self, expr: &Expr) {
        for rule in &mut self.rules {
            if let Some(mut findings) = rule.leave_expr(expr, &self.context) {
                self.findings.append(&mut findings);
            }
        }
    }

    fn visit_expr_children(&mut self, expr: &Expr) {
        if self.visit_primary_expr_children(expr) {
            return;
        }
        if self.visit_container_expr_children(expr) {
            return;
        }
        let _ = self.visit_comprehension_expr_children(expr);
    }

    fn visit_primary_expr_children(&mut self, expr: &Expr) -> bool {
        if self.visit_call_expr_children(expr) {
            return true;
        }
        if self.visit_access_expr_children(expr) {
            return true;
        }
        self.visit_operator_expr_children(expr)
    }

    fn visit_call_expr_children(&mut self, expr: &Expr) -> bool {
        let Expr::Call(node) = expr else {
            return false;
        };

        self.visit_expr(&node.func);
        self.visit_exprs(&node.arguments.args);
        for keyword in &node.arguments.keywords {
            self.visit_expr(&keyword.value);
        }
        true
    }

    fn visit_access_expr_children(&mut self, expr: &Expr) -> bool {
        match expr {
            Expr::Attribute(node) => self.visit_expr(&node.value),
            Expr::UnaryOp(node) => self.visit_expr(&node.operand),
            Expr::Starred(node) => self.visit_expr(&node.value),
            _ => return false,
        }
        true
    }

    fn visit_operator_expr_children(&mut self, expr: &Expr) -> bool {
        match expr {
            Expr::BinOp(node) => {
                self.visit_expr(&node.left);
                self.visit_expr(&node.right);
            }
            Expr::BoolOp(node) => {
                self.visit_exprs(&node.values);
            }
            Expr::Compare(node) => {
                self.visit_expr(&node.left);
                self.visit_exprs(&node.comparators);
            }
            _ => return false,
        }
        true
    }

    fn visit_container_expr_children(&mut self, expr: &Expr) -> bool {
        if self.visit_sequence_expr_children(expr) {
            return true;
        }
        if self.visit_mapping_expr_children(expr) {
            return true;
        }
        self.visit_passthrough_expr_children(expr)
    }

    fn visit_sequence_expr_children(&mut self, expr: &Expr) -> bool {
        match expr {
            Expr::List(node) => self.visit_exprs(&node.elts),
            Expr::Tuple(node) => self.visit_exprs(&node.elts),
            Expr::Set(node) => self.visit_exprs(&node.elts),
            _ => return false,
        }
        true
    }

    fn visit_mapping_expr_children(&mut self, expr: &Expr) -> bool {
        match expr {
            Expr::Dict(node) => {
                for item in &node.items {
                    self.visit_optional_expr(item.key.as_ref());
                    self.visit_expr(&item.value);
                }
            }
            Expr::Subscript(node) => {
                self.visit_expr(&node.value);
                self.visit_expr(&node.slice);
            }
            _ => return false,
        }
        true
    }

    fn visit_passthrough_expr_children(&mut self, expr: &Expr) -> bool {
        match expr {
            Expr::Yield(node) => self.visit_optional_expr(node.value.as_deref()),
            Expr::YieldFrom(node) => self.visit_expr(&node.value),
            Expr::Await(node) => self.visit_expr(&node.value),
            Expr::Lambda(node) => self.visit_expr(&node.body),
            _ => return false,
        }
        true
    }

    fn visit_comprehension_expr_children(&mut self, expr: &Expr) -> bool {
        match expr {
            Expr::ListComp(node) => {
                self.visit_comprehension_generators(&node.generators);
                self.visit_expr(&node.elt);
            }
            Expr::SetComp(node) => {
                self.visit_comprehension_generators(&node.generators);
                self.visit_expr(&node.elt);
            }
            _ => return self.visit_mapping_comprehension_expr_children(expr),
        }
        true
    }

    fn visit_mapping_comprehension_expr_children(&mut self, expr: &Expr) -> bool {
        match expr {
            Expr::DictComp(node) => {
                self.visit_comprehension_generators(&node.generators);
                self.visit_expr(&node.key);
                self.visit_expr(&node.value);
            }
            Expr::Generator(node) => {
                self.visit_comprehension_generators(&node.generators);
                self.visit_expr(&node.elt);
            }
            _ => return false,
        }
        true
    }

    fn visit_exprs(&mut self, exprs: &[Expr]) {
        for expr in exprs {
            self.visit_expr(expr);
        }
    }

    fn visit_comprehension_generators(&mut self, generators: &[ruff_python_ast::Comprehension]) {
        for generator in generators {
            self.visit_expr(&generator.iter);
            for test in &generator.ifs {
                self.visit_expr(test);
            }
        }
    }
}
