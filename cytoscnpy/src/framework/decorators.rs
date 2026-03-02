use crate::framework::visitor::FrameworkAwareVisitor;
use ruff_python_ast::{Decorator, Expr};

pub(super) fn check_decorators(
    visitor: &mut FrameworkAwareVisitor,
    decorators: &[Decorator],
    line: usize,
) {
    for decorator in decorators {
        let name = get_decorator_name(&decorator.expression);
        if is_framework_decorator(&name) {
            visitor.framework_decorated_lines.insert(line);
            visitor.is_framework_file = true;
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
    let name = name.to_lowercase();
    name.contains("route")
        || name.contains("get")
        || name.contains("post")
        || name.contains("put")
        || name.contains("delete")
        || name.contains("validator")
        || name.contains("task")
        || name.contains("login_required")
        || name.contains("permission_required")
        || name.contains("trigger")
        || name.contains("function_name")
        || name.ends_with("_input")
        || name.ends_with("_output")
}
