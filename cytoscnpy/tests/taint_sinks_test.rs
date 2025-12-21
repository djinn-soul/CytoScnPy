//! Tests for taint sink detection.
//!
//! Checks that security sinks (eval, exec, SQL execution, etc.) are correctly identified.

#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]
use cytoscnpy::taint::sinks::check_sink;
use cytoscnpy::taint::types::VulnType;
use ruff_python_ast::{self as ast, Expr};
use ruff_python_parser::{parse, Mode};

fn parse_call(source: &str) -> ast::ExprCall {
    let tree = parse(source, Mode::Expression.into()).unwrap();
    if let ast::Mod::Expression(expr) = tree.into_syntax() {
        if let Expr::Call(call) = *expr.body {
            return call;
        }
    }
    panic!("Expected call expression")
}

#[test]
fn test_eval_sink() {
    let call = parse_call("eval(x)");
    let sink = check_sink(&call);
    assert!(sink.is_some());
    assert!(matches!(sink.unwrap().vuln_type, VulnType::CodeInjection));
}

#[test]
fn test_execute_sink() {
    let call = parse_call("cursor.execute(query)");
    let sink = check_sink(&call);
    assert!(sink.is_some());
    assert!(matches!(sink.unwrap().vuln_type, VulnType::SqlInjection));
}

#[test]
fn test_subprocess_shell_true() {
    let call = parse_call("subprocess.run(cmd, shell=True)");
    let sink = check_sink(&call);
    assert!(sink.is_some());
    assert!(matches!(
        sink.unwrap().vuln_type,
        VulnType::CommandInjection
    ));
}
