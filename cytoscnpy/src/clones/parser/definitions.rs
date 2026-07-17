use super::expressions::extract_expr_nodes;
use super::types::SubtreeNode;
use ruff_python_ast as ast;

pub(super) fn function_nodes(function: &ast::StmtFunctionDef) -> Vec<SubtreeNode> {
    let mut nodes = decorator_nodes(&function.decorator_list);
    if let Some(type_params) = &function.type_params {
        nodes.extend(type_parameter_nodes(type_params));
    }
    nodes.extend(parameter_nodes(&function.parameters));
    if let Some(returns) = &function.returns {
        nodes.push(SubtreeNode {
            kind: "returns".into(),
            label: semantic_expr_name(returns),
            children: extract_expr_nodes(returns),
        });
    }
    nodes
}

pub(super) fn parameter_nodes(parameters: &ast::Parameters) -> Vec<SubtreeNode> {
    let mut nodes = Vec::new();
    for parameter in &parameters.posonlyargs {
        nodes.push(parameter_node("posonly_param", parameter));
    }
    for parameter in &parameters.args {
        nodes.push(parameter_node("param", parameter));
    }
    if let Some(parameter) = &parameters.vararg {
        nodes.push(bare_parameter_node("vararg", parameter));
    }
    for parameter in &parameters.kwonlyargs {
        nodes.push(parameter_node("kwonly_param", parameter));
    }
    if let Some(parameter) = &parameters.kwarg {
        nodes.push(bare_parameter_node("kwarg", parameter));
    }
    nodes
}

pub(super) fn class_nodes(class: &ast::StmtClassDef) -> Vec<SubtreeNode> {
    let mut nodes = decorator_nodes(&class.decorator_list);
    if let Some(type_params) = &class.type_params {
        nodes.extend(type_parameter_nodes(type_params));
    }
    nodes.extend(class.bases().iter().map(|base| SubtreeNode {
        kind: "base".into(),
        label: semantic_expr_name(base),
        children: extract_expr_nodes(base),
    }));
    nodes.extend(class.keywords().iter().map(|keyword| SubtreeNode {
        kind: "class_keyword".into(),
        label: keyword.arg.as_ref().map(ToString::to_string),
        children: extract_expr_nodes(&keyword.value),
    }));
    nodes
}

fn type_parameter_nodes(type_params: &ast::TypeParams) -> Vec<SubtreeNode> {
    type_params
        .type_params
        .iter()
        .map(|parameter| match parameter {
            ast::TypeParam::TypeVar(type_var) => {
                let mut children = type_var
                    .bound
                    .as_deref()
                    .map_or_else(Vec::new, extract_expr_nodes);
                if let Some(default) = type_var.default.as_deref() {
                    children.extend(extract_expr_nodes(default));
                }
                SubtreeNode {
                    kind: "type_var".into(),
                    label: Some(type_var.name.to_string()),
                    children,
                }
            }
            ast::TypeParam::TypeVarTuple(tuple) => SubtreeNode {
                kind: "type_var_tuple".into(),
                label: Some(tuple.name.to_string()),
                children: tuple
                    .default
                    .as_deref()
                    .map_or_else(Vec::new, extract_expr_nodes),
            },
            ast::TypeParam::ParamSpec(spec) => SubtreeNode {
                kind: "param_spec".into(),
                label: Some(spec.name.to_string()),
                children: spec
                    .default
                    .as_deref()
                    .map_or_else(Vec::new, extract_expr_nodes),
            },
        })
        .collect()
}

fn decorator_nodes(decorators: &[ast::Decorator]) -> Vec<SubtreeNode> {
    decorators
        .iter()
        .map(|decorator| SubtreeNode {
            kind: "decorator".into(),
            label: decorator_name(&decorator.expression),
            children: extract_expr_nodes(&decorator.expression),
        })
        .collect()
}

fn parameter_node(kind: &str, parameter: &ast::ParameterWithDefault) -> SubtreeNode {
    let mut node = bare_parameter_node(kind, &parameter.parameter);
    if let Some(default) = &parameter.default {
        node.children.push(SubtreeNode {
            kind: "default".into(),
            label: None,
            children: extract_expr_nodes(default),
        });
    }
    node
}

fn bare_parameter_node(kind: &str, parameter: &ast::Parameter) -> SubtreeNode {
    let children = parameter
        .annotation
        .as_deref()
        .map_or_else(Vec::new, |annotation| {
            vec![SubtreeNode {
                kind: "annotation".into(),
                label: semantic_expr_name(annotation),
                children: extract_expr_nodes(annotation),
            }]
        });
    SubtreeNode {
        kind: kind.into(),
        label: Some(parameter.name.to_string()),
        children,
    }
}

fn decorator_name(expression: &ast::Expr) -> Option<String> {
    match expression {
        ast::Expr::Call(call) => semantic_expr_name(&call.func),
        _ => semantic_expr_name(expression),
    }
}

fn semantic_expr_name(expression: &ast::Expr) -> Option<String> {
    match expression {
        ast::Expr::Name(name) => Some(name.id.to_string()),
        ast::Expr::Attribute(attribute) => {
            semantic_expr_name(&attribute.value).map(|base| format!("{base}.{}", attribute.attr))
        }
        ast::Expr::Subscript(subscript) => semantic_expr_name(&subscript.value),
        _ => None,
    }
}
