//! Taint source detection.
//!
//! Identifies where untrusted user input enters the program.

use super::types::{TaintInfo, TaintSource};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

/// Checks if an expression is a taint source and returns the taint info.
pub fn check_taint_source(expr: &Expr) -> Option<TaintInfo> {
    match expr {
        // Check for function calls that return tainted data
        Expr::Call(call) => check_call_source(call),
        // Check for attribute access on request objects
        Expr::Attribute(attr) => check_attribute_source(attr),
        // Check for subscript on request objects (request.args['key'])
        Expr::Subscript(sub) => check_subscript_source(sub),
        _ => None,
    }
}

/// Checks if a call expression is a taint source.
fn check_call_source(call: &ast::ExprCall) -> Option<TaintInfo> {
    let line = call.range().start().to_u32() as usize;

    // Get the function name
    if let Some(name) = get_call_name(&call.func) {
        // input() builtin
        if name == "input" {
            return Some(TaintInfo::new(TaintSource::Input, line));
        }

        // os.getenv() or os.environ.get()
        if name == "os.getenv" || name == "getenv" || name == "os.environ.get" {
            return Some(TaintInfo::new(TaintSource::Environment, line));
        }

        // Flask request methods: request.args.get(), request.form.get(), etc.
        if name.starts_with("request.args.")
            || name.starts_with("request.form.")
            || name.starts_with("request.data.")
            || name.starts_with("request.json.")
            || name.starts_with("request.cookies.")
            || name.starts_with("request.files.")
        {
            let attr = name.split('.').nth(1).unwrap_or("args");
            return Some(TaintInfo::new(
                TaintSource::FlaskRequest(attr.to_owned()),
                line,
            ));
        }

        // Django request methods
        if name.starts_with("request.GET.")
            || name.starts_with("request.POST.")
            || name.starts_with("request.COOKIES.")
        {
            let attr = name.split('.').nth(1).unwrap_or("GET");
            return Some(TaintInfo::new(
                TaintSource::DjangoRequest(attr.to_owned()),
                line,
            ));
        }

        // File reads
        // This is not a file extension comparison - we're checking method name suffixes
        #[allow(clippy::case_sensitive_file_extension_comparisons)]
        if name.ends_with(".read") || name.ends_with(".readlines") || name.ends_with(".readline") {
            return Some(TaintInfo::new(TaintSource::FileRead, line));
        }

        // JSON/YAML load
        if name == "json.load"
            || name == "json.loads"
            || name == "yaml.load"
            || name == "yaml.safe_load"
        {
            return Some(TaintInfo::new(TaintSource::ExternalData, line));
        }
    }

    None
}

/// Checks if an attribute expression is a taint source.
fn check_attribute_source(attr: &ast::ExprAttribute) -> Option<TaintInfo> {
    let line = attr.range().start().to_u32() as usize;
    let attr_name = attr.attr.as_str();

    // Check if the value is 'request'
    if let Expr::Name(name) = &*attr.value {
        if name.id.as_str() == "request" {
            // Flask sources
            match attr_name {
                "args" | "form" | "data" | "json" | "cookies" | "files" | "values" => {
                    return Some(TaintInfo::new(
                        TaintSource::FlaskRequest(attr_name.to_owned()),
                        line,
                    ));
                }
                // Django sources
                "GET" | "POST" | "body" | "COOKIES" => {
                    return Some(TaintInfo::new(
                        TaintSource::DjangoRequest(attr_name.to_owned()),
                        line,
                    ));
                }
                _ => {}
            }
        }

        // sys.argv
        if name.id.as_str() == "sys" && attr_name == "argv" {
            return Some(TaintInfo::new(TaintSource::CommandLine, line));
        }

        // os.environ
        if name.id.as_str() == "os" && attr_name == "environ" {
            return Some(TaintInfo::new(TaintSource::Environment, line));
        }
    }

    // Check for chained attribute access like request.args.get
    if let Expr::Attribute(inner) = &*attr.value {
        if let Expr::Name(name) = &*inner.value {
            if name.id.as_str() == "request" {
                let inner_attr = inner.attr.as_str();
                // request.args, request.form, etc.
                match inner_attr {
                    "args" | "form" | "data" | "json" | "cookies" | "files" => {
                        return Some(TaintInfo::new(
                            TaintSource::FlaskRequest(inner_attr.to_owned()),
                            line,
                        ));
                    }
                    "GET" | "POST" | "COOKIES" => {
                        return Some(TaintInfo::new(
                            TaintSource::DjangoRequest(inner_attr.to_owned()),
                            line,
                        ));
                    }
                    _ => {}
                }
            }
        }
    }

    None
}

