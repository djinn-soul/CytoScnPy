use super::expression_complex::extract_complex_expr;
use super::types::SubtreeNode;
use ruff_python_ast as ast;

/// Extract structural nodes from an expression
pub(super) fn extract_expr_nodes(expr: &ast::Expr) -> Vec<SubtreeNode> {
    match expr {
        ast::Expr::Name(n) => vec![SubtreeNode {
            kind: "name".into(),
            label: Some(n.id.to_string()),
            children: vec![],
        }],
        ast::Expr::Call(c) => {
            let mut children = extract_expr_nodes(&c.func);
            for arg in &c.arguments.args {
                children.extend(extract_expr_nodes(arg));
            }
            for keyword in &c.arguments.keywords {
                children.push(SubtreeNode {
                    kind: "keyword".into(),
                    label: Some(
                        keyword
                            .arg
                            .as_ref()
                            .map_or_else(|| "**".to_owned(), ToString::to_string),
                    ),
                    children: extract_expr_nodes(&keyword.value),
                });
            }
            vec![SubtreeNode {
                kind: "call".into(),
                label: None,
                children,
            }]
        }
        ast::Expr::Attribute(a) => {
            let mut children = extract_expr_nodes(&a.value);
            children.push(SubtreeNode {
                kind: "attr".into(),
                label: Some(a.attr.to_string()),
                children: vec![],
            });
            vec![SubtreeNode {
                kind: "attribute".into(),
                label: None,
                children,
            }]
        }
        ast::Expr::BinOp(b) => {
            let mut children = extract_expr_nodes(&b.left);
            children.extend(extract_expr_nodes(&b.right));
            vec![SubtreeNode {
                kind: "bin_op".into(),
                label: Some(format!("{:?}", b.op)),
                children,
            }]
        }
        ast::Expr::BoolOp(node) => vec![SubtreeNode {
            kind: "bool_op".into(),
            label: Some(format!("{:?}", node.op)),
            children: node.values.iter().flat_map(extract_expr_nodes).collect(),
        }],
        ast::Expr::UnaryOp(node) => vec![SubtreeNode {
            kind: "unary_op".into(),
            label: Some(format!("{:?}", node.op)),
            children: extract_expr_nodes(&node.operand),
        }],
        ast::Expr::Compare(node) => {
            let mut children = extract_expr_nodes(&node.left);
            for (op, comparator) in node.ops.iter().zip(&node.comparators) {
                children.push(SubtreeNode {
                    kind: "compare_op".into(),
                    label: Some(format!("{op:?}")),
                    children: extract_expr_nodes(comparator),
                });
            }
            vec![SubtreeNode {
                kind: "compare".into(),
                label: None,
                children,
            }]
        }
        ast::Expr::StringLiteral(s) => vec![SubtreeNode {
            kind: "str".into(),
            label: Some(s.value.to_string()),
            children: vec![],
        }],
        ast::Expr::NumberLiteral(n) => vec![SubtreeNode {
            kind: "num".into(),
            label: Some(format!("{:?}", n.value)),
            children: vec![],
        }],
        ast::Expr::BooleanLiteral(b) => vec![SubtreeNode {
            kind: "bool".into(),
            label: Some(b.value.to_string()),
            children: vec![],
        }],
        ast::Expr::NoneLiteral(_) => vec![SubtreeNode {
            kind: "none".into(),
            label: Some("None".to_owned()),
            children: vec![],
        }],
        ast::Expr::EllipsisLiteral(_) => vec![SubtreeNode {
            kind: "ellipsis".into(),
            label: None,
            children: vec![],
        }],
        ast::Expr::BytesLiteral(_) => vec![SubtreeNode {
            kind: "bytes".into(),
            label: Some("BYTES".to_owned()),
            children: vec![],
        }],
        ast::Expr::List(l) => {
            let children = l.elts.iter().flat_map(extract_expr_nodes).collect();
            vec![SubtreeNode {
                kind: "list".into(),
                label: None,
                children,
            }]
        }
        ast::Expr::Set(s) => vec![SubtreeNode {
            kind: "set".into(),
            label: None,
            children: s.elts.iter().flat_map(extract_expr_nodes).collect(),
        }],
        ast::Expr::Tuple(t) => {
            let children = t.elts.iter().flat_map(extract_expr_nodes).collect();
            vec![SubtreeNode {
                kind: "tuple".into(),
                label: None,
                children,
            }]
        }
        ast::Expr::Dict(d) => {
            let mut children = vec![];
            for item in &d.items {
                if let Some(key) = &item.key {
                    children.extend(extract_expr_nodes(key));
                }
                children.extend(extract_expr_nodes(&item.value));
            }
            vec![SubtreeNode {
                kind: "dict".into(),
                label: None,
                children,
            }]
        }
        ast::Expr::Named(node) => {
            let mut children = extract_expr_nodes(&node.target);
            children.extend(extract_expr_nodes(&node.value));
            vec![SubtreeNode {
                kind: "named".into(),
                label: None,
                children,
            }]
        }
        ast::Expr::If(node) => {
            let mut children = extract_expr_nodes(&node.test);
            children.extend(extract_expr_nodes(&node.body));
            children.extend(extract_expr_nodes(&node.orelse));
            vec![SubtreeNode {
                kind: "if_expr".into(),
                label: None,
                children,
            }]
        }
        ast::Expr::Subscript(node) => {
            let mut children = extract_expr_nodes(&node.value);
            children.extend(extract_expr_nodes(&node.slice));
            vec![SubtreeNode {
                kind: "subscript".into(),
                label: None,
                children,
            }]
        }
        ast::Expr::Await(node) => wrap("await", extract_expr_nodes(&node.value)),
        ast::Expr::Yield(node) => wrap(
            "yield",
            node.value
                .as_deref()
                .map_or_else(Vec::new, extract_expr_nodes),
        ),
        ast::Expr::YieldFrom(node) => wrap("yield_from", extract_expr_nodes(&node.value)),
        ast::Expr::Starred(node) => wrap("starred", extract_expr_nodes(&node.value)),
        ast::Expr::Slice(node) => {
            let mut children = node
                .lower
                .as_deref()
                .map_or_else(Vec::new, extract_expr_nodes);
            if let Some(upper) = node.upper.as_deref() {
                children.extend(extract_expr_nodes(upper));
            }
            if let Some(step) = node.step.as_deref() {
                children.extend(extract_expr_nodes(step));
            }
            wrap("slice", children)
        }
        _ => extract_complex_expr(expr),
    }
}

fn wrap(kind: &str, children: Vec<SubtreeNode>) -> Vec<SubtreeNode> {
    vec![SubtreeNode {
        kind: kind.into(),
        label: None,
        children,
    }]
}
