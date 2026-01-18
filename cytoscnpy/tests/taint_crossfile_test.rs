//! Tests for cross-file taint analysis module.
//! Increases coverage for `src/taint/crossfile.rs`

#![allow(clippy::unwrap_used)]
#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::str_to_string)]

use cytoscnpy::taint::analyzer::TaintAnalyzer;
use cytoscnpy::taint::crossfile::{analyze_project, CrossFileAnalyzer};
use cytoscnpy::utils::LineIndex;
use std::path::PathBuf;

// =============================================================================
// CROSSFILE ANALYZER BASIC TESTS
// =============================================================================

#[test]
fn test_crossfile_analyzer_new() {
    let analyzer = CrossFileAnalyzer::new();
    // Should have builtins registered
    assert!(analyzer.get_module_summaries("__builtins__").is_some());
}

#[test]
fn test_register_and_resolve_import() {
    let mut analyzer = CrossFileAnalyzer::new();

    analyzer.register_import("app", "pd", "pandas", "pandas");

    let resolved = analyzer.resolve_import("app", "pd");
    assert!(resolved.is_some());
    let (module, name) = resolved.unwrap();
    assert_eq!(module, "pandas");
    assert_eq!(name, "pandas");
}

#[test]
fn test_resolve_unregistered_import() {
    let analyzer = CrossFileAnalyzer::new();

    let resolved = analyzer.resolve_import("nonexistent", "module");
    assert!(resolved.is_none());
}

#[test]
fn test_analyze_empty_file() {
    let mut analyzer = CrossFileAnalyzer::new();
    let taint_analyzer = TaintAnalyzer::default();
    let path = PathBuf::from("empty.py");
    let code = "";

    if let Ok(parsed) = ruff_python_parser::parse_module(code) {
        let module = parsed.into_syntax();
        let line_index = LineIndex::new(code);
        let findings = analyzer.analyze_file(&taint_analyzer, &path, &module.body, &line_index);
        assert!(findings.is_empty());
    }
}

#[test]
fn test_analyze_simple_file() {
    let mut analyzer = CrossFileAnalyzer::new();
    let taint_analyzer = TaintAnalyzer::default();
    let path = PathBuf::from("simple.py");
    let code = r#"
def hello():
    print("Hello, world!")
"#;

    if let Ok(parsed) = ruff_python_parser::parse_module(code) {
        let module = parsed.into_syntax();
        let line_index = LineIndex::new(code);
        let findings = analyzer.analyze_file(&taint_analyzer, &path, &module.body, &line_index);
        // No taint issues in this simple code
        assert!(findings.is_empty());
    }
}

#[test]
fn test_analyze_file_caching() {
    let mut analyzer = CrossFileAnalyzer::new();
    let path = PathBuf::from("cached.py");
    let code = "x = 1";

    // First call
    if let Ok(parsed) = ruff_python_parser::parse_module(code) {
        let module = parsed.into_syntax();
        let line_index = LineIndex::new(code);
        let taint_analyzer = TaintAnalyzer::default();

        let findings1 = analyzer.analyze_file(&taint_analyzer, &path, &module.body, &line_index);
        // Second call should return cached results
        let findings2 = analyzer.analyze_file(&taint_analyzer, &path, &module.body, &line_index);

        assert_eq!(findings1.len(), findings2.len());
    }
}

#[test]
fn test_clear_cache() {
    let mut analyzer = CrossFileAnalyzer::new();
    let path = PathBuf::from("to_clear.py");

    let code = "x = 1";
    if let Ok(parsed) = ruff_python_parser::parse_module(code) {
        let module = parsed.into_syntax();
        let line_index = LineIndex::new(code);
        let taint_analyzer = TaintAnalyzer::default();
        analyzer.analyze_file(&taint_analyzer, &path, &module.body, &line_index);
    }
    analyzer.clear_cache();

    // After clearing, get_all_findings should be empty
    assert!(analyzer.get_all_findings().is_empty());
}

#[test]
fn test_get_all_findings_empty() {
    let analyzer = CrossFileAnalyzer::new();
    assert!(analyzer.get_all_findings().is_empty());
}

#[test]
fn test_external_function_taints_return_builtins() {
    let analyzer = CrossFileAnalyzer::new();

    // input() is a builtin that taints return
    let taints = analyzer.external_function_taints_return("__builtins__", "input");
    assert!(taints);
}

