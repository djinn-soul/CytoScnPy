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
        } else if is_relative_blueprint_route(visitor, &name) {
            visitor.is_framework_file = true;
            visitor.detected_frameworks.insert("flask".to_owned());
            visitor.framework_decorated_lines.insert(line);
        }
    }
}

fn get_decorator_name(decorator: &Expr) -> String {
    match decorator {
        Expr::Name(node) => node.id.to_string(),
        Expr::Attribute(node) => {
            let owner = get_decorator_name(&node.value);
            if owner.is_empty() {
                node.attr.to_string()
            } else {
                format!("{owner}.{}", node.attr)
            }
        }
        Expr::Call(node) => get_decorator_name(&node.func),
        _ => String::new(),
    }
}

fn is_relative_blueprint_route(visitor: &FrameworkAwareVisitor, name: &str) -> bool {
    let Some((owner, method)) = name.rsplit_once('.') else {
        return false;
    };
    method == "route" && visitor.relative_imported_symbols.contains(owner)
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
