use super::definitions::{class_nodes, function_nodes};
use super::expressions::extract_expr_nodes;
use super::statement_misc::{misc_stmt_node, scoped_stmt_node};
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
            let mut children = function_nodes(f);
            children.extend(extract_stmt_nodes(&f.body));
            Some(SubtreeNode {
                kind: kind.into(),
                label: Some(f.name.to_string()),
                children,
            })
        }
        Stmt::ClassDef(c) => {
            let mut children = class_nodes(c);
            children.extend(extract_stmt_nodes(&c.body));
            Some(SubtreeNode {
                kind: "class".into(),
                label: Some(c.name.to_string()),
                children,
            })
        }
        Stmt::For(f) => {
            let kind = if f.is_async { "async_for" } else { "for" };
            let mut children = extract_expr_nodes(&f.target);
            children.extend(extract_expr_nodes(&f.iter));
            children.extend(extract_stmt_nodes(&f.body));
            if !f.orelse.is_empty() {
                children.push(branch_node("for_else", &f.orelse));
            }
            Some(SubtreeNode {
                kind: kind.into(),
                label: None,
                children,
            })
        }
        Stmt::While(w) => {
            let mut children = extract_expr_nodes(&w.test);
            children.extend(extract_stmt_nodes(&w.body));
            if !w.orelse.is_empty() {
                children.push(branch_node("while_else", &w.orelse));
            }
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
                let kind = if clause.test.is_some() {
                    "elif"
                } else {
                    "else"
                };
                let mut branch_children = Vec::new();
                if let Some(test) = &clause.test {
                    branch_children.extend(extract_expr_nodes(test));
                }
                branch_children.extend(extract_stmt_nodes(&clause.body));
                children.push(SubtreeNode {
                    kind: kind.into(),
                    label: None,
                    children: branch_children,
                });
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
                        children.push(branch_node("except", &h.body));
                    }
                }
            }
            if !t.orelse.is_empty() {
                children.push(branch_node("try_else", &t.orelse));
            }
            if !t.finalbody.is_empty() {
                children.push(branch_node("finally", &t.finalbody));
            }
            Some(SubtreeNode {
                kind: "try".into(),
                label: None,
                children,
            })
        }
        _ => None,
    }
}

fn branch_node(kind: &str, body: &[Stmt]) -> SubtreeNode {
    SubtreeNode {
        kind: kind.into(),
        label: None,
        children: extract_stmt_nodes(body),
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
                label: Some(format!("{:?}", a.op)),
                children,
            })
        }
        Stmt::AnnAssign(a) => {
            let mut children = extract_expr_nodes(&a.target);
            children.extend(extract_expr_nodes(&a.annotation));
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
