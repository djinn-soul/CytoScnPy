use ruff_python_ast::{self as ast, Expr};

use super::HalsteadVisitor;

pub(super) fn visit_expr(visitor: &mut HalsteadVisitor, expr: &Expr) {
    match expr {
        Expr::BoolOp(node) => visit_bool_op(visitor, node),
        Expr::Named(node) => {
            visitor.add_operator(":=");
            visitor.visit_expr(&node.target);
            visitor.visit_expr(&node.value);
        }
        Expr::BinOp(node) => visit_bin_op(visitor, node),
        Expr::UnaryOp(node) => visit_unary_op(visitor, node),
        Expr::Lambda(node) => visit_lambda(visitor, node),
        Expr::If(node) => {
            visitor.add_operator("if");
            visitor.add_operator("else");
            visitor.visit_expr(&node.test);
            visitor.visit_expr(&node.body);
            visitor.visit_expr(&node.orelse);
        }
        Expr::Dict(_) | Expr::Set(_) | Expr::List(_) | Expr::Tuple(_) => {
            visit_structure_expr(visitor, expr);
        }
        Expr::ListComp(_) | Expr::SetComp(_) | Expr::DictComp(_) | Expr::Generator(_) => {
            visit_comprehension_expr(visitor, expr);
        }
        Expr::Await(node) => {
            visitor.add_operator("await");
            visitor.visit_expr(&node.value);
        }
        Expr::Yield(_) | Expr::YieldFrom(_) => visit_yield_expr(visitor, expr),
        Expr::Compare(node) => visit_compare(visitor, node),
        Expr::Call(node) => {
            visitor.add_operator("()");
            visitor.visit_expr(&node.func);
            for arg in &node.arguments.args {
                visitor.visit_expr(arg);
            }
            for keyword in &node.arguments.keywords {
                visitor.visit_expr(&keyword.value);
            }
        }
        Expr::FString(_)
        | Expr::StringLiteral(_)
        | Expr::BytesLiteral(_)
        | Expr::NumberLiteral(_)
        | Expr::BooleanLiteral(_)
        | Expr::NoneLiteral(_)
        | Expr::EllipsisLiteral(_) => visit_literal_expr(visitor, expr),
        Expr::Attribute(node) => visit_attribute(visitor, node),
        Expr::Subscript(node) => visit_subscript(visitor, node),
        Expr::Starred(node) => visit_starred(visitor, node),
        Expr::Name(node) => visitor.add_operand(&node.id),
        Expr::Slice(node) => visit_slice(visitor, node),
        Expr::TString(_) | Expr::IpyEscapeCommand(_) => {}
    }
}

fn visit_bool_op(visitor: &mut HalsteadVisitor, node: &ast::ExprBoolOp) {
    visitor.add_operator(match node.op {
        ast::BoolOp::And => "and",
        ast::BoolOp::Or => "or",
    });
    for value in &node.values {
        visitor.visit_expr(value);
    }
}

fn visit_bin_op(visitor: &mut HalsteadVisitor, node: &ast::ExprBinOp) {
    visitor.add_operator(match node.op {
        ast::Operator::Add => "+",
        ast::Operator::Sub => "-",
        ast::Operator::Mult => "*",
        ast::Operator::MatMult => "@",
        ast::Operator::Div => "/",
        ast::Operator::Mod => "%",
        ast::Operator::Pow => "**",
        ast::Operator::LShift => "<<",
        ast::Operator::RShift => ">>",
        ast::Operator::BitOr => "|",
        ast::Operator::BitXor => "^",
        ast::Operator::BitAnd => "&",
        ast::Operator::FloorDiv => "//",
    });
    visitor.visit_expr(&node.left);
    visitor.visit_expr(&node.right);
}

fn visit_unary_op(visitor: &mut HalsteadVisitor, node: &ast::ExprUnaryOp) {
    visitor.add_operator(match node.op {
        ast::UnaryOp::Invert => "~",
        ast::UnaryOp::Not => "not",
        ast::UnaryOp::UAdd => "+",
        ast::UnaryOp::USub => "-",
    });
    visitor.visit_expr(&node.operand);
}

fn visit_compare(visitor: &mut HalsteadVisitor, node: &ast::ExprCompare) {
    for op in &node.ops {
        visitor.add_operator(match op {
            ast::CmpOp::Eq => "==",
            ast::CmpOp::NotEq => "!=",
            ast::CmpOp::Lt => "<",
            ast::CmpOp::LtE => "<=",
            ast::CmpOp::Gt => ">",
            ast::CmpOp::GtE => ">=",
            ast::CmpOp::Is => "is",
            ast::CmpOp::IsNot => "is not",
            ast::CmpOp::In => "in",
            ast::CmpOp::NotIn => "not in",
        });
    }
    visitor.visit_expr(&node.left);
    for comparator in &node.comparators {
        visitor.visit_expr(comparator);
    }
}

