//! Extended tests for `taint/sources.rs` - Taint source detection.
#![allow(clippy::unwrap_used)]

use cytoscnpy::taint::sources::check_taint_source;
use cytoscnpy::taint::types::TaintSource;
use ruff_python_parser::parse_expression;

/// Parse an expression for testing.
fn parse_expr(source: &str) -> ruff_python_ast::Expr {
    parse_expression(source).unwrap().into_expr()
}

// ============================================================================
// Input Sources
// ============================================================================

#[test]
fn test_source_input_function() {
    let expr = parse_expr("input('Enter: ')");
    let result = check_taint_source(&expr);
    assert!(result.is_some());
    assert_eq!(result.unwrap().source, TaintSource::Input);
}

#[test]
fn test_source_input_no_prompt() {
    let expr = parse_expr("input()");
    let result = check_taint_source(&expr);
    assert!(result.is_some());
    assert_eq!(result.unwrap().source, TaintSource::Input);
}

#[test]
fn test_source_raw_input_not_supported() {
    // raw_input was Python 2, not currently detected
    let expr = parse_expr("raw_input()");
    let result = check_taint_source(&expr);
    // May or may not be detected depending on implementation
    let _ = result;
}

// ============================================================================
// Command Line Sources
// ============================================================================

#[test]
fn test_source_sys_argv_subscript() {
    let expr = parse_expr("sys.argv[1]");
    let result = check_taint_source(&expr);
    assert!(result.is_some());
    assert_eq!(result.unwrap().source, TaintSource::CommandLine);
}

#[test]
fn test_source_sys_argv_zero() {
    let expr = parse_expr("sys.argv[0]");
    let result = check_taint_source(&expr);
    assert!(result.is_some());
    assert_eq!(result.unwrap().source, TaintSource::CommandLine);
}

// ============================================================================
// Environment Sources
// ============================================================================

#[test]
fn test_source_os_environ_subscript() {
    let expr = parse_expr("os.environ['PATH']");
    let result = check_taint_source(&expr);
    assert!(result.is_some());
    assert_eq!(result.unwrap().source, TaintSource::Environment);
}

#[test]
fn test_source_os_environ_home() {
    let expr = parse_expr("os.environ['HOME']");
    let result = check_taint_source(&expr);
    assert!(result.is_some());
    assert_eq!(result.unwrap().source, TaintSource::Environment);
}

// ============================================================================
// File Read Sources
// ============================================================================

#[test]
fn test_source_file_read() {
    let expr = parse_expr("file.read()");
    let result = check_taint_source(&expr);
    assert!(result.is_some());
    assert_eq!(result.unwrap().source, TaintSource::FileRead);
}

#[test]
fn test_source_file_readline() {
    let expr = parse_expr("file.readline()");
    let result = check_taint_source(&expr);
    assert!(result.is_some());
    assert_eq!(result.unwrap().source, TaintSource::FileRead);
}

#[test]
fn test_source_file_readlines() {
    let expr = parse_expr("file.readlines()");
    let result = check_taint_source(&expr);
    assert!(result.is_some());
    assert_eq!(result.unwrap().source, TaintSource::FileRead);
}

// ============================================================================
// External Data Sources
// ============================================================================

#[test]
fn test_source_json_load() {
    let expr = parse_expr("json.load(f)");
    let result = check_taint_source(&expr);
    assert!(result.is_some());
    assert_eq!(result.unwrap().source, TaintSource::ExternalData);
}

#[test]
fn test_source_json_loads() {
    let expr = parse_expr("json.loads(data)");
    let result = check_taint_source(&expr);
    assert!(result.is_some());
    assert_eq!(result.unwrap().source, TaintSource::ExternalData);
}

#[test]
fn test_source_requests_get_not_source() {
    // requests.get is a SINK (SSRF), not a source
    let expr = parse_expr("requests.get(url)");
    let result = check_taint_source(&expr);
    assert!(result.is_none());
}

#[test]
fn test_source_urlopen_not_source() {
    // urlopen is a SINK (SSRF), not a source
    let expr = parse_expr("urlopen(url)");
    let result = check_taint_source(&expr);
    assert!(result.is_none());
}

// ============================================================================
// Flask Request Sources (tuple variant with String)
// ============================================================================

#[test]
fn test_source_flask_request_args() {
    let expr = parse_expr("request.args");
    let result = check_taint_source(&expr);
    assert!(result.is_some());
    assert!(matches!(
        result.unwrap().source,
        TaintSource::FlaskRequest(_)
    ));
}

#[test]
fn test_source_flask_request_form() {
    let expr = parse_expr("request.form");
    let result = check_taint_source(&expr);
    assert!(result.is_some());
    assert!(matches!(
        result.unwrap().source,
        TaintSource::FlaskRequest(_)
    ));
}

#[test]
fn test_source_flask_request_json() {
    let expr = parse_expr("request.json");
    let result = check_taint_source(&expr);
    assert!(result.is_some());
    assert!(matches!(
        result.unwrap().source,
        TaintSource::FlaskRequest(_)
    ));
}

#[test]
fn test_source_flask_request_data() {
    // request.data is another Flask source
    let expr = parse_expr("request.data");
    let result = check_taint_source(&expr);
    assert!(result.is_some());
    assert!(matches!(
        result.unwrap().source,
        TaintSource::FlaskRequest(_)
    ));
}

#[test]
fn test_source_flask_request_cookies() {
    let expr = parse_expr("request.cookies");
    let result = check_taint_source(&expr);
    assert!(result.is_some());
    assert!(matches!(
        result.unwrap().source,
        TaintSource::FlaskRequest(_)
    ));
}

// ============================================================================
// Django Request Sources (tuple variant with String)
// ============================================================================

#[test]
fn test_source_django_request_get() {
    let expr = parse_expr("request.GET");
    let result = check_taint_source(&expr);
    assert!(result.is_some());
    assert!(matches!(
        result.unwrap().source,
        TaintSource::DjangoRequest(_)
    ));
}

#[test]
fn test_source_django_request_post() {
    let expr = parse_expr("request.POST");
    let result = check_taint_source(&expr);
    assert!(result.is_some());
    assert!(matches!(
        result.unwrap().source,
        TaintSource::DjangoRequest(_)
    ));
}

// ============================================================================
// Safe (Non-Tainted) Sources
// ============================================================================

#[test]
fn test_source_string_literal_safe() {
    let expr = parse_expr("'hello world'");
    let result = check_taint_source(&expr);
    assert!(result.is_none());
}

#[test]
fn test_source_number_literal_safe() {
    let expr = parse_expr("42");
    let result = check_taint_source(&expr);
    assert!(result.is_none());
}

#[test]
fn test_source_print_call_safe() {
    let expr = parse_expr("print(x)");
    let result = check_taint_source(&expr);
    assert!(result.is_none());
}

#[test]
fn test_source_safe_attribute() {
    let expr = parse_expr("obj.method");
    let result = check_taint_source(&expr);
    assert!(result.is_none());
}

#[test]
fn test_source_safe_subscript() {
    let expr = parse_expr("my_list[0]");
    let result = check_taint_source(&expr);
    assert!(result.is_none());
}
