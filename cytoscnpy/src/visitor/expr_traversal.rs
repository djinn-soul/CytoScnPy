use super::{ast, CytoScnPyVisitor, Expr};

impl CytoScnPyVisitor<'_> {
    pub(super) fn visit_expr_children(&mut self, expr: &Expr) {
        match expr {
            Expr::ListComp(_) | Expr::SetComp(_) | Expr::DictComp(_) | Expr::Generator(_) => {
                self.visit_comprehension_expr(expr);
            }
            Expr::FString(node) => self.visit_fstring_expr(node),
            _ => self.visit_non_special_expr(expr),
        }
    }

    fn visit_non_special_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Name(_) | Expr::Call(_) | Expr::Attribute(_) | Expr::StringLiteral(_) => {
                self.visit_primary_expr(expr);
            }
            Expr::Lambda(_) | Expr::If(_) => self.visit_conditional_expr(expr),
            Expr::BoolOp(_) | Expr::BinOp(_) | Expr::UnaryOp(_) | Expr::Compare(_) => {
                self.visit_operation_expr(expr);
            }
            Expr::Dict(node) => self.visit_dict_expr(node),
            Expr::Set(node) => self.visit_expr_list(&node.elts),
            Expr::List(_) | Expr::Tuple(_) | Expr::Slice(_) => self.visit_sequence_expr(expr),
            Expr::Await(_) | Expr::Yield(_) | Expr::YieldFrom(_) => {
                self.visit_flow_value_expr(expr);
            }
            Expr::Named(_) | Expr::Subscript(_) | Expr::Starred(_) => {
                self.visit_target_or_access_expr(expr);
            }
            _ => {}
        }
    }

    fn visit_primary_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Name(node) => self.visit_name_expr(node),
            Expr::Call(node) => self.visit_call_expr(node),
            Expr::Attribute(node) => self.visit_attribute_expr(node),
            Expr::StringLiteral(node) => self.visit_string_literal(node),
            _ => {}
        }
    }

    fn visit_conditional_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Lambda(node) => self.visit_expr(&node.body),
            Expr::If(node) => {
                self.visit_expr(&node.test);
                self.visit_expr(&node.body);
                self.visit_expr(&node.orelse);
            }
            _ => {}
        }
    }

    fn visit_operation_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::BoolOp(node) => self.visit_expr_list(&node.values),
            Expr::BinOp(node) => self.visit_binary_expr(&node.left, &node.right),
            Expr::UnaryOp(node) => self.visit_expr(&node.operand),
            Expr::Compare(node) => self.visit_compare_expr(&node.left, &node.comparators),
            _ => {}
        }
    }

    fn visit_binary_expr(&mut self, left: &Expr, right: &Expr) {
        self.visit_expr(left);
        self.visit_expr(right);
    }

    fn visit_compare_expr(&mut self, left: &Expr, comparators: &[Expr]) {
        self.visit_expr(left);
        self.visit_expr_list(comparators);
    }

    fn visit_dict_expr(&mut self, node: &ast::ExprDict) {
        for item in &node.items {
            if let Some(k) = &item.key {
                self.visit_expr(k);
            }
            self.visit_expr(&item.value);
        }
    }

    fn visit_sequence_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::List(node) => self.visit_expr_list(&node.elts),
            Expr::Tuple(node) => self.visit_expr_list(&node.elts),
            Expr::Slice(node) => self.visit_slice_expr(node),
            _ => {}
        }
    }

    fn visit_comprehension_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::ListComp(node) => {
                self.visit_comprehension_generators(&node.generators, Some(&node.elt), None, None);
            }
            Expr::SetComp(node) => {
                self.visit_comprehension_generators(&node.generators, Some(&node.elt), None, None);
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
                self.visit_comprehension_generators(&node.generators, Some(&node.elt), None, None);
            }
            _ => {}
        }
    }

    fn visit_flow_value_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Await(node) => self.visit_expr(&node.value),
            Expr::Yield(node) => {
                if let Some(value) = &node.value {
                    self.visit_expr(value);
                }
            }
            Expr::YieldFrom(node) => self.visit_expr(&node.value),
            _ => {}
        }
    }

    fn visit_target_or_access_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Named(node) => {
                self.visit_definition_target(&node.target);
                self.visit_expr(&node.value);
            }
            Expr::Subscript(node) => {
                self.visit_expr(&node.value);
                self.visit_expr(&node.slice);
            }
            Expr::Starred(node) => self.visit_expr(&node.value),
            _ => {}
        }
    }

    fn visit_expr_list(&mut self, exprs: &[Expr]) {
        for expr in exprs {
            self.visit_expr(expr);
        }
    }

    fn visit_slice_expr(&mut self, node: &ast::ExprSlice) {
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

    fn visit_fstring_expr(&mut self, node: &ast::ExprFString) {
        for part in &node.value {
            self.visit_fstring_part(part);
        }
    }

    fn visit_fstring_part(&mut self, part: &ast::FStringPart) {
        if let ast::FStringPart::FString(fstring) = part {
            self.visit_fstring_elements(&fstring.elements);
        }
    }

    fn visit_fstring_elements(&mut self, elements: &[ast::InterpolatedStringElement]) {
        for element in elements {
            if let ast::InterpolatedStringElement::Interpolation(interp) = element {
                self.visit_expr(&interp.expression);
            }
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