fn visit_generators(visitor: &mut HalsteadVisitor, generators: &[ast::Comprehension]) {
    for gen in generators {
        visitor.add_operator("for");
        visitor.add_operator("in");
        visitor.visit_expr(&gen.target);
        visitor.visit_expr(&gen.iter);
        for if_ in &gen.ifs {
            visitor.add_operator("if");
            visitor.visit_expr(if_);
        }
    }
}

fn visit_literal_expr(visitor: &mut HalsteadVisitor, expr: &Expr) {
    match expr {
        Expr::StringLiteral(node) => visitor.add_operand(&node.value.to_string()),
        Expr::BytesLiteral(node) => visitor.add_operand(&format!("{:?}", node.value)),
        Expr::NumberLiteral(node) => visitor.add_operand(&format!("{:?}", node.value)),
        Expr::BooleanLiteral(node) => visitor.add_operand(&node.value.to_string()),
        Expr::NoneLiteral(_) => visitor.add_operand("None"),
        Expr::EllipsisLiteral(_) => visitor.add_operand("..."),
        Expr::FString(node) => {
            for part in &node.value {
                if let ast::FStringPart::Literal(s) = part {
                    visitor.add_operand(s);
                }
            }
        }
        _ => {}
    }
}

fn visit_structure_expr(visitor: &mut HalsteadVisitor, expr: &Expr) {
    match expr {
        Expr::Dict(node) => {
            visitor.add_operator("{}");
            for item in &node.items {
                if let Some(key) = &item.key {
                    visitor.visit_expr(key);
                }
                visitor.visit_expr(&item.value);
            }
        }
        Expr::Set(node) => {
            visitor.add_operator("{}");
            for elt in &node.elts {
                visitor.visit_expr(elt);
            }
        }
        Expr::List(node) => {
            visitor.add_operator("[]");
            for elt in &node.elts {
                visitor.visit_expr(elt);
            }
        }
        Expr::Tuple(node) => {
            visitor.add_operator("()");
            for elt in &node.elts {
                visitor.visit_expr(elt);
            }
        }
        _ => {}
    }
}

fn visit_lambda(visitor: &mut HalsteadVisitor, node: &ast::ExprLambda) {
    visitor.add_operator("lambda");
    if let Some(parameters) = &node.parameters {
        for arg in &parameters.args {
            visitor.add_operand(arg.parameter.name.as_str());
        }
    }
    visitor.visit_expr(&node.body);
}

fn visit_comprehension_expr(visitor: &mut HalsteadVisitor, expr: &Expr) {
    match expr {
        Expr::ListComp(node) => {
            visitor.add_operator("[]");
            visitor.visit_expr(&node.elt);
            visit_generators(visitor, &node.generators);
        }
        Expr::SetComp(node) => {
            visitor.add_operator("{}");
            visitor.visit_expr(&node.elt);
            visit_generators(visitor, &node.generators);
        }
        Expr::DictComp(node) => {
            visitor.add_operator("{}");
            visitor.visit_expr(&node.key);
            visitor.visit_expr(&node.value);
            visit_generators(visitor, &node.generators);
        }
        Expr::Generator(node) => {
            visitor.add_operator("()");
            visitor.visit_expr(&node.elt);
            visit_generators(visitor, &node.generators);
        }
        _ => {}
    }
}

fn visit_yield_expr(visitor: &mut HalsteadVisitor, expr: &Expr) {
    match expr {
        Expr::Yield(node) => {
            visitor.add_operator("yield");
            if let Some(value) = &node.value {
                visitor.visit_expr(value);
            }
        }
        Expr::YieldFrom(node) => {
            visitor.add_operator("yield from");
            visitor.visit_expr(&node.value);
        }
        _ => {}
    }
}

fn visit_attribute(visitor: &mut HalsteadVisitor, node: &ast::ExprAttribute) {
    visitor.add_operator(".");
    visitor.visit_expr(&node.value);
    visitor.add_operand(&node.attr);
}

fn visit_subscript(visitor: &mut HalsteadVisitor, node: &ast::ExprSubscript) {
    visitor.add_operator("[]");
    visitor.visit_expr(&node.value);
    visitor.visit_expr(&node.slice);
}

fn visit_starred(visitor: &mut HalsteadVisitor, node: &ast::ExprStarred) {
    visitor.add_operator("*");
    visitor.visit_expr(&node.value);
}

fn visit_slice(visitor: &mut HalsteadVisitor, node: &ast::ExprSlice) {
    visitor.add_operator(":");
    if let Some(lower) = &node.lower {
        visitor.visit_expr(lower);
    }
    if let Some(upper) = &node.upper {
        visitor.visit_expr(upper);
    }
    if let Some(step) = &node.step {
        visitor.visit_expr(step);
    }
}
