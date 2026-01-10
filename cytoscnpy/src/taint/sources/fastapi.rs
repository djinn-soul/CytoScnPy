//! `FastAPI` specific taint source detection.

use super::utils::get_call_name;
use crate::taint::types::{TaintInfo, TaintSource};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

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
