use super::expressions::extract_expr_nodes;
use super::types::SubtreeNode;
use ruff_python_ast::{self as ast, Stmt};

/// Extract structural nodes from statements for tree comparison
pub(super) fn extract_stmt_nodes(body: &[Stmt]) -> Vec<SubtreeNode> {
    body.iter().map(stmt_to_node).collect()
}

/// Convert a statement to a subtree node
fn stmt_to_node(stmt: &Stmt) -> SubtreeNode {
    if let Some(node) = compound_stmt_node(stmt) {
        return node;
    }
    if let Some(node) = flow_stmt_node(stmt) {
        return node;
    }
    if let Some(node) = scoped_stmt_node(stmt) {
        return node;
    }
    misc_stmt_node(stmt)
}

fn compound_stmt_node(stmt: &Stmt) -> Option<SubtreeNode> {
    match stmt {
        Stmt::FunctionDef(f) => {
            let kind = if f.is_async {
                "async_function"
            } else {
                "function"
            };
            Some(SubtreeNode {
                kind: kind.into(),
                label: Some(f.name.to_string()),
                children: extract_stmt_nodes(&f.body),
            })
        }
        Stmt::ClassDef(c) => Some(SubtreeNode {
            kind: "class".into(),
            label: Some(c.name.to_string()),
            children: extract_stmt_nodes(&c.body),
        }),
        Stmt::For(f) => {
            let kind = if f.is_async { "async_for" } else { "for" };
            let mut children = extract_expr_nodes(&f.target);
            children.extend(extract_expr_nodes(&f.iter));
            children.extend(extract_stmt_nodes(&f.body));
            Some(SubtreeNode {
                kind: kind.into(),
                label: None,
                children,
            })
        }
        Stmt::While(w) => {
            let mut children = extract_expr_nodes(&w.test);
            children.extend(extract_stmt_nodes(&w.body));
            Some(SubtreeNode {
                kind: "while".into(),
                label: None,
                children,
            })
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
            Some(SubtreeNode {
                kind: "if".into(),
                label: None,
                children,
            })
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
            Some(SubtreeNode {
                kind: kind.into(),
                label: None,
                children,
            })
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
            Some(SubtreeNode {
                kind: "try".into(),
                label: None,
                children,
            })
        }
        _ => None,
    }
}

fn flow_stmt_node(stmt: &Stmt) -> Option<SubtreeNode> {
    match stmt {
        Stmt::Return(r) => {
            let children = r
                .value
                .as_ref()
                .map_or(vec![], |expr| extract_expr_nodes(expr.as_ref()));
            Some(SubtreeNode {
                kind: "return".into(),
                label: None,
                children,
            })
        }
        Stmt::Assign(a) => {
            let mut children = vec![];
            for target in &a.targets {
                children.extend(extract_expr_nodes(target));
            }
            children.extend(extract_expr_nodes(&a.value));
            Some(SubtreeNode {
                kind: "assign".into(),
                label: None,
                children,
            })
        }
        Stmt::AugAssign(a) => {
            let mut children = extract_expr_nodes(&a.target);
            children.extend(extract_expr_nodes(&a.value));
            Some(SubtreeNode {
                kind: "aug_assign".into(),
                label: None,
                children,
            })
        }
        Stmt::AnnAssign(a) => {
            let mut children = extract_expr_nodes(&a.target);
            if let Some(value) = &a.value {
                children.extend(extract_expr_nodes(value));
            }
            Some(SubtreeNode {
                kind: "ann_assign".into(),
                label: None,
                children,
            })
        }
        Stmt::Expr(e) => Some(SubtreeNode {
            kind: "expr".into(),
            label: None,
            children: extract_expr_nodes(&e.value),
        }),
        Stmt::Raise(r) => {
            let mut children = vec![];
            if let Some(exc) = &r.exc {
                children.extend(extract_expr_nodes(exc));
            }
            if let Some(cause) = &r.cause {
                children.extend(extract_expr_nodes(cause));
            }
            Some(SubtreeNode {
                kind: "raise".into(),
                label: None,
                children,
            })
        }
        Stmt::Assert(a) => {
            let mut children = extract_expr_nodes(&a.test);
            if let Some(msg) = &a.msg {
                children.extend(extract_expr_nodes(msg));
            }
            Some(SubtreeNode {
                kind: "assert".into(),
                label: None,
                children,
            })
        }
        Stmt::Delete(d) => {
            let children = d.targets.iter().flat_map(extract_expr_nodes).collect();
            Some(SubtreeNode {
                kind: "delete".into(),
                label: None,
                children,
            })
        }
        _ => None,
    }
}

fn scoped_stmt_node(stmt: &Stmt) -> Option<SubtreeNode> {
    match stmt {
        Stmt::Pass(_) => Some(SubtreeNode {
            kind: "pass".into(),
            label: None,
            children: vec![],
        }),
        Stmt::Break(_) => Some(SubtreeNode {
            kind: "break".into(),
            label: None,
            children: vec![],
        }),
        Stmt::Continue(_) => Some(SubtreeNode {
            kind: "continue".into(),
            label: None,
            children: vec![],
        }),
        Stmt::Import(i) => {
            let labels: Vec<String> = i.names.iter().map(|n| n.name.as_str().to_owned()).collect();
            Some(SubtreeNode {
                kind: "import".into(),
                label: Some(labels.join(",")),
                children: vec![],
            })
        }
        Stmt::ImportFrom(i) => {
            let module = i
                .module
                .as_ref()
                .map_or("", ruff_python_ast::Identifier::as_str)
                .to_owned();
            let labels: Vec<String> = i.names.iter().map(|n| n.name.as_str().to_owned()).collect();
            Some(SubtreeNode {
                kind: "import_from".into(),
                label: Some(format!("{}::{}", module, labels.join(","))),
                children: vec![],
            })
        }
        Stmt::Global(g) => Some(SubtreeNode {
            kind: "global".into(),
            label: Some(
                g.names
                    .iter()
                    .map(ruff_python_ast::Identifier::as_str)
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            children: vec![],
        }),
        Stmt::Nonlocal(n) => Some(SubtreeNode {
            kind: "nonlocal".into(),
            label: Some(
                n.names
                    .iter()
                    .map(ruff_python_ast::Identifier::as_str)
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            children: vec![],
        }),
        Stmt::Match(m) => Some(SubtreeNode {
            kind: "match".into(),
            label: None,
            children: {
                let mut children = extract_expr_nodes(&m.subject);
                children.extend(m.cases.iter().flat_map(|c| extract_stmt_nodes(&c.body)));
                children
            },
        }),
        Stmt::TypeAlias(t) => {
            let mut children = extract_expr_nodes(&t.name);
            children.extend(extract_expr_nodes(&t.value));
            Some(SubtreeNode {
                kind: "type_alias".into(),
                label: None,
                children,
            })
        }
        Stmt::IpyEscapeCommand(_) => Some(SubtreeNode {
            kind: "ipy_escape".into(),
            label: None,
            children: vec![],
        }),
        _ => None,
    }
}

fn misc_stmt_node(stmt: &Stmt) -> SubtreeNode {
    // Fallback for statements that are syntactically valid but not yet modeled.
    SubtreeNode {
        kind: format!("{stmt:?}"),
        label: None,
        children: vec![],
    }
}
