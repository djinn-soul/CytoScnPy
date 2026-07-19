use crate::framework::visitor::FrameworkAwareVisitor;
use crate::framework::FRAMEWORK_DECORATORS;
use ruff_python_ast::{Decorator, Expr};

pub(super) fn check_decorators(
    visitor: &mut FrameworkAwareVisitor,
    decorators: &[Decorator],
    line: usize,
) {
    for decorator in decorators {
        let name = get_decorator_name(&decorator.expression);
        if visitor.is_framework_file && is_framework_decorator(&name) {
            visitor.framework_decorated_lines.insert(line);
        }
    }
}

fn get_decorator_name(decorator: &Expr) -> String {
    match decorator {
        Expr::Name(node) => node.id.to_string(),
        Expr::Attribute(node) => node.attr.to_string(),
        Expr::Call(node) => get_decorator_name(&node.func),
        _ => String::new(),
    }
}

fn is_framework_decorator(name: &str) -> bool {
    let method = name.rsplit('.').next().unwrap_or(name);
    FRAMEWORK_DECORATORS.iter().any(|pattern| {
        let pattern = pattern.trim_start_matches('@');
        pattern
            .strip_prefix("*.")
            .is_some_and(|expected| method == expected)
            || pattern == name
    })
}
