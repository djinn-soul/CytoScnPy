use crate::framework::helpers::get_call_name;
use crate::framework::visitor::FrameworkAwareVisitor;
use ruff_python_ast::{Expr, Parameters};

pub(super) fn extract_fastapi_depends(visitor: &mut FrameworkAwareVisitor, args: &Parameters) {
    for arg in &args.args {
        if let Some(default) = &arg.default {
            check_depends_call(visitor, default);
        }
        if let Some(annotation) = &arg.parameter.annotation {
            check_depends_call(visitor, annotation);
        }
    }
    for arg in &args.posonlyargs {
        if let Some(default) = &arg.default {
            check_depends_call(visitor, default);
        }
        if let Some(annotation) = &arg.parameter.annotation {
            check_depends_call(visitor, annotation);
        }
    }
    for arg in &args.kwonlyargs {
        if let Some(default) = &arg.default {
            check_depends_call(visitor, default);
        }
        if let Some(annotation) = &arg.parameter.annotation {
            check_depends_call(visitor, annotation);
        }
    }
}

fn check_depends_call(visitor: &mut FrameworkAwareVisitor, expr: &Expr) {
    if let Expr::Call(call) = expr {
        if get_call_name(&call.func) == "Depends" {
            visitor.is_framework_file = true;
            visitor.detected_frameworks.insert("fastapi".to_owned());
            if let Some(first_arg) = call.arguments.args.first() {
                match first_arg {
                    Expr::Name(name) => visitor.framework_references.push(name.id.to_string()),
                    Expr::Attribute(attr) => {
                        if let Expr::Name(name) = &*attr.value {
                            visitor.framework_references.push(name.id.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    match expr {
        Expr::Subscript(node) => check_depends_call(visitor, &node.slice),
        Expr::Tuple(node) => {
            for element in &node.elts {
                check_depends_call(visitor, element);
            }
        }
        _ => {}
    }
}
