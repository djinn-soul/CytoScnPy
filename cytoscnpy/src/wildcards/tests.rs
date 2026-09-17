//! Unit tests for the wildcard-import scanner.
#![allow(clippy::unwrap_used)]

use std::path::Path;

use crate::wildcards::scanner::detect_wildcard_imports;

fn path() -> &'static Path {
    Path::new("test_file.py")
}

// ── Basic detection ──────────────────────────────────────────────────────

#[test]
fn test_module_level_star_import_detected() {
    let src = "from os.path import *\n";
    let matches = detect_wildcard_imports(src, path());
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].module, "os.path");
    assert_eq!(matches[0].line, 1);
}

#[test]
fn test_multiple_star_imports_detected() {
    let src = "from os import *\nfrom sys import *\n";
    let matches = detect_wildcard_imports(src, path());
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].module, "os");
    assert_eq!(matches[1].module, "sys");
}

#[test]
fn test_named_import_not_flagged() {
    let src = "from os.path import join, exists\n";
    let matches = detect_wildcard_imports(src, path());
    assert!(matches.is_empty(), "named imports must not be flagged");
}

#[test]
fn test_plain_import_not_flagged() {
    let src = "import os\nimport sys\n";
    let matches = detect_wildcard_imports(src, path());
    assert!(matches.is_empty(), "plain imports must not be flagged");
}

#[test]
fn test_clean_source_returns_empty() {
    let src = "def add(a, b):\n    return a + b\n";
    let matches = detect_wildcard_imports(src, path());
    assert!(matches.is_empty(), "clean code should have no matches");
}

// ── Module name capture ──────────────────────────────────────────────────

#[test]
fn test_module_name_captured_correctly() {
    let src = "from typing import *\n";
    let matches = detect_wildcard_imports(src, path());
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].module, "typing");
}

#[test]
fn test_nested_module_name_captured() {
    let src = "from collections.abc import *\n";
    let matches = detect_wildcard_imports(src, path());
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].module, "collections.abc");
}

// ── Nested-scope detection ───────────────────────────────────────────────

#[test]
fn test_star_import_inside_function_detected() {
    let src = "def setup():\n    from glob import *\n";
    let matches = detect_wildcard_imports(src, path());
    assert_eq!(
        matches.len(),
        1,
        "star import inside function should be detected"
    );
    assert_eq!(matches[0].module, "glob");
}

#[test]
fn test_star_import_inside_class_detected() {
    let src = "class Mixin:\n    from mixin_base import *\n";
    let matches = detect_wildcard_imports(src, path());
    assert_eq!(
        matches.len(),
        1,
        "star import inside class should be detected"
    );
}

#[test]
fn test_star_import_inside_if_detected() {
    let src = "if True:\n    from compat import *\n";
    let matches = detect_wildcard_imports(src, path());
    assert_eq!(matches.len(), 1);
}

// ── Snippet content ──────────────────────────────────────────────────────

#[test]
fn test_snippet_contains_import_statement() {
    let src = "from pathlib import *\n";
    let matches = detect_wildcard_imports(src, path());
    assert!(
        !matches[0].snippet.is_empty(),
        "snippet should be populated"
    );
    assert!(
        matches[0].snippet.contains("import"),
        "snippet should contain the import keyword"
    );
}

// ── Mixed imports ────────────────────────────────────────────────────────

#[test]
fn test_mixed_imports_only_star_flagged() {
    let src = concat!(
        "import os\n",
        "from sys import argv\n",
        "from glob import *\n",
        "from pathlib import Path\n",
    );
    let matches = detect_wildcard_imports(src, path());
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].module, "glob");
}

#[test]
fn test_relative_star_imports_detected() {
    let src = "from . import *\nfrom ..sub import *\n";
    let matches = detect_wildcard_imports(src, path());
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].module, ".");
    assert_eq!(matches[1].module, "..sub");
}
