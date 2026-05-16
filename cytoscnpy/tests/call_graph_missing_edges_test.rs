//! Regression tests for call-graph edges that were previously missed by
//! `visit_stmt` / `visit_expr_for_calls`. Each test demonstrates a Python
//! construct whose inner function calls must be tracked, otherwise downstream
//! reachability analysis falsely marks the callee as unreachable.

use cytoscnpy::taint::call_graph::CallGraph;
use ruff_python_parser::parse_module;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn build_cg(source: &str) -> Result<CallGraph, Box<dyn std::error::Error>> {
    let parsed = parse_module(source).map_err(|e| e.to_string())?;
    let module = parsed.into_syntax();
    let mut cg = CallGraph::new();
    cg.build_from_module(&module.body, "m");
    Ok(cg)
}

#[test]
fn elif_condition_call_is_tracked() -> TestResult {
    let cg = build_cg(
        "
def cond():
    return True

def caller():
    if False:
        pass
    elif cond():
        pass
",
    )?;
    let node = cg.nodes.get("m.caller").ok_or("caller node missing")?;
    assert!(
        node.calls.contains("m.cond"),
        "elif test must register `cond` as a call from `caller`; got: {:?}",
        node.calls
    );
    Ok(())
}

#[test]
fn while_else_body_call_is_tracked() -> TestResult {
    let cg = build_cg(
        "
def helper():
    return 42

def caller():
    while False:
        pass
    else:
        helper()
",
    )?;
    let node = cg.nodes.get("m.caller").ok_or("caller node missing")?;
    assert!(
        node.calls.contains("m.helper"),
        "while...else body must register `helper` as a call; got: {:?}",
        node.calls
    );
    Ok(())
}

#[test]
fn except_handler_type_call_is_tracked() -> TestResult {
    let cg = build_cg(
        "
def make_exc():
    return ValueError

def caller():
    try:
        pass
    except (make_exc(),):
        pass
",
    )?;
    let node = cg.nodes.get("m.caller").ok_or("caller node missing")?;
    assert!(
        node.calls.contains("m.make_exc"),
        "except type expression must register `make_exc`; got: {:?}",
        node.calls
    );
    Ok(())
}

#[test]
fn assign_target_subscript_call_is_tracked() -> TestResult {
    let cg = build_cg(
        "
def get_key():
    return 'k'

def caller():
    d = {}
    d[get_key()] = 1
",
    )?;
    let node = cg.nodes.get("m.caller").ok_or("caller node missing")?;
    assert!(
        node.calls.contains("m.get_key"),
        "assign target subscript must register `get_key`; got: {:?}",
        node.calls
    );
    Ok(())
}

#[test]
fn aug_assign_target_subscript_call_is_tracked() -> TestResult {
    let cg = build_cg(
        "
def get_key():
    return 'k'

def caller():
    d = {'k': 0}
    d[get_key()] += 1
",
    )?;
    let node = cg.nodes.get("m.caller").ok_or("caller node missing")?;
    assert!(
        node.calls.contains("m.get_key"),
        "aug-assign target subscript must register `get_key`; got: {:?}",
        node.calls
    );
    Ok(())
}
