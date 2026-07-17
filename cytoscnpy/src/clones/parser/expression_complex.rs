use super::definitions::parameter_nodes;
use super::expressions::extract_expr_nodes;
use super::types::SubtreeNode;
use ruff_python_ast as ast;

pub(super) fn extract_complex_expr(expr: &ast::Expr) -> Vec<SubtreeNode> {
    match expr {
        ast::Expr::Lambda(node) => {
            let mut children = node
                .parameters
                .as_deref()
                .map_or_else(Vec::new, parameter_nodes);
            children.extend(extract_expr_nodes(&node.body));
            wrap("lambda", children)
        }
        ast::Expr::ListComp(node) => {
            comprehension("list_comp", extract_expr_nodes(&node.elt), &node.generators)
        }
        ast::Expr::SetComp(node) => {
            comprehension("set_comp", extract_expr_nodes(&node.elt), &node.generators)
        }
        ast::Expr::Generator(node) => {
            comprehension("generator", extract_expr_nodes(&node.elt), &node.generators)
        }
        ast::Expr::DictComp(node) => {
            let mut result = extract_expr_nodes(&node.key);
            result.extend(extract_expr_nodes(&node.value));
            comprehension("dict_comp", result, &node.generators)
        }
        ast::Expr::FString(node) => wrap("f_string", f_string_nodes(&node.value)),
        ast::Expr::TString(node) => wrap("t_string", interpolated_elements(node.value.elements())),
        ast::Expr::IpyEscapeCommand(node) => vec![SubtreeNode {
            kind: "ipy_escape_expr".into(),
            label: Some(format!("{:?}:{}", node.kind, node.value)),
            children: vec![],
        }],
        _ => vec![SubtreeNode {
            kind: "unmodeled_expr".into(),
            label: None,
            children: vec![],
        }],
    }
}

fn comprehension(
    kind: &str,
    mut children: Vec<SubtreeNode>,
    generators: &[ast::Comprehension],
) -> Vec<SubtreeNode> {
    for generator in generators {
        let mut generator_children = extract_expr_nodes(&generator.target);
        generator_children.extend(extract_expr_nodes(&generator.iter));
        for condition in &generator.ifs {
            generator_children.push(SubtreeNode {
                kind: "comp_if".into(),
                label: None,
                children: extract_expr_nodes(condition),
            });
        }
        children.push(SubtreeNode {
            kind: if generator.is_async {
                "async_comprehension"
            } else {
                "comprehension"
            }
            .into(),
            label: None,
            children: generator_children,
        });
    }
    wrap(kind, children)
}

fn f_string_nodes(value: &ast::FStringValue) -> Vec<SubtreeNode> {
    let mut nodes = Vec::new();
    for part in value {
        match part {
            ast::FStringPart::Literal(literal) => nodes.push(literal_node(&literal.value)),
            ast::FStringPart::FString(f_string) => {
                nodes.extend(interpolated_elements(f_string.elements.iter()));
            }
        }
    }
    nodes
}

fn interpolated_elements<'a>(
    elements: impl Iterator<Item = &'a ast::InterpolatedStringElement>,
) -> Vec<SubtreeNode> {
    elements
        .map(|element| match element {
            ast::InterpolatedStringElement::Literal(literal) => literal_node(&literal.value),
            ast::InterpolatedStringElement::Interpolation(interpolation) => {
                let mut children = extract_expr_nodes(&interpolation.expression);
                if let Some(spec) = &interpolation.format_spec {
                    children.extend(interpolated_elements(spec.elements.iter()));
                }
                SubtreeNode {
                    kind: "interpolation".into(),
                    label: Some(format!("{:?}", interpolation.conversion)),
                    children,
                }
            }
        })
        .collect()
}

fn literal_node(value: &str) -> SubtreeNode {
    SubtreeNode {
        kind: "interpolated_literal".into(),
        label: Some(value.to_owned()),
        children: vec![],
    }
}

fn wrap(kind: &str, children: Vec<SubtreeNode>) -> Vec<SubtreeNode> {
    vec![SubtreeNode {
        kind: kind.into(),
        label: None,
        children,
    }]
}
