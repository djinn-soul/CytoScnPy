//! Call-graph edges for types referenced from function annotations.

use super::CallGraph;
use ruff_python_ast as ast;

/// Types named in a function's annotations stay alive while the function does.
/// String annotations (forward references) are parsed.
pub(super) fn visit_annotation_refs(
    graph: &mut CallGraph,
    func: &ast::StmtFunctionDef,
    func_name: &str,
    module_name: &str,
) {
    let params = &func.parameters;
    let annotations = params
        .iter()
        .filter_map(ast::AnyParameterRef::annotation)
        .chain(func.returns.as_deref());
    for annotation in annotations {
        visit_annotation_expr(graph, annotation, func_name, module_name);
    }
}

fn visit_annotation_expr(
    graph: &mut CallGraph,
    annotation: &ast::Expr,
    caller: &str,
    module_name: &str,
) {
    if let ast::Expr::StringLiteral(s) = annotation {
        if let Ok(parsed) = ruff_python_parser::parse_expression(s.value.to_str().trim()) {
            visit_annotation_expr(graph, parsed.expr(), caller, module_name);
        }
        return;
    }
    graph.visit_expr_for_calls(annotation, caller, module_name);
    // Strings nested in subscripts, e.g. `Optional["Foo"]`.
    if let ast::Expr::Subscript(subscript) = annotation {
        let elements: &[ast::Expr] = match &*subscript.slice {
            ast::Expr::Tuple(tuple) => &tuple.elts,
            slice => std::slice::from_ref(slice),
        };
        for element in elements {
            if matches!(
                element,
                ast::Expr::StringLiteral(_) | ast::Expr::Subscript(_)
            ) {
                visit_annotation_expr(graph, element, caller, module_name);
            }
        }
    }
}
