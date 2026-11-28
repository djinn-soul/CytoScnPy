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

/// Straight-line bodies only: docstrings, a parent `__init__` call
/// (e.g. `super().__init__(msg)`), assignments to `self.<attr>`, and
/// `return`. Every value must be a simple expression (literals, names,
/// attribute reads, f-strings over those), so dunders that call helpers
/// such as `render_details(payload)` still count as real logic.
fn is_simple_exception_method(body: &[ast::Stmt]) -> bool {
    body.len() <= SIMPLE_EXCEPTION_METHOD_MAX_STMTS
        && body.iter().all(|stmt| match stmt {
            ast::Stmt::Pass(_) => true,
            ast::Stmt::Expr(expr) => {
                matches!(&*expr.value, ast::Expr::StringLiteral(_))
                    || is_parent_init_call(&expr.value)
            }
            ast::Stmt::Return(ret) => ret.value.as_deref().map_or(true, is_simple_value),
            ast::Stmt::Assign(assign) => {
                assign.targets.iter().all(is_self_attribute) && is_simple_value(&assign.value)
            }
            ast::Stmt::AnnAssign(assign) => {
                is_self_attribute(&assign.target)
                    && assign.value.as_deref().map_or(true, is_simple_value)
            }
            _ => false,
        })
}

/// `super().__init__(...)` or `Base.__init__(self, ...)` with simple arguments.
fn is_parent_init_call(expr: &ast::Expr) -> bool {
    let ast::Expr::Call(call) = expr else {
        return false;
    };
    let ast::Expr::Attribute(attr) = &*call.func else {
        return false;
    };
    if attr.attr.as_str() != "__init__" {
        return false;
    }
    let receiver_ok = match &*attr.value {
        ast::Expr::Name(_) => true,
        ast::Expr::Call(inner) => {
            matches!(&*inner.func, ast::Expr::Name(name) if name.id.as_str() == "super")
                && inner.arguments.args.is_empty()
                && inner.arguments.keywords.is_empty()
        }
        _ => false,
    };
    receiver_ok
        && call.arguments.args.iter().all(|arg| match arg {
            ast::Expr::Starred(starred) => is_simple_value(&starred.value),
            other => is_simple_value(other),
        })
        && call
            .arguments
            .keywords
            .iter()
            .all(|kw| is_simple_value(&kw.value))
}

/// Literals, names, attribute reads on simple values, containers of simple
/// values, and f-strings whose interpolations are simple. No calls.
fn is_simple_value(expr: &ast::Expr) -> bool {
    match expr {
        ast::Expr::StringLiteral(_)
        | ast::Expr::BytesLiteral(_)
        | ast::Expr::NumberLiteral(_)
        | ast::Expr::BooleanLiteral(_)
        | ast::Expr::NoneLiteral(_)
        | ast::Expr::EllipsisLiteral(_)
        | ast::Expr::Name(_) => true,
        ast::Expr::Attribute(attr) => is_simple_value(&attr.value),
        ast::Expr::Tuple(tuple) => tuple.elts.iter().all(is_simple_value),
        ast::Expr::List(list) => list.elts.iter().all(is_simple_value),
        ast::Expr::FString(f_string) => f_string.value.iter().all(|part| match part {
            ast::FStringPart::Literal(_) => true,
            ast::FStringPart::FString(inner) => inner.elements.iter().all(is_simple_interpolation),
        }),
        _ => false,
    }
}

fn is_simple_interpolation(element: &ast::InterpolatedStringElement) -> bool {
    match element {
        ast::InterpolatedStringElement::Literal(_) => true,
        ast::InterpolatedStringElement::Interpolation(interpolation) => {
            is_simple_value(&interpolation.expression)
                && interpolation.format_spec.as_ref().map_or(true, |spec| {
                    spec.elements.iter().all(is_simple_interpolation)
                })
        }
    }
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
