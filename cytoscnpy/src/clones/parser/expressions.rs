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
        _ => vec![],
    }
}
