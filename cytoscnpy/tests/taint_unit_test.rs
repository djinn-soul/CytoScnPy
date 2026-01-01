//! Taint Analysis Unit Tests
//!
//! Tests for core taint analysis mechanics.

use cytoscnpy::taint::analyzer::{TaintAnalyzer, TaintConfig};
use std::path::PathBuf;

fn analyze_code(source: &str) -> Vec<cytoscnpy::taint::types::TaintFinding> {
    let config = TaintConfig::all_levels();
    let analyzer = TaintAnalyzer::new(config);
    let path = PathBuf::from("test.py");
    analyzer.analyze_file(source, &path)
}

// ============================================================================
// Analyzer Configuration Tests
// ============================================================================

#[test]
fn test_analyzer_creation() {
    let config = TaintConfig::all_levels();
    let analyzer = TaintAnalyzer::new(config);
    assert!(analyzer.config.intraprocedural);
    assert!(analyzer.config.interprocedural);
    assert!(analyzer.config.crossfile);
}

#[test]
fn test_config_intraprocedural_only() {
    let config = TaintConfig::intraprocedural_only();
    assert!(config.intraprocedural);
    assert!(!config.interprocedural);
    assert!(!config.crossfile);
}

// ============================================================================
// Empty and Safe Code Tests
// ============================================================================

#[test]
fn test_empty_file() {
    let findings = analyze_code("");
    assert!(findings.is_empty(), "Empty file should have no findings");
}

#[test]
fn test_no_taint_sources() {
    let findings = analyze_code(
        r#"
def safe_function():
    x = 42
    y = "literal"
    return x + len(y)
"#,
    );
    assert!(
        findings.is_empty(),
        "File with no taint sources should have no findings"
    );
}

// ============================================================================
// False Positive Prevention Tests
// ============================================================================

#[test]
fn test_literal_eval_safe() {
    let findings = analyze_code(
        r#"
result = eval("2 + 2")
"#,
    );
    assert!(
        findings.is_empty(),
        "eval with literal should not be flagged"
    );
}

#[test]
fn test_constant_sql_safe() {
    let findings = analyze_code(
        r#"
cursor.execute("SELECT COUNT(*) FROM users")
"#,
    );
    assert!(
        findings.is_empty(),
        "SQL with constant query should not be flagged"
    );
}

#[test]
fn test_subprocess_literal_safe() {
    let findings = analyze_code(
        r#"
import subprocess
subprocess.run(["ls", "-la"])
"#,
    );
    assert!(
        findings.is_empty(),
        "subprocess with literal list should not be flagged"
    );
}

// ============================================================================
// Sanitizer Tests
// ============================================================================

#[test]
fn test_int_sanitizer() {
    let findings = analyze_code(
        r#"
def safe_query(user_input):
    user_id = int(user_input)
    cursor.execute(f"SELECT * FROM users WHERE id = {user_id}")
"#,
    );
    assert!(
        findings.is_empty(),
        "int() should sanitize taint for SQL injection"
    );
}

#[test]
fn test_parameterized_query_sanitizer() {
    let findings = analyze_code(
        r#"
import sqlite3

def safe_query(username):
    conn = sqlite3.connect('db.sqlite')
    cursor = conn.cursor()
    cursor.execute("SELECT * FROM users WHERE name = ?", (username,))
"#,
    );
    assert!(
        findings.is_empty(),
        "Parameterized queries should prevent SQL injection findings"
    );
}
