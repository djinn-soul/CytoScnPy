use super::expressions::extract_expr_nodes;
use super::patterns::match_case_node;
use super::types::SubtreeNode;
use ruff_python_ast::Stmt;

pub(super) fn scoped_stmt_node(stmt: &Stmt) -> Option<SubtreeNode> {
    match stmt {
        Stmt::Pass(_) => Some(leaf("pass")),
        Stmt::Break(_) => Some(leaf("break")),
        Stmt::Continue(_) => Some(leaf("continue")),
        Stmt::Import(node) => Some(labeled(
            "import",
            node.names
                .iter()
                .map(|name| name.name.as_str())
                .collect::<Vec<_>>()
                .join(","),
        )),
        Stmt::ImportFrom(node) => {
            let module = node
                .module
                .as_ref()
                .map_or("", ruff_python_ast::Identifier::as_str);
            let names = node
                .names
                .iter()
                .map(|name| name.name.as_str())
                .collect::<Vec<_>>()
                .join(",");
            Some(labeled("import_from", format!("{module}::{names}")))
        }
        Stmt::Global(node) => Some(labeled("global", identifiers(&node.names))),
        Stmt::Nonlocal(node) => Some(labeled("nonlocal", identifiers(&node.names))),
        Stmt::Match(node) => {
            let mut children = extract_expr_nodes(&node.subject);
            children.extend(node.cases.iter().map(match_case_node));
            Some(SubtreeNode {
                kind: "match".into(),
                label: None,
                children,
            })
        }
        Stmt::TypeAlias(node) => {
            let mut children = extract_expr_nodes(&node.name);
            children.extend(extract_expr_nodes(&node.value));
            Some(SubtreeNode {
                kind: "type_alias".into(),
                label: None,
                children,
            })
        }
        Stmt::IpyEscapeCommand(_) => Some(leaf("ipy_escape")),
        _ => None,
    }
}

pub(super) fn misc_stmt_node(stmt: &Stmt) -> SubtreeNode {
    SubtreeNode {
        kind: format!("{stmt:?}"),
        label: None,
        children: vec![],
    }
}

fn leaf(kind: &str) -> SubtreeNode {
    SubtreeNode {
        kind: kind.into(),
        label: None,
        children: vec![],
    }
}

fn labeled(kind: &str, label: String) -> SubtreeNode {
    SubtreeNode {
        kind: kind.into(),
        label: Some(label),
        children: vec![],
    }
}

fn identifiers(names: &[ruff_python_ast::Identifier]) -> String {
    names
        .iter()
        .map(ruff_python_ast::Identifier::as_str)
        .collect::<Vec<_>>()
        .join(",")
}
