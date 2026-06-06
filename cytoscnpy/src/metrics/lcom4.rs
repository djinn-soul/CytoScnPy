use ruff_python_ast::{self as ast, Stmt};

mod graph;
mod visitor;

#[derive(Clone, Copy, Debug)]
enum MethodKind {
    Instance,
    Class,
    Static,
}

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
pub fn calculate_lcom4(class_body: &[Stmt]) -> usize {
    let method_data = graph::collect_methods(class_body);
    if method_data.is_empty() {
        return 0;
    }

    let (method_list, adjacency) = graph::build_adjacency(&method_data);
    graph::count_connected_components(&method_list, &adjacency)
}

fn classify_method(decorators: &[ast::Decorator]) -> MethodKind {
    let mut is_static = false;
    let mut is_class = false;
    for decorator in decorators {
        if decorator_matches(&decorator.expression, "staticmethod") {
            is_static = true;
        }
        if decorator_matches(&decorator.expression, "classmethod") {
            is_class = true;
        }
    }
    method_kind_from_flags(is_static, is_class)
}

fn method_kind_from_flags(is_static: bool, is_class: bool) -> MethodKind {
    if is_static {
        MethodKind::Static
    } else if is_class {
        MethodKind::Class
    } else {
        MethodKind::Instance
    }
}

fn decorator_matches(expr: &ast::Expr, expected: &str) -> bool {
    match expr {
        ast::Expr::Name(name) => name.id == expected,
        ast::Expr::Attribute(attr) => attr.attr.id == expected,
        ast::Expr::Call(call) => decorator_matches(&call.func, expected),
        _ => false,
    }
}

fn first_parameter_name(parameters: &ast::Parameters) -> Option<String> {
    if let Some(arg) = parameters.posonlyargs.first() {
        return Some(arg.parameter.name.to_string());
    }
    parameters
        .args
        .first()
        .map(|arg| arg.parameter.name.to_string())
}
