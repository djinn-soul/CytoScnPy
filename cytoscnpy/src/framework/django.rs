use crate::framework::helpers::get_call_name;
use crate::framework::visitor::FrameworkAwareVisitor;
use ruff_python_ast::Expr;

pub(super) fn extract_urlpatterns_views(visitor: &mut FrameworkAwareVisitor, expr: &Expr) {
    match expr {
        Expr::List(list) => {
            for element in &list.elts {
                extract_path_view(visitor, element);
            }
        }
        Expr::BinOp(binop) => {
            extract_urlpatterns_views(visitor, &binop.left);
            extract_urlpatterns_views(visitor, &binop.right);
        }
        _ => {}
    }
}

fn extract_path_view(visitor: &mut FrameworkAwareVisitor, expr: &Expr) {
    if let Expr::Call(call) = expr {
        let function_name = get_call_name(&call.func);
        if (function_name == "path" || function_name == "re_path" || function_name == "url")
            && call.arguments.args.len() >= 2
        {
            extract_view_reference(visitor, &call.arguments.args[1]);
        }
    }
}

fn extract_view_reference(visitor: &mut FrameworkAwareVisitor, expr: &Expr) {
    match expr {
        Expr::Name(name) => visitor.framework_references.push(name.id.to_string()),
        Expr::Attribute(attr) => {
            if let Expr::Name(name) = &*attr.value {
                visitor.framework_references.push(name.id.to_string());
            }
        }
        Expr::Call(call) => extract_view_reference(visitor, &call.func),
        _ => {}
    }
}

pub(super) fn check_django_call_patterns(visitor: &mut FrameworkAwareVisitor, expr: &Expr) {
    if let Expr::Call(call) = expr {
        let function_name = get_call_name(&call.func);
        let is_django_registration = (function_name == "register" && is_admin_register(&call.func))
            || (function_name == "connect" && is_signal_connect(&call.func));

        if is_django_registration {
            visitor.is_framework_file = true;
            visitor.detected_frameworks.insert("django".to_owned());
            if let Some(Expr::Name(name)) = call.arguments.args.first() {
                visitor.framework_references.push(name.id.to_string());
            }
        }
    }
}

fn is_admin_register(func: &Expr) -> bool {
    if let Expr::Attribute(attr) = func {
        if let Expr::Attribute(inner) = &*attr.value {
            if inner.attr.as_str() == "site" {
                if let Expr::Name(name) = &*inner.value {
                    return name.id.as_str() == "admin";
                }
            }
        }
        if let Expr::Name(name) = &*attr.value {
            return name.id.as_str() == "admin";
        }
    }
    false
}

fn is_signal_connect(func: &Expr) -> bool {
    if let Expr::Attribute(attr) = func {
        if let Expr::Name(name) = &*attr.value {
            let signal_names = [
                "pre_save",
                "post_save",
                "pre_delete",
                "post_delete",
                "pre_init",
                "post_init",
                "m2m_changed",
                "pre_migrate",
                "post_migrate",
                "request_started",
                "request_finished",
                "got_request_exception",
            ];
            return signal_names.contains(&name.id.as_str());
        }
    }
    false
}
