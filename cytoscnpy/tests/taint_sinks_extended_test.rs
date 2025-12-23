//! Extended tests for `taint/sinks.rs` - Dangerous sink detection.
#![allow(clippy::unwrap_used)]

use cytoscnpy::taint::sinks::{check_sink, SINK_PATTERNS};
use cytoscnpy::taint::types::{Severity, VulnType};
use ruff_python_ast as ast;
use ruff_python_parser::parse_expression;

/// Parse a call expression for testing.
fn parse_call(source: &str) -> ast::ExprCall {
    let expr = parse_expression(source).unwrap().into_expr();
    if let ast::Expr::Call(call) = expr {
        call
    } else {
        panic!("Expected call expression, got: {source}");
    }
}

// ============================================================================
// check_sink Tests - Code Execution
// ============================================================================

#[test]
fn test_sink_eval() {
    let call = parse_call("eval(code)");
    let result = check_sink(&call);
    assert!(result.is_some());
    let info = result.unwrap();
    assert_eq!(info.vuln_type, VulnType::CodeInjection);
    assert_eq!(info.severity, Severity::Critical);
}

#[test]
fn test_sink_exec() {
    let call = parse_call("exec(code)");
    let result = check_sink(&call);
    assert!(result.is_some());
    let info = result.unwrap();
    assert_eq!(info.vuln_type, VulnType::CodeInjection);
}

#[test]
fn test_sink_compile() {
    let call = parse_call("compile(code, '<string>', 'exec')");
    let result = check_sink(&call);
    assert!(result.is_some());
    let info = result.unwrap();
    assert_eq!(info.vuln_type, VulnType::CodeInjection);
}

// ============================================================================
// check_sink Tests - SQL Injection
// ============================================================================

#[test]
fn test_sink_execute() {
    let call = parse_call("cursor.execute(query)");
    let result = check_sink(&call);
    assert!(result.is_some());
    let info = result.unwrap();
    assert_eq!(info.vuln_type, VulnType::SqlInjection);
}

#[test]
fn test_sink_executemany() {
    let call = parse_call("cursor.executemany(query)");
    let result = check_sink(&call);
    assert!(result.is_some());
    let info = result.unwrap();
    assert_eq!(info.vuln_type, VulnType::SqlInjection);
}

#[test]
fn test_sink_objects_raw() {
    let call = parse_call("Model.objects.raw(query)");
    let result = check_sink(&call);
    assert!(result.is_some());
    let info = result.unwrap();
    assert_eq!(info.vuln_type, VulnType::SqlInjection);
}

// ============================================================================
// check_sink Tests - Command Injection
// ============================================================================

#[test]
fn test_sink_os_system() {
    let call = parse_call("os.system(cmd)");
    let result = check_sink(&call);
    assert!(result.is_some());
    let info = result.unwrap();
    assert_eq!(info.vuln_type, VulnType::CommandInjection);
}

#[test]
fn test_sink_os_popen() {
    let call = parse_call("os.popen(cmd)");
    let result = check_sink(&call);
    assert!(result.is_some());
    let info = result.unwrap();
    assert_eq!(info.vuln_type, VulnType::CommandInjection);
}

#[test]
fn test_sink_subprocess_call() {
    let call = parse_call("subprocess.call(cmd, shell=True)");
    let result = check_sink(&call);
    assert!(result.is_some());
}

#[test]
fn test_sink_subprocess_run() {
    let call = parse_call("subprocess.run(cmd, shell=True)");
    let result = check_sink(&call);
    assert!(result.is_some());
}

#[test]
fn test_sink_subprocess_popen() {
    let call = parse_call("subprocess.Popen(cmd, shell=True)");
    let result = check_sink(&call);
    assert!(result.is_some());
}

// ============================================================================
// check_sink Tests - XSS
// ============================================================================

#[test]
fn test_sink_render_template_string() {
    let call = parse_call("render_template_string(template)");
    let result = check_sink(&call);
    assert!(result.is_some());
    let info = result.unwrap();
    assert_eq!(info.vuln_type, VulnType::Xss);
}

