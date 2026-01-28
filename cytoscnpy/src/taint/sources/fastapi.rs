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

#[cfg(test)]
mod tests {
    use super::*;
    use ruff_python_parser::parse_module;

    #[test]
    fn test_fastapi_params_detected() {
        let source = "
def api_handler(
    q: str = Query(None),
    p: int = Path(1),
    b: dict = Body(...),
    f: str = Form(),
    h: str = Header(None),
    c: str = Cookie(None),
    ignored: str = SomethingElse()
):
    pass
";
        let parsed = parse_module(source).expect("Failed to parse");
        let body = parsed.into_syntax().body;
        let func_def = body[0].as_function_def_stmt().expect("Not a function");

        let findings = check_fastapi_param(func_def);

        assert_eq!(findings.len(), 6);

        let names: Vec<&str> = findings.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"q"));
        assert!(names.contains(&"p"));
        assert!(names.contains(&"b"));
        assert!(names.contains(&"f"));
        assert!(names.contains(&"h"));
        assert!(names.contains(&"c"));
        assert!(!names.contains(&"ignored"));
    }

    #[test]
    fn test_fastapi_no_defaults() {
        let source = "
def basic_func(a, b: int):
    pass
";
        let parsed = parse_module(source).expect("Failed to parse");
        let body = parsed.into_syntax().body;
        let func_def = body[0].as_function_def_stmt().expect("Not a function");

        let findings = check_fastapi_param(func_def);
        assert!(findings.is_empty());
    }
}
