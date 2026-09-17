//! Python AST visitor for detecting singleton patterns.

use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;
use std::path::Path;

use super::types::{SingletonKind, SingletonMatch};
use crate::utils::LineIndex;

const MAX_SNIPPET: usize = 120;

/// Detects singleton patterns in Python source code.
#[must_use]
pub fn detect_python_singletons(source: &str, file: &Path) -> Vec<SingletonMatch> {
    let Ok(parsed) = parse_module(source) else {
        return Vec::new();
    };

    let line_index = LineIndex::new(source);
    let mut matches = Vec::new();

    visit_statements(parsed.suite(), source, file, &line_index, &mut matches);

    matches
}

fn visit_statements(
    stmts: &[Stmt],
    source: &str,
    file: &Path,
    line_index: &LineIndex,
    matches: &mut Vec<SingletonMatch>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::ClassDef(class_def) => {
                check_class(class_def, source, file, line_index, matches);
                visit_statements(&class_def.body, source, file, line_index, matches);
            }
            Stmt::FunctionDef(func_def) => {
                visit_statements(&func_def.body, source, file, line_index, matches);
            }
            Stmt::If(if_stmt) => {
                visit_statements(&if_stmt.body, source, file, line_index, matches);
                for clause in &if_stmt.elif_else_clauses {
                    visit_statements(&clause.body, source, file, line_index, matches);
                }
            }
            Stmt::For(for_stmt) => {
                visit_statements(&for_stmt.body, source, file, line_index, matches);
                visit_statements(&for_stmt.orelse, source, file, line_index, matches);
            }
            Stmt::While(while_stmt) => {
                visit_statements(&while_stmt.body, source, file, line_index, matches);
                visit_statements(&while_stmt.orelse, source, file, line_index, matches);
            }
            Stmt::Try(try_stmt) => {
                visit_statements(&try_stmt.body, source, file, line_index, matches);
                for h in &try_stmt.handlers {
                    let ast::ExceptHandler::ExceptHandler(handler) = h;
                    visit_statements(&handler.body, source, file, line_index, matches);
                }
                visit_statements(&try_stmt.orelse, source, file, line_index, matches);
                visit_statements(&try_stmt.finalbody, source, file, line_index, matches);
            }
            _ => {}
        }
    }
}

fn check_class(
    class_def: &ast::StmtClassDef,
    source: &str,
    file: &Path,
    line_index: &LineIndex,
    matches: &mut Vec<SingletonMatch>,
) {
    let line = line_index.line_index(class_def.range().start());
    let column = line_index.column_index(class_def.range().start());
    let class_name = class_def.name.as_str().to_owned();

    // 1. Check decorator (e.g. @singleton or @Singleton)
    for decorator in &class_def.decorator_list {
        if is_singleton_decorator(&decorator.expression) {
            matches.push(SingletonMatch {
                file: file.to_path_buf(),
                line,
                column,
                class_name,
                kind: SingletonKind::Decorator,
                snippet: get_line_snippet(source, line),
            });
            return;
        }
    }

    // 2. Check metaclass=Singleton argument
    if let Some(args) = &class_def.arguments {
        for keyword in &args.keywords {
            if keyword
                .arg
                .as_ref()
                .is_some_and(|a| a.as_str() == "metaclass")
                && is_singleton_metaclass_expr(&keyword.value)
            {
                matches.push(SingletonMatch {
                    file: file.to_path_buf(),
                    line,
                    column,
                    class_name,
                    kind: SingletonKind::Metaclass,
                    snippet: get_line_snippet(source, line),
                });
                return;
            }
        }
    }

    // 3. Inspect class body for instance cache and methods
    let mut has_instance_field = false;
    let mut has_new_singleton = false;
    let mut has_get_instance = false;

    for body_stmt in &class_def.body {
        match body_stmt {
            Stmt::Assign(assign) => {
                for target in &assign.targets {
                    if is_instance_name(target) {
                        has_instance_field = true;
                    }
                }
            }
            Stmt::AnnAssign(ann) => {
                if is_instance_name(&ann.target) {
                    has_instance_field = true;
                }
            }
            Stmt::FunctionDef(fn_def) => {
                let fn_name = fn_def.name.as_str();
                if fn_name == "__new__" {
                    if function_body_references_instance(&fn_def.body) {
                        has_new_singleton = true;
                    }
                } else if is_get_instance_name(fn_name) {
                    has_get_instance = true;
                }
            }
            _ => {}
        }
    }

    if has_new_singleton {
        matches.push(SingletonMatch {
            file: file.to_path_buf(),
            line,
            column,
            class_name,
            kind: SingletonKind::NewMethod,
            snippet: get_line_snippet(source, line),
        });
    } else if has_instance_field && has_get_instance {
        matches.push(SingletonMatch {
            file: file.to_path_buf(),
            line,
            column,
            class_name,
            kind: SingletonKind::GetInstanceMethod,
            snippet: get_line_snippet(source, line),
        });
    }
}

