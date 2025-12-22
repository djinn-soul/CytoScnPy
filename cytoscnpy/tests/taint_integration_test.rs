//! Taint Analysis Integration Tests
//!
//! End-to-end tests verifying the taint analysis pipeline works correctly.
//! These tests verify the pipeline doesn't crash on basic code patterns.
#![allow(clippy::needless_raw_string_hashes)]

use cytoscnpy::taint::analyzer::{TaintAnalyzer, TaintConfig};
use std::path::PathBuf;

fn analyze_code(source: &str) -> Vec<cytoscnpy::taint::types::TaintFinding> {
    let config = TaintConfig::all_levels();
    let analyzer = TaintAnalyzer::new(config);
    let path = PathBuf::from("test.py");
    analyzer.analyze_file(source, &path)
}

// ============================================================================
// Basic Integration Tests
// ============================================================================

#[test]
fn test_analyze_empty_runs() {
    let _findings = analyze_code("");
}

#[test]
fn test_analyze_simple_function_runs() {
    let _findings = analyze_code(
        r#"
def hello():
    print("Hello, World!")
"#,
    );
}

#[test]
fn test_analyze_function_with_args_runs() {
    let _findings = analyze_code(
        r#"
def greet(name):
    return f"Hello, {name}"
"#,
    );
}

#[test]
fn test_analyze_simple_class_runs() {
    let _findings = analyze_code(
        r#"
class Greeter:
    def greet(self, name):
        return f"Hello, {name}"
"#,
    );
}

#[test]
fn test_analyze_imports_runs() {
    let _findings = analyze_code(
        r#"
import os
import sys
from pathlib import Path

def get_path():
    return Path.cwd()
"#,
    );
}

#[test]
fn test_analyze_control_flow_runs() {
    let _findings = analyze_code(
        r#"
def check(x):
    if x > 0:
        return "positive"
    elif x < 0:
        return "negative"
    else:
        return "zero"
"#,
    );
}

#[test]
fn test_analyze_loop_runs() {
    let _findings = analyze_code(
        r#"
def iterate():
    result = []
    for i in range(10):
        result.append(i * 2)
    return result
"#,
    );
}

#[test]
fn test_analyze_try_except_runs() {
    let _findings = analyze_code(
        r#"
def safe_divide(a, b):
    try:
        return a / b
    except ZeroDivisionError:
        return None
"#,
    );
}

#[test]
fn test_analyze_literal_eval_runs() {
    let _findings = analyze_code(
        r#"
result = eval("2 + 2")
"#,
    );
}

#[test]
fn test_analyze_literal_sql_runs() {
    let _findings = analyze_code(
        r#"
import sqlite3
conn = sqlite3.connect(':memory:')
conn.execute("SELECT 1")
"#,
    );
}
