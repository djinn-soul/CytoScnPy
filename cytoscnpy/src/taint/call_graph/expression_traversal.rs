use super::CallGraph;
use ruff_python_ast::{self as ast, Expr};

pub(super) fn visit_expr_for_calls(
    graph: &mut CallGraph,
    expr: &Expr,
    caller: &str,
    module_name: &str,
) {
    if visit_call_expr(graph, expr, caller, module_name) {
        return;
    }
    if visit_operator_expr(graph, expr, caller, module_name) {
        return;
    }
    if visit_flow_expr(graph, expr, caller, module_name) {
        return;
    }
    if visit_container_expr(graph, expr, caller, module_name) {
        return;
    }
    let _ = visit_string_or_access_expr(graph, expr, caller, module_name);
}

fn visit_call_expr(graph: &mut CallGraph, expr: &Expr, caller: &str, module_name: &str) -> bool {
    let Expr::Call(call) = expr else {
        return false;
    };

    if let Some(callee) = CallGraph::get_call_name(&call.func) {
        graph.add_call_edge(caller, &callee, module_name);
        add_reflection_call_edges(graph, caller, &callee, &call.arguments.args);
    }

    graph.visit_expr_for_calls(&call.func, caller, module_name);
    visit_exprs(graph, &call.arguments.args, caller, module_name);
    for keyword in &call.arguments.keywords {
        graph.visit_expr_for_calls(&keyword.value, caller, module_name);
    }
    true
}

fn add_reflection_call_edges(graph: &mut CallGraph, caller: &str, callee: &str, args: &[Expr]) {
    if !matches!(callee, "hasattr" | "getattr" | "setattr") {
        return;
    }

    if let Some(Expr::StringLiteral(s)) = args.get(1) {
        graph.add_reflection_edge(caller, s.value.to_str());
    }
}

fn visit_operator_expr(
    graph: &mut CallGraph,
    expr: &Expr,
    caller: &str,
    module_name: &str,
) -> bool {
    match expr {
        Expr::BinOp(binop) => {
            graph.visit_expr_for_calls(&binop.left, caller, module_name);
            graph.visit_expr_for_calls(&binop.right, caller, module_name);
        }
        Expr::BoolOp(boolop) => visit_exprs(graph, &boolop.values, caller, module_name),
        Expr::UnaryOp(unary) => graph.visit_expr_for_calls(&unary.operand, caller, module_name),
        Expr::Compare(cmp) => {
            graph.visit_expr_for_calls(&cmp.left, caller, module_name);
            visit_exprs(graph, &cmp.comparators, caller, module_name);
        }
        _ => return false,
    }
    true
}

fn visit_flow_expr(graph: &mut CallGraph, expr: &Expr, caller: &str, module_name: &str) -> bool {
    match expr {
        Expr::If(ifexp) => {
            graph.visit_expr_for_calls(&ifexp.test, caller, module_name);
            graph.visit_expr_for_calls(&ifexp.body, caller, module_name);
            graph.visit_expr_for_calls(&ifexp.orelse, caller, module_name);
        }
        Expr::Named(named) => graph.visit_expr_for_calls(&named.value, caller, module_name),
        Expr::Await(await_expr) => {
            graph.visit_expr_for_calls(&await_expr.value, caller, module_name);
        }
        Expr::Yield(yield_expr) => {
            visit_optional_expr(graph, yield_expr.value.as_deref(), caller, module_name);
        }
        Expr::YieldFrom(yield_from) => {
            graph.visit_expr_for_calls(&yield_from.value, caller, module_name);
        }
        Expr::Lambda(lambda) => graph.visit_expr_for_calls(&lambda.body, caller, module_name),
        _ => return false,
    }
    true
}

fn visit_container_expr(
    graph: &mut CallGraph,
    expr: &Expr,
    caller: &str,
    module_name: &str,
) -> bool {
    if visit_sequence_expr(graph, expr, caller, module_name) {
        return true;
    }
    if visit_mapping_expr(graph, expr, caller, module_name) {
        return true;
    }
    visit_comprehension_expr(graph, expr, caller, module_name)
}