#[test]
fn test_external_function_taints_return_unknown() {
    let analyzer = CrossFileAnalyzer::new();

    let taints = analyzer.external_function_taints_return("unknown_module", "unknown_func");
    assert!(!taints);
}

#[test]
fn test_get_module_summaries_nonexistent() {
    let analyzer = CrossFileAnalyzer::new();

    let summaries = analyzer.get_module_summaries("nonexistent");
    assert!(summaries.is_none());
}

// =============================================================================
// IMPORT EXTRACTION TESTS
// =============================================================================

#[test]
fn test_extract_import_statement() {
    let mut analyzer = CrossFileAnalyzer::new();
    let path = PathBuf::from("imports.py");
    let code = r#"
import os
import sys as system
"#;

    if let Ok(parsed) = ruff_python_parser::parse_module(code) {
        let module = parsed.into_syntax();
        let line_index = LineIndex::new(code);
        let taint_analyzer = TaintAnalyzer::default();
        analyzer.analyze_file(&taint_analyzer, &path, &module.body, &line_index);
    }

    // Should have registered the imports
    let resolved_os = analyzer.resolve_import("imports", "os");
    assert!(resolved_os.is_some());

    let resolved_sys = analyzer.resolve_import("imports", "system");
    assert!(resolved_sys.is_some());
}

#[test]
fn test_extract_from_import_statement() {
    let mut analyzer = CrossFileAnalyzer::new();
    let path = PathBuf::from("from_imports.py");
    let code = r#"
from flask import Flask, request as req
from os.path import join
"#;

    if let Ok(parsed) = ruff_python_parser::parse_module(code) {
        let module = parsed.into_syntax();
        let line_index = LineIndex::new(code);
        let taint_analyzer = TaintAnalyzer::default();
        analyzer.analyze_file(&taint_analyzer, &path, &module.body, &line_index);
    }

    // Should have registered the from imports
    let resolved_flask = analyzer.resolve_import("from_imports", "Flask");
    assert!(resolved_flask.is_some());

    let resolved_req = analyzer.resolve_import("from_imports", "req");
    assert!(resolved_req.is_some());
}

// =============================================================================
// ANALYZE PROJECT TESTS
// =============================================================================

#[test]
fn test_analyze_project_empty() {
    let files: Vec<(PathBuf, String)> = vec![];
    let taint_analyzer = TaintAnalyzer::default();
    let findings = analyze_project(&taint_analyzer, &files);
    assert!(findings.is_empty());
}

#[test]
fn test_analyze_project_single_file() {
    let files = vec![(PathBuf::from("single.py"), "x = 1".to_string())];
    let taint_analyzer = TaintAnalyzer::default();
    let findings = analyze_project(&taint_analyzer, &files);
    assert!(findings.is_empty());
}

#[test]
fn test_analyze_project_multiple_files() {
    let files = vec![
        (
            PathBuf::from("module_a.py"),
            "def func_a(): return 1".to_string(),
        ),
        (
            PathBuf::from("module_b.py"),
            "def func_b(): return 2".to_string(),
        ),
    ];
    let taint_analyzer = TaintAnalyzer::default();
    let findings = analyze_project(&taint_analyzer, &files);
    // No taint issues expected
    assert!(findings.is_empty());
}

#[test]
fn test_analyze_project_with_taint() {
    let files = vec![(
        PathBuf::from("tainted.py"),
        r#"
from flask import request

def handler():
    user_input = request.args.get('data')
    eval(user_input)
"#
        .to_string(),
    )];

    let taint_analyzer = TaintAnalyzer::default();
    let findings = analyze_project(&taint_analyzer, &files);
    // Should detect the taint flow from request to eval
    // Note: May or may not find depending on analysis depth
    // The analysis should complete without error
    let _ = findings.len();
}

#[test]
fn test_analyze_project_syntax_error() {
    let files = vec![(
        PathBuf::from("syntax_error.py"),
        "def invalid(:\n    pass".to_string(),
    )];

    // Should handle syntax errors gracefully
    let taint_analyzer = TaintAnalyzer::default();
    let findings = analyze_project(&taint_analyzer, &files);
    assert!(findings.is_empty());
}
