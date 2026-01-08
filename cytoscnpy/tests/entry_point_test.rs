//! Tests for entry point detection.
//!
//! Verifies that `if __name__ == "__main__":` blocks and similar patterns are correctly detected.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use cytoscnpy::entry_point::detect_entry_point_calls;
use ruff_python_parser::parse_module;

#[test]
fn test_entry_point_detection() {
    let source = r#"
def my_function():
    pass

if __name__ == "__main__":
    my_function()
    another_call()
"#;

    let parsed = parse_module(source).expect("Failed to parse");
    let module = parsed.into_syntax();
    let calls = detect_entry_point_calls(&module.body);

    assert!(
        calls.contains("my_function"),
        "Should detect my_function call"
    );
    assert!(calls.contains("another_call"), "Should detect another_call");
}

#[test]
fn test_no_entry_point() {
    let source = r"
def my_function():
    pass
";

    let parsed = parse_module(source).expect("Failed to parse");
    let module = parsed.into_syntax();
    let calls = detect_entry_point_calls(&module.body);
    assert_eq!(calls.len(), 0, "Should detect no entry point calls");
}

#[test]
fn test_reversed_main_guard() {
    let source = r#"
def func():
    pass

if "__main__" == __name__:
    func()
"#;

    let parsed = parse_module(source).expect("Failed to parse");
    let module = parsed.into_syntax();
    let calls = detect_entry_point_calls(&module.body);
    assert!(calls.contains("func"), "Should handle reversed comparison");
}