fn visit_sequence_expr(
    graph: &mut CallGraph,
    expr: &Expr,
    caller: &str,
    module_name: &str,
) -> bool {
    match expr {
        Expr::List(list) => visit_exprs(graph, &list.elts, caller, module_name),
        Expr::Tuple(tuple) => visit_exprs(graph, &tuple.elts, caller, module_name),
        Expr::Set(set) => visit_exprs(graph, &set.elts, caller, module_name),
        _ => return false,
    }
    true
}

fn visit_mapping_expr(graph: &mut CallGraph, expr: &Expr, caller: &str, module_name: &str) -> bool {
    match expr {
        Expr::Dict(dict) => {
            for item in &dict.items {
                visit_optional_expr(graph, item.key.as_ref(), caller, module_name);
                graph.visit_expr_for_calls(&item.value, caller, module_name);
            }
        }
        Expr::Subscript(subscript) => {
            graph.visit_expr_for_calls(&subscript.value, caller, module_name);
            graph.visit_expr_for_calls(&subscript.slice, caller, module_name);
        }
        _ => return false,
    }
    true
}

fn visit_comprehension_expr(
    graph: &mut CallGraph,
    expr: &Expr,
    caller: &str,
    module_name: &str,
) -> bool {
    match expr {
        Expr::ListComp(comp) => {
            visit_elt_comprehension(graph, &comp.elt, &comp.generators, caller, module_name);
        }
        Expr::SetComp(comp) => {
            visit_elt_comprehension(graph, &comp.elt, &comp.generators, caller, module_name);
        }
        Expr::Generator(comp) => {
            visit_elt_comprehension(graph, &comp.elt, &comp.generators, caller, module_name);
        }
        Expr::DictComp(comp) => {
            graph.visit_expr_for_calls(&comp.key, caller, module_name);
            graph.visit_expr_for_calls(&comp.value, caller, module_name);
            visit_generators(graph, &comp.generators, caller, module_name);
        }
        _ => return false,
    }
    true
}

fn visit_elt_comprehension(
    graph: &mut CallGraph,
    elt: &Expr,
    generators: &[ast::Comprehension],
    caller: &str,
    module_name: &str,
) {
    graph.visit_expr_for_calls(elt, caller, module_name);
    visit_generators(graph, generators, caller, module_name);
}

fn visit_string_or_access_expr(
    graph: &mut CallGraph,
    expr: &Expr,
    caller: &str,
    module_name: &str,
) -> bool {
    match expr {
        Expr::Starred(starred) => graph.visit_expr_for_calls(&starred.value, caller, module_name),
        Expr::Slice(slice) => visit_slice(graph, slice, caller, module_name),
        Expr::Attribute(attr) => graph.visit_expr_for_calls(&attr.value, caller, module_name),
        Expr::FString(fstring) => visit_fstring(graph, fstring, caller, module_name),
        _ => return false,
    }
    true
}

fn visit_slice(graph: &mut CallGraph, slice: &ast::ExprSlice, caller: &str, module_name: &str) {
    visit_optional_expr(graph, slice.lower.as_deref(), caller, module_name);
    visit_optional_expr(graph, slice.upper.as_deref(), caller, module_name);
    visit_optional_expr(graph, slice.step.as_deref(), caller, module_name);
}

fn visit_fstring(
    graph: &mut CallGraph,
    fstring: &ast::ExprFString,
    caller: &str,
    module_name: &str,
) {
    for part in &fstring.value {
        if let ast::FStringPart::FString(f) = part {
            for element in &f.elements {
                if let ast::InterpolatedStringElement::Interpolation(interp) = element {
                    graph.visit_expr_for_calls(&interp.expression, caller, module_name);
                }
            }
        }
    }
}

fn visit_generators(
    graph: &mut CallGraph,
    generators: &[ast::Comprehension],
    caller: &str,
    module_name: &str,
) {
    for generator in generators {
        graph.visit_expr_for_calls(&generator.iter, caller, module_name);
        visit_exprs(graph, &generator.ifs, caller, module_name);
    }
}

fn visit_exprs(graph: &mut CallGraph, exprs: &[Expr], caller: &str, module_name: &str) {
    for expr in exprs {
        graph.visit_expr_for_calls(expr, caller, module_name);
    }
}

fn visit_optional_expr(
    graph: &mut CallGraph,
    expr: Option<&Expr>,
    caller: &str,
    module_name: &str,
) {
    if let Some(expr) = expr {
        graph.visit_expr_for_calls(expr, caller, module_name);
    }
}
