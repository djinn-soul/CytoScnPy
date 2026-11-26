use ruff_python_ast as ast;

/// Determines whether a class definition is a pure interface / protocol / stub class.
/// Pure interfaces contain no executable implementation logic to deduplicate, so
/// extracting them as clone candidates produces high-volume false positives.
///
/// Every method must be a stub (or `@abstractmethod`) and the body may hold
/// nothing but docstrings, `pass` and annotations. A class with no methods
/// at all only qualifies when it explicitly derives from `ABC`/`Protocol`.
pub(super) fn is_pure_interface_class(class: &ast::StmtClassDef) -> bool {
    let mut has_methods = false;

    for stmt in &class.body {
        match stmt {
            ast::Stmt::FunctionDef(f) => {
                if !is_stub_method_body(&f.body) && !has_abstract_decorator(&f.decorator_list) {
                    return false;
                }
                has_methods = true;
            }
            // Allow docstrings, pass, and type annotations
            ast::Stmt::Pass(_) | ast::Stmt::Expr(_) | ast::Stmt::AnnAssign(_) => {}
            _ => return false,
        }
    }

    has_methods || class.bases().iter().any(is_interface_base)
}

/// Maximum statements in a "simple" exception dunder method.
const SIMPLE_EXCEPTION_METHOD_MAX_STMTS: usize = 6;

/// Determines whether a class is a simple custom exception definition.
/// Exception classes that only declare class attributes and short, straight-line
/// `__init__` / `__str__` / `__repr__` methods are domain error types, not
/// procedural logic to deduplicate.
pub(super) fn is_simple_exception_class(class: &ast::StmtClassDef) -> bool {
    if !class.bases().iter().any(is_exception_base) {
        return false;
    }

    class.body.iter().all(|stmt| match stmt {
        ast::Stmt::FunctionDef(f) => {
            matches!(f.name.as_str(), "__init__" | "__str__" | "__repr__")
                && is_simple_exception_method(&f.body)
        }
        ast::Stmt::Pass(_)
        | ast::Stmt::Expr(_)
        | ast::Stmt::Assign(_)
        | ast::Stmt::AnnAssign(_) => true,
        _ => false,
    })
}

/// Straight-line bodies only: docstrings / calls (e.g. `super().__init__(msg)`),
/// assignments to `self.<attr>`, and `return`. No control flow or locals.
fn is_simple_exception_method(body: &[ast::Stmt]) -> bool {
    body.len() <= SIMPLE_EXCEPTION_METHOD_MAX_STMTS
        && body.iter().all(|stmt| match stmt {
            ast::Stmt::Pass(_) | ast::Stmt::Expr(_) | ast::Stmt::Return(_) => true,
            ast::Stmt::Assign(assign) => assign.targets.iter().all(is_self_attribute),
            ast::Stmt::AnnAssign(assign) => is_self_attribute(&assign.target),
            _ => false,
        })
}

fn is_self_attribute(target: &ast::Expr) -> bool {
    matches!(target, ast::Expr::Attribute(attr)
        if matches!(&*attr.value, ast::Expr::Name(name) if name.id.as_str() == "self"))
}

fn is_exception_base(expr: &ast::Expr) -> bool {
    let name = match expr {
        ast::Expr::Name(name) => name.id.as_str(),
        ast::Expr::Attribute(attr) => attr.attr.as_str(),
        _ => return false,
    };
    matches!(name, "Exception" | "BaseException")
        || name.ends_with("Error")
        || name.ends_with("Exception")
}

/// Only the standard interface markers count; names that merely end in
/// `Protocol`/`Interface` (e.g. `asyncio.Protocol` subclasses) are often
/// concrete implementations.
fn is_interface_base(expr: &ast::Expr) -> bool {
    match expr {
        ast::Expr::Name(name) => matches!(name.id.as_str(), "ABC" | "Protocol"),
        ast::Expr::Attribute(attr) => {
            let module_ok = matches!(&*attr.value, ast::Expr::Name(module)
                if matches!(module.id.as_str(), "abc" | "typing" | "typing_extensions"));
            module_ok && matches!(attr.attr.as_str(), "ABC" | "Protocol")
        }
        ast::Expr::Subscript(sub) => is_interface_base(&sub.value),
        _ => false,
    }
}

fn is_stub_method_body(body: &[ast::Stmt]) -> bool {
    if body.is_empty() {
        return true;
    }
    body.iter().all(|stmt| match stmt {
        ast::Stmt::Pass(_) => true,
        ast::Stmt::Expr(expr) => matches!(
            &*expr.value,
            ast::Expr::EllipsisLiteral(_) | ast::Expr::StringLiteral(_)
        ),
        ast::Stmt::Raise(r) => r
            .exc
            .as_ref()
            .is_some_and(|exc| is_not_implemented_error(exc)),
        ast::Stmt::Return(ret) => {
            ret.value.is_none()
                || ret
                    .value
                    .as_ref()
                    .is_some_and(|v| matches!(&**v, ast::Expr::NoneLiteral(_)))
        }
        _ => false,
    })
}

fn is_not_implemented_error(expr: &ast::Expr) -> bool {
    match expr {
        ast::Expr::Name(name) => name.id.as_str() == "NotImplementedError",
        ast::Expr::Call(call) => is_not_implemented_error(&call.func),
        ast::Expr::Attribute(attr) => attr.attr.as_str() == "NotImplementedError",
        _ => false,
    }
}

fn has_abstract_decorator(decorators: &[ast::Decorator]) -> bool {
    decorators.iter().any(|d| {
        let expr = match &d.expression {
            ast::Expr::Call(c) => &c.func,
            other => other,
        };
        match expr {
            ast::Expr::Name(name) => name.id.as_str() == "abstractmethod",
            ast::Expr::Attribute(attr) => attr.attr.as_str() == "abstractmethod",
            _ => false,
        }
    })
}
