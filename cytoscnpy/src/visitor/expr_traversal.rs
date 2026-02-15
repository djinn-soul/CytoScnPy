use super::*;

impl<'a> CytoScnPyVisitor<'a> {
    pub(super) fn visit_expr_children(&mut self, expr: &Expr) {
        match expr {
            Expr::Name(node) => self.visit_name_expr(node),
            Expr::Call(node) => self.visit_call_expr(node),
            Expr::Attribute(node) => self.visit_attribute_expr(node),
            Expr::StringLiteral(node) => self.visit_string_literal(node),
            Expr::BoolOp(node) => {
                for value in &node.values {
                    self.visit_expr(value);
                }
            }
            Expr::BinOp(node) => {
                self.visit_expr(&node.left);
                self.visit_expr(&node.right);
            }
            Expr::UnaryOp(node) => self.visit_expr(&node.operand),
            Expr::Lambda(node) => self.visit_expr(&node.body),
            Expr::If(node) => {
                self.visit_expr(&node.test);
                self.visit_expr(&node.body);
                self.visit_expr(&node.orelse);
            }
            Expr::Dict(node) => {
                for item in &node.items {
                    if let Some(k) = &item.key {
                        self.visit_expr(k);
                    }
                    self.visit_expr(&item.value);
                }
            }
            Expr::Set(node) => {
                for elt in &node.elts {
                    self.visit_expr(elt);
                }
            }
            Expr::ListComp(node) => {
                self.visit_comprehension_generators(&node.generators, Some(&node.elt), None, None)
            }
            Expr::SetComp(node) => {
                self.visit_comprehension_generators(&node.generators, Some(&node.elt), None, None)
            }
            Expr::DictComp(node) => {
                self.visit_comprehension_generators(
                    &node.generators,
                    None,
                    Some(&node.key),
                    Some(&node.value),
                );
            }
            Expr::Generator(node) => {
                self.visit_comprehension_generators(&node.generators, Some(&node.elt), None, None)
            }
            Expr::Await(node) => self.visit_expr(&node.value),
            Expr::Yield(node) => {
                if let Some(value) = &node.value {
                    self.visit_expr(value);
                }
            }
            Expr::YieldFrom(node) => self.visit_expr(&node.value),
            Expr::Compare(node) => {
                self.visit_expr(&node.left);
                for comparator in &node.comparators {
                    self.visit_expr(comparator);
                }
            }
            Expr::Subscript(node) => {
                self.visit_expr(&node.value);
                self.visit_expr(&node.slice);
            }
            Expr::FString(node) => {
                for part in &node.value {
                    match part {
                        ast::FStringPart::Literal(_) => {}
                        ast::FStringPart::FString(f) => {
                            for element in &f.elements {
                                if let ast::InterpolatedStringElement::Interpolation(interp) =
                                    element
                                {
                                    self.visit_expr(&interp.expression);
                                }
                            }
                        }
                    }
                }
            }
            Expr::List(node) => {
                for elt in &node.elts {
                    self.visit_expr(elt);
                }
            }
            Expr::Tuple(node) => {
                for elt in &node.elts {
                    self.visit_expr(elt);
                }
            }
            Expr::Slice(node) => {
                if let Some(lower) = &node.lower {
                    self.visit_expr(lower);
                }
                if let Some(upper) = &node.upper {
                    self.visit_expr(upper);
                }
                if let Some(step) = &node.step {
                    self.visit_expr(step);
                }
            }
            Expr::Starred(node) => self.visit_expr(&node.value),
            _ => {}
        }
    }

    pub(super) fn visit_comprehension_generators(
        &mut self,
        generators: &[ast::Comprehension],
        elt: Option<&Expr>,
        key: Option<&Expr>,
        value: Option<&Expr>,
    ) {
        for gen in generators {
            self.visit_expr(&gen.iter);
            self.visit_definition_target(&gen.target);
            for if_expr in &gen.ifs {
                self.visit_expr(if_expr);
            }
        }

        if let Some(expr) = elt {
            self.visit_expr(expr);
        }
        if let Some(expr) = key {
            self.visit_expr(expr);
        }
        if let Some(expr) = value {
            self.visit_expr(expr);
        }
    }
}
