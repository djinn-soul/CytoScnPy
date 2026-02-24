use super::expressions::extract_expr_nodes;
use super::types::SubtreeNode;
use ruff_python_ast::{self as ast, Stmt};

/// Extract structural nodes from statements for tree comparison
pub(super) fn extract_stmt_nodes(body: &[Stmt]) -> Vec<SubtreeNode> {
    body.iter().map(stmt_to_node).collect()
}

/// Convert a statement to a subtree node
#[allow(clippy::too_many_lines)]
fn stmt_to_node(stmt: &Stmt) -> SubtreeNode {
    match stmt {
        Stmt::FunctionDef(f) => {
            let kind = if f.is_async {
                "async_function"
            } else {
                "function"
            };
            SubtreeNode {
                kind: kind.into(),
                label: Some(f.name.to_string()),
                children: extract_stmt_nodes(&f.body),
            }
        }
        Stmt::ClassDef(c) => SubtreeNode {
            kind: "class".into(),
            label: Some(c.name.to_string()),
            children: extract_stmt_nodes(&c.body),
        },
        Stmt::Return(r) => {
            let children = r
                .value
                .as_ref()
                .map_or(vec![], |expr| extract_expr_nodes(expr.as_ref()));
            SubtreeNode {
                kind: "return".into(),
                label: None,
                children,
            }
        }
        Stmt::Assign(a) => {
            let mut children = vec![];
            for target in &a.targets {
                children.extend(extract_expr_nodes(target));
            }
            children.extend(extract_expr_nodes(&a.value));
            SubtreeNode {
                kind: "assign".into(),
                label: None,
                children,
            }
        }
        Stmt::AugAssign(a) => {
            let mut children = extract_expr_nodes(&a.target);
            children.extend(extract_expr_nodes(&a.value));
            SubtreeNode {
                kind: "aug_assign".into(),
                label: None,
                children,
            }
        }
        Stmt::AnnAssign(a) => {
            let mut children = extract_expr_nodes(&a.target);
            if let Some(value) = &a.value {
                children.extend(extract_expr_nodes(value));
            }
            SubtreeNode {
                kind: "ann_assign".into(),
                label: None,
                children,
            }
        }
        Stmt::For(f) => {
            let kind = if f.is_async { "async_for" } else { "for" };
            let mut children = extract_expr_nodes(&f.target);
            children.extend(extract_expr_nodes(&f.iter));
            children.extend(extract_stmt_nodes(&f.body));
            SubtreeNode {
                kind: kind.into(),
                label: None,
                children,
            }
        }
        Stmt::While(w) => {
            let mut children = extract_expr_nodes(&w.test);
            children.extend(extract_stmt_nodes(&w.body));
            SubtreeNode {
                kind: "while".into(),
                label: None,
                children,
            }
        }
        Stmt::If(i) => {
            let mut children = extract_expr_nodes(&i.test);
            children.extend(extract_stmt_nodes(&i.body));
            for clause in &i.elif_else_clauses {
                if let Some(test) = &clause.test {
                    children.extend(extract_expr_nodes(test));
                }
                children.extend(extract_stmt_nodes(&clause.body));
            }
            SubtreeNode {
                kind: "if".into(),
                label: None,
                children,
            }
        }
        Stmt::With(w) => {
            let kind = if w.is_async { "async_with" } else { "with" };
            let mut children = vec![];
            for item in &w.items {
                children.extend(extract_expr_nodes(&item.context_expr));
                if let Some(opt) = &item.optional_vars {
                    children.extend(extract_expr_nodes(opt));
                }
            }
            children.extend(extract_stmt_nodes(&w.body));
            SubtreeNode {
                kind: kind.into(),
                label: None,
                children,
            }
        }
        Stmt::Try(t) => {
            let mut children = extract_stmt_nodes(&t.body);
            for handler in &t.handlers {
                match handler {
                    ast::ExceptHandler::ExceptHandler(h) => {
                        if let Some(type_) = &h.type_ {
                            children.extend(extract_expr_nodes(type_));
                        }
                        children.extend(extract_stmt_nodes(&h.body));
                    }
                }
            }
            children.extend(extract_stmt_nodes(&t.orelse));
            children.extend(extract_stmt_nodes(&t.finalbody));
            SubtreeNode {
                kind: "try".into(),
                label: None,
                children,
            }
        }
        Stmt::Expr(e) => SubtreeNode {
            kind: "expr".into(),
            label: None,
            children: extract_expr_nodes(&e.value),
        },
        Stmt::Pass(_) => SubtreeNode {
            kind: "pass".into(),
            label: None,
            children: vec![],
        },
        Stmt::Break(_) => SubtreeNode {
            kind: "break".into(),
            label: None,
            children: vec![],
        },
        Stmt::Continue(_) => SubtreeNode {
            kind: "continue".into(),
            label: None,
            children: vec![],
        },
        Stmt::Raise(r) => {
            let mut children = vec![];
            if let Some(exc) = &r.exc {
                children.extend(extract_expr_nodes(exc));
            }
            if let Some(cause) = &r.cause {
                children.extend(extract_expr_nodes(cause));
            }
            SubtreeNode {
                kind: "raise".into(),
                label: None,
                children,
            }
        }
        Stmt::Assert(a) => {
            let mut children = extract_expr_nodes(&a.test);
            if let Some(msg) = &a.msg {
                children.extend(extract_expr_nodes(msg));
            }
            SubtreeNode {
                kind: "assert".into(),
                label: None,
                children,
            }
        }
        Stmt::Import(i) => {
            let labels: Vec<String> = i.names.iter().map(|n| n.name.as_str().to_owned()).collect();
            SubtreeNode {
                kind: "import".into(),
                label: Some(labels.join(",")),
                children: vec![],
            }
        }
        Stmt::ImportFrom(i) => {
            let module = i
                .module
                .as_ref()
                .map_or("", ruff_python_ast::Identifier::as_str)
                .to_owned();
            let labels: Vec<String> = i.names.iter().map(|n| n.name.as_str().to_owned()).collect();
            SubtreeNode {
                kind: "import_from".into(),
                label: Some(format!("{}::{}", module, labels.join(","))),
                children: vec![],
            }
        }
        Stmt::Global(g) => SubtreeNode {
            kind: "global".into(),
            label: Some(
                g.names
                    .iter()
                    .map(ruff_python_ast::Identifier::as_str)
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            children: vec![],
        },
        Stmt::Nonlocal(n) => SubtreeNode {
            kind: "nonlocal".into(),
            label: Some(
                n.names
                    .iter()
                    .map(ruff_python_ast::Identifier::as_str)
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            children: vec![],
        },
        Stmt::Match(m) => SubtreeNode {
            kind: "match".into(),
            label: None,
            children: {
                let mut children = extract_expr_nodes(&m.subject);
                children.extend(m.cases.iter().flat_map(|c| extract_stmt_nodes(&c.body)));
                children
            },
        },
        Stmt::TypeAlias(t) => {
            let mut children = extract_expr_nodes(&t.name);
            children.extend(extract_expr_nodes(&t.value));
            SubtreeNode {
                kind: "type_alias".into(),
                label: None,
                children,
            }
        }
        Stmt::Delete(d) => {
            let children = d.targets.iter().flat_map(extract_expr_nodes).collect();
            SubtreeNode {
                kind: "delete".into(),
                label: None,
                children,
            }
        }
        Stmt::IpyEscapeCommand(_) => SubtreeNode {
            kind: "ipy_escape".into(),
            label: None,
            children: vec![],
        },
    }
}