#[test]
fn test_sink_markup() {
    let call = parse_call("Markup(html)");
    let result = check_sink(&call);
    assert!(result.is_some());
    let info = result.unwrap();
    assert_eq!(info.vuln_type, VulnType::Xss);
}

#[test]
fn test_sink_mark_safe() {
    let call = parse_call("mark_safe(html)");
    let result = check_sink(&call);
    assert!(result.is_some());
    let info = result.unwrap();
    assert_eq!(info.vuln_type, VulnType::Xss);
}

// ============================================================================
// check_sink Tests - Deserialization
// ============================================================================

#[test]
fn test_sink_pickle_loads() {
    let call = parse_call("pickle.loads(data)");
    let result = check_sink(&call);
    assert!(result.is_some());
    let info = result.unwrap();
    assert_eq!(info.vuln_type, VulnType::Deserialization);
}

#[test]
fn test_sink_pickle_load() {
    let call = parse_call("pickle.load(file)");
    let result = check_sink(&call);
    assert!(result.is_some());
    let info = result.unwrap();
    assert_eq!(info.vuln_type, VulnType::Deserialization);
}

#[test]
fn test_sink_yaml_load() {
    let call = parse_call("yaml.load(data)");
    let result = check_sink(&call);
    assert!(result.is_some());
    let info = result.unwrap();
    assert_eq!(info.vuln_type, VulnType::Deserialization);
}

// ============================================================================
// check_sink Tests - Path Traversal
// ============================================================================

#[test]
fn test_sink_open() {
    let call = parse_call("open(path)");
    let result = check_sink(&call);
    assert!(result.is_some());
    let info = result.unwrap();
    assert_eq!(info.vuln_type, VulnType::PathTraversal);
}

// ============================================================================
// check_sink Tests - SSRF
// ============================================================================

#[test]
fn test_sink_requests_get() {
    let call = parse_call("requests.get(url)");
    let result = check_sink(&call);
    assert!(result.is_some());
    let info = result.unwrap();
    assert_eq!(info.vuln_type, VulnType::Ssrf);
}

#[test]
fn test_sink_requests_post() {
    let call = parse_call("requests.post(url)");
    let result = check_sink(&call);
    assert!(result.is_some());
    let info = result.unwrap();
    assert_eq!(info.vuln_type, VulnType::Ssrf);
}

#[test]
fn test_sink_httpx_get() {
    let call = parse_call("httpx.get(url)");
    let result = check_sink(&call);
    assert!(result.is_some());
    let info = result.unwrap();
    assert_eq!(info.vuln_type, VulnType::Ssrf);
}

#[test]
fn test_sink_urlopen() {
    let call = parse_call("urlopen(url)");
    let result = check_sink(&call);
    assert!(result.is_some());
    let info = result.unwrap();
    assert_eq!(info.vuln_type, VulnType::Ssrf);
}

// ============================================================================
// check_sink Tests - Safe Calls
// ============================================================================

#[test]
fn test_no_sink_print() {
    let call = parse_call("print(x)");
    let result = check_sink(&call);
    assert!(result.is_none());
}

#[test]
fn test_no_sink_len() {
    let call = parse_call("len(x)");
    let result = check_sink(&call);
    assert!(result.is_none());
}

#[test]
fn test_no_sink_str() {
    let call = parse_call("str(x)");
    let result = check_sink(&call);
    assert!(result.is_none());
}

// ============================================================================
// SINK_PATTERNS Tests
// ============================================================================

#[test]
fn test_sink_patterns_contains_eval() {
    assert!(SINK_PATTERNS.contains(&"eval"));
}

#[test]
fn test_sink_patterns_contains_exec() {
    assert!(SINK_PATTERNS.contains(&"exec"));
}

#[test]
fn test_sink_patterns_contains_os_system() {
    assert!(SINK_PATTERNS.contains(&"os.system"));
}

#[test]
fn test_sink_patterns_contains_subprocess() {
    assert!(SINK_PATTERNS.contains(&"subprocess."));
}

#[test]
fn test_sink_patterns_contains_pickle() {
    assert!(SINK_PATTERNS.contains(&"pickle.loads"));
}