fn is_singleton_decorator(expr: &Expr) -> bool {
    match expr {
        Expr::Name(n) => {
            let id = n.id.as_str();
            id.eq_ignore_ascii_case("singleton")
        }
        Expr::Call(call) => is_singleton_decorator(&call.func),
        Expr::Attribute(attr) => attr.attr.as_str().eq_ignore_ascii_case("singleton"),
        _ => false,
    }
}

fn is_singleton_metaclass_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Name(n) => {
            let id = n.id.as_str();
            id == "Singleton" || id == "SingletonMeta" || id == "SingletonType"
        }
        Expr::Attribute(attr) => {
            let name = attr.attr.as_str();
            name == "Singleton" || name == "SingletonMeta" || name == "SingletonType"
        }
        _ => false,
    }
}

fn is_instance_name(expr: &Expr) -> bool {
    if let Expr::Name(n) = expr {
        let name = n.id.as_str();
        name == "_instance" || name == "__instance" || name == "_instance_" || name == "instance"
    } else {
        false
    }
}

fn is_get_instance_name(name: &str) -> bool {
    name == "get_instance"
        || name == "getInstance"
        || name == "get_singleton"
        || name == "getSingleton"
        || name == "instance"
}

fn function_body_references_instance(body: &[Stmt]) -> bool {
    let mut found = false;
    for stmt in body {
        if statement_contains_instance_ref(stmt) {
            found = true;
            break;
        }
    }
    found
}

fn statement_contains_instance_ref(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Assign(a) => {
            a.targets.iter().any(is_instance_or_attr) || is_instance_or_attr(&a.value)
        }
        Stmt::If(i) => {
            is_instance_or_attr(&i.test)
                || i.body.iter().any(statement_contains_instance_ref)
                || i.elif_else_clauses
                    .iter()
                    .any(|c| c.body.iter().any(statement_contains_instance_ref))
        }
        Stmt::Return(r) => r.value.as_ref().is_some_and(|v| is_instance_or_attr(v)),
        _ => false,
    }
}

fn is_instance_or_attr(expr: &Expr) -> bool {
    match expr {
        Expr::Name(n) => {
            let id = n.id.as_str();
            id == "_instance" || id == "__instance" || id == "instance"
        }
        Expr::Attribute(a) => {
            let attr = a.attr.as_str();
            attr == "_instance" || attr == "__instance" || attr == "instance"
        }
        Expr::Compare(comp) => {
            is_instance_or_attr(&comp.left) || comp.comparators.iter().any(is_instance_or_attr)
        }
        Expr::UnaryOp(u) => is_instance_or_attr(&u.operand),
        _ => false,
    }
}

fn get_line_snippet(source: &str, line_1_indexed: usize) -> String {
    source
        .lines()
        .nth(line_1_indexed.saturating_sub(1))
        .unwrap_or("")
        .trim()
        .chars()
        .take(MAX_SNIPPET)
        .collect()
}
