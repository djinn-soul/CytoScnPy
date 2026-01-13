//! Checks for subscript-based taint sources.

use crate::taint::types::{TaintInfo, TaintSource};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

/// Checks if a subscript expression is a taint source.
pub(crate) fn check_subscript_source(sub: &ast::ExprSubscript) -> Option<TaintInfo> {
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
