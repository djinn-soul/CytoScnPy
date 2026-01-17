//! Checks for attribute-based taint sources.

use crate::taint::types::{TaintInfo, TaintSource};
use crate::utils::LineIndex;
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

/// Checks if an attribute expression is a taint source.
pub(crate) fn check_attribute_source(
    attr: &ast::ExprAttribute,
    line_index: &LineIndex,
) -> Option<TaintInfo> {
    let line = line_index.line_index(attr.range().start());
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

        // Azure Functions 'req' object (conventional parameter name)
        if name.id.as_str() == "req" {
            match attr_name {
                "params" | "route_params" | "headers" | "form" => {
                    return Some(TaintInfo::new(
                        TaintSource::AzureFunctionsRequest(attr_name.to_owned()),
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
