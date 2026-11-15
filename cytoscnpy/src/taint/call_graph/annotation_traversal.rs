//! Call-graph edges for types referenced from function annotations.
//!
//! Tracks type references inside function parameter/return annotations, class
//! attribute annotations (`AnnAssign`), generic base classes, and PEP 695 type
//! parameters so that types used purely for type checking remain reachable.

use super::CallGraph;
use ruff_python_ast as ast;

/// Types named in a function's annotations stay alive while the function does.
/// String annotations (forward references) are parsed and resolved.
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

/// Recursively traverses a type annotation expression to record call graph references.
///
/// This resolves nested types inside container structures, unions, tuples, and forward
/// references (string literals). Any symbol identified within the annotation is registered
/// as a reference edge from `caller` in the call graph.
pub(super) fn visit_annotation_expr(
    graph: &mut CallGraph,
    annotation: &ast::Expr,
    caller: &str,
    module_name: &str,
) {
    match annotation {
        // Handle forward references written as string literals (e.g. `x: "TargetClass"`).
        // The inner string is parsed into an AST expression and traversed recursively.
        ast::Expr::StringLiteral(s) => {
            if let Ok(parsed) = ruff_python_parser::parse_expression(s.value.to_str().trim()) {
                visit_annotation_expr(graph, parsed.expr(), caller, module_name);
            }
        }
        // Handle generic type arguments such as `dict[str, Target]` or `Generic[TargetClass]`.
        // Both the outer constructor (`dict`, `Generic`) and inner slice arguments are visited.
        ast::Expr::Subscript(subscript) => {
            visit_annotation_expr(graph, &subscript.value, caller, module_name);
            let elements: &[ast::Expr] = match &*subscript.slice {
                ast::Expr::Tuple(tuple) => &tuple.elts,
                slice => std::slice::from_ref(slice),
            };
            for element in elements {
                visit_annotation_expr(graph, element, caller, module_name);
            }
        }
        // Handle union types via PEP 604 binary-or expressions (e.g. `TargetClass | None`).
        // Both the left and right operands must be kept alive.
        ast::Expr::BinOp(binop) => {
            visit_annotation_expr(graph, &binop.left, caller, module_name);
            visit_annotation_expr(graph, &binop.right, caller, module_name);
        }
        // Handle tuple elements inside type annotations (e.g. `Callable[[Arg1, Arg2], Ret]`).
        ast::Expr::Tuple(tuple) => {
            for elt in &tuple.elts {
                visit_annotation_expr(graph, elt, caller, module_name);
            }
        }
        // Fall back to general expression traversal (e.g. bare Name `Target` or Attribute `pkg.Target`),
        // which inserts the appropriate reference edge into `graph.nodes[caller].references`.
        _ => {
            graph.visit_expr_for_calls(annotation, caller, module_name);
        }
    }
}

/// Visits PEP 695 type parameter bounds and defaults as type annotations.
///
/// For example, in `def fn[T: BoundClass = DefaultClass]():`, `BoundClass` and `DefaultClass`
/// are registered as dependencies of `caller`.
pub(super) fn visit_type_params(
    graph: &mut CallGraph,
    type_params: Option<&ast::TypeParams>,
    caller: &str,
    module_name: &str,
) {
    let Some(type_params) = type_params else {
        return;
    };
    for param in &**type_params {
        let (bound, default) = match param {
            ast::TypeParam::TypeVar(tv) => (tv.bound.as_deref(), tv.default.as_deref()),
            ast::TypeParam::ParamSpec(ps) => (None, ps.default.as_deref()),
            ast::TypeParam::TypeVarTuple(tvt) => (None, tvt.default.as_deref()),
        };
        for expr in bound.into_iter().chain(default) {
            visit_annotation_expr(graph, expr, caller, module_name);
        }
    }
}
