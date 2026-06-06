use super::BlockComplexityVisitor;
use ruff_python_ast::{self as ast, Expr};

impl BlockComplexityVisitor {
    pub(super) fn visit_expr(&mut self, expr: &Expr) {
        if self.visit_logic_expr(expr) {
            return;
        }
        if self.visit_comprehension_expr(expr) {
            return;
        }
        if self.visit_operator_expr(expr) {
            return;
        }
        if self.visit_collection_expr(expr) {
            return;
        }
        let _ = self.visit_value_expr(expr);
    }

    fn visit_logic_expr(&mut self, expr: &Expr) -> bool {
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
            _ => return false,
        }
        true
    }

    fn visit_comprehension_expr(&mut self, expr: &Expr) -> bool {
        match expr {
            Expr::ListComp(node) => self.visit_generators(&node.generators, Some(&node.elt), None),
            Expr::SetComp(node) => self.visit_generators(&node.generators, Some(&node.elt), None),
            Expr::DictComp(node) => {
                self.visit_generators(&node.generators, Some(&node.key), Some(&node.value));
            }
            Expr::Generator(node) => self.visit_generators(&node.generators, Some(&node.elt), None),
            _ => return false,
        }
        true
    }

    fn visit_operator_expr(&mut self, expr: &Expr) -> bool {
        match expr {
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
            _ => return false,
        }
        true
    }

    fn visit_collection_expr(&mut self, expr: &Expr) -> bool {
        match expr {
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
            _ => return false,
        }
        true
    }

    fn visit_value_expr(&mut self, expr: &Expr) -> bool {
        match expr {
            Expr::Lambda(node) => self.visit_expr(&node.body),
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
            _ => return false,
        }
        true
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