/// Checks if a subscript expression is a taint source.
fn check_subscript_source(sub: &ast::ExprSubscript) -> Option<TaintInfo> {
    let line = sub.range().start().to_u32() as usize;

    // Check for request.args['key'] or request['key']
    if let Expr::Attribute(attr) = &*sub.value {
        if let Expr::Name(name) = &*attr.value {
            if name.id.as_str() == "request" {
                let attr_name = attr.attr.as_str();
                match attr_name {
                    "args" | "form" | "data" | "json" | "cookies" | "files" => {
                        return Some(TaintInfo::new(
                            TaintSource::FlaskRequest(attr_name.to_owned()),
                            line,
                        ));
                    }
                    "GET" | "POST" | "COOKIES" => {
                        return Some(TaintInfo::new(
                            TaintSource::DjangoRequest(attr_name.to_owned()),
                            line,
                        ));
                    }
                    _ => {}
                }
            }
        }
    }

    // os.environ['VAR']
    if let Expr::Attribute(attr) = &*sub.value {
        if let Expr::Name(name) = &*attr.value {
            if name.id.as_str() == "os" && attr.attr.as_str() == "environ" {
                return Some(TaintInfo::new(TaintSource::Environment, line));
            }
        }
    }

    // sys.argv[0]
    if let Expr::Attribute(attr) = &*sub.value {
        if let Expr::Name(name) = &*attr.value {
            if name.id.as_str() == "sys" && attr.attr.as_str() == "argv" {
                return Some(TaintInfo::new(TaintSource::CommandLine, line));
            }
        }
    }

    None
}

/// Extracts the call name from a function expression.
fn get_call_name(func: &Expr) -> Option<String> {
    match func {
        Expr::Name(node) => Some(node.id.to_string()),
        Expr::Attribute(node) => {
            if let Expr::Name(value) = &*node.value {
                Some(format!("{}.{}", value.id, node.attr))
            } else if let Expr::Attribute(inner) = &*node.value {
                // Handle chained attributes like request.args.get
                if let Expr::Name(name) = &*inner.value {
                    Some(format!("{}.{}.{}", name.id, inner.attr, node.attr))
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Checks if a `FastAPI` function parameter is tainted.
pub fn check_fastapi_param(func_def: &ast::StmtFunctionDef) -> Vec<(String, TaintInfo)> {
    let mut tainted_params = Vec::new();
    let line = func_def.range().start().to_u32() as usize;

    // Check for Query(), Path(), Body(), Form() in parameter defaults
    for arg in &func_def.parameters.args {
        if let Some(default) = &arg.default {
            if let Expr::Call(call) = &**default {
                if let Some(name) = get_call_name(&call.func) {
                    let param_name = arg.parameter.name.as_str();
                    match name.as_str() {
                        "Query" | "Path" | "Body" | "Form" | "Header" | "Cookie" => {
                            let source = TaintSource::FastApiParam(param_name.to_owned());
                            tainted_params
                                .push((param_name.to_owned(), TaintInfo::new(source, line)));
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    tainted_params
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruff_python_parser::{parse, Mode};

    fn parse_expr(source: &str) -> Expr {
        let tree = parse(source, Mode::Expression.into()).unwrap();
        if let ast::Mod::Expression(expr) = tree.into_syntax() {
            *expr.body
        } else {
            panic!("Expected expression")
        }
    }

    #[test]
    fn test_input_source() {
        let expr = parse_expr("input()");
        let taint = check_taint_source(&expr);
        assert!(taint.is_some());
        assert!(matches!(taint.unwrap().source, TaintSource::Input));
    }

    #[test]
    fn test_flask_request_args() {
        let expr = parse_expr("request.args");
        let taint = check_taint_source(&expr);
        assert!(taint.is_some());
        assert!(matches!(
            taint.unwrap().source,
            TaintSource::FlaskRequest(_)
        ));
    }

    #[test]
    fn test_sys_argv() {
        let expr = parse_expr("sys.argv");
        let taint = check_taint_source(&expr);
        assert!(taint.is_some());
        assert!(matches!(taint.unwrap().source, TaintSource::CommandLine));
    }
}
