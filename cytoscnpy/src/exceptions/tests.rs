//! Unit tests for the exceptions (bare-except / empty-handler) scanner.
#![allow(clippy::unwrap_used)]

use std::path::Path;

use crate::exceptions::scanner::detect_python_exceptions;
use crate::exceptions::types::ExceptionKind;

fn path() -> &'static Path {
    Path::new("test_file.py")
}

// ── Bare-except detection ────────────────────────────────────────────────

#[test]
fn test_bare_except_detected() {
    let src = "try:\n    pass\nexcept:\n    print('oops')\n";
    let matches = detect_python_exceptions(src, path());
    assert!(!matches.is_empty(), "should detect bare except");
    assert_eq!(matches[0].kind, ExceptionKind::BareExcept);
    assert_eq!(matches[0].line, 3);
}

#[test]
fn test_typed_except_not_flagged_as_bare() {
    let src = "try:\n    x = int('bad')\nexcept ValueError:\n    pass\n";
    let matches = detect_python_exceptions(src, path());
    // should NOT have BareExcept; may have EmptyHandler (pass body)
    assert!(
        matches.iter().all(|m| m.kind != ExceptionKind::BareExcept),
        "typed except must not be flagged as bare"
    );
}

#[test]
fn test_except_exception_not_flagged_as_bare() {
    let src = "try:\n    risky()\nexcept Exception:\n    pass\n";
    let matches = detect_python_exceptions(src, path());
    assert!(
        matches.iter().all(|m| m.kind != ExceptionKind::BareExcept),
        "except Exception is not bare-except"
    );
}

// ── Empty-handler detection ──────────────────────────────────────────────

#[test]
fn test_pass_only_body_is_empty_handler() {
    let src = "try:\n    x = 1\nexcept ValueError:\n    pass\n";
    let matches = detect_python_exceptions(src, path());
    assert!(
        matches
            .iter()
            .any(|m| m.kind == ExceptionKind::EmptyHandler),
        "pass-only body should be empty handler"
    );
}

#[test]
fn test_ellipsis_body_is_empty_handler() {
    let src = "try:\n    y = 2\nexcept TypeError:\n    ...\n";
    let matches = detect_python_exceptions(src, path());
    assert!(
        matches
            .iter()
            .any(|m| m.kind == ExceptionKind::EmptyHandler),
        "ellipsis body should be empty handler"
    );
}

#[test]
fn test_string_literal_body_is_empty_handler() {
    let src = "try:\n    z = 3\nexcept OSError:\n    \"intentionally ignored\"\n";
    let matches = detect_python_exceptions(src, path());
    assert!(
        matches
            .iter()
            .any(|m| m.kind == ExceptionKind::EmptyHandler),
        "bare string body should be empty handler"
    );
}

#[test]
fn test_handler_with_logging_is_not_empty() {
    let src = "try:\n    risky()\nexcept RuntimeError as e:\n    logger.error(e)\n";
    let matches = detect_python_exceptions(src, path());
    assert!(
        matches
            .iter()
            .all(|m| m.kind != ExceptionKind::EmptyHandler),
        "handler with logging must not be flagged empty"
    );
}

#[test]
fn test_handler_with_reraise_is_not_empty() {
    let src = "try:\n    risky()\nexcept Exception:\n    raise\n";
    let matches = detect_python_exceptions(src, path());
    assert!(
        matches
            .iter()
            .all(|m| m.kind != ExceptionKind::EmptyHandler),
        "handler with re-raise must not be flagged empty"
    );
}

// ── Nesting and recursion ────────────────────────────────────────────────

#[test]
fn test_nested_try_in_function() {
    let src = "def process():\n    try:\n        do_work()\n    except:\n        pass\n";
    let matches = detect_python_exceptions(src, path());
    let bare = matches
        .iter()
        .filter(|m| m.kind == ExceptionKind::BareExcept)
        .count();
    let empty = matches
        .iter()
        .filter(|m| m.kind == ExceptionKind::EmptyHandler)
        .count();
    assert_eq!(bare, 1, "one bare-except inside function");
    assert_eq!(
        empty, 0,
        "bare-except body is not separately counted as empty handler"
    );
}

#[test]
fn test_nested_try_in_class_method() {
    let src =
        "class Foo:\n    def run(self):\n        try:\n            work()\n        except:\n            ...\n";
    let matches = detect_python_exceptions(src, path());
    assert!(
        matches.iter().any(|m| m.kind == ExceptionKind::BareExcept),
        "bare-except in class method should be detected"
    );
    assert!(
        matches
            .iter()
            .all(|m| m.kind != ExceptionKind::EmptyHandler),
        "bare-except body must not be double-counted as empty handler"
    );
}

#[test]
fn test_clean_source_returns_empty() {
    let src = "def add(a, b):\n    return a + b\n";
    let matches = detect_python_exceptions(src, path());
    assert!(matches.is_empty(), "clean code should have no matches");
}

#[test]
fn test_multiple_handlers_mixed() {
    let src = concat!(
        "try:\n",
        "    risky()\n",
        "except ValueError:\n",
        "    handle()\n",
        "except TypeError:\n",
        "    pass\n",
        "except:\n",
        "    ...\n",
    );
    let matches = detect_python_exceptions(src, path());
    let bare = matches
        .iter()
        .filter(|m| m.kind == ExceptionKind::BareExcept)
        .count();
    let empty = matches
        .iter()
        .filter(|m| m.kind == ExceptionKind::EmptyHandler)
        .count();
    // TypeError: pass → EmptyHandler(1); bare except: ... → BareExcept(1) only, not double-counted.
    assert_eq!(bare, 1);
    assert_eq!(
        empty, 1,
        "only the typed pass handler is empty; bare-except is not double-counted"
    );
}
