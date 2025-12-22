//! Taint Analysis Feature Tests
//!
//! Feature tests for taint analysis
#![allow(clippy::needless_raw_string_hashes)]
//! Tests for taint flow detection from sources to sinks.

use cytoscnpy::taint::analyzer::{TaintAnalyzer, TaintConfig};
use std::path::PathBuf;

fn analyze_code(source: &str) -> Vec<cytoscnpy::taint::types::TaintFinding> {
    let config = TaintConfig::all_levels();
    let analyzer = TaintAnalyzer::new(config);
    let path = PathBuf::from("test.py");
    analyzer.analyze_file(source, &path)
}

// ============================================================================
// Flask Source Detection
// ============================================================================

#[test]
fn test_flask_request_args_to_eval() {
    let findings = analyze_code(
        r#"
from flask import request

@app.route('/search')
def search():
    query = request.args.get('q')
    result = eval(query)
    return result
"#,
    );
    assert!(
        !findings.is_empty(),
        "Should detect taint from request.args to eval"
    );
}

#[test]
fn test_flask_request_form_to_subprocess() {
    let findings = analyze_code(
        r#"
from flask import request
import subprocess

@app.route('/exec', methods=['POST'])
def execute():
    cmd = request.form['command']
    subprocess.run(cmd, shell=True)
"#,
    );
    assert!(
        !findings.is_empty(),
        "Should detect taint from request.form to subprocess"
    );
}

// ============================================================================
// Django Source Detection
// ============================================================================

#[test]
fn test_django_request_get_to_eval() {
    let findings = analyze_code(
        r#"
def search_view(request):
    query = request.GET.get('q')
    eval(query)
"#,
    );
    assert!(
        !findings.is_empty(),
        "Should detect taint from request.GET to eval"
    );
}

#[test]
fn test_django_request_post_to_os_system() {
    let findings = analyze_code(
        r#"
import os

def upload_view(request):
    filename = request.POST['filename']
    os.system(f"mv upload.tmp {filename}")
"#,
    );
    assert!(
        !findings.is_empty(),
        "Should detect command injection from request.POST"
    );
}

// ============================================================================
// Builtin Source Detection
// ============================================================================

#[test]
fn test_input_to_eval() {
    let findings = analyze_code(
        r#"
user_input = input("Enter expression: ")
result = eval(user_input)
"#,
    );
    assert!(
        !findings.is_empty(),
        "Should detect taint from input() to eval"
    );
}

#[test]
fn test_environ_to_subprocess() {
    let findings = analyze_code(
        r#"
import os
import subprocess

cmd = os.environ.get('CMD')
subprocess.run(cmd, shell=True)
"#,
    );
    assert!(
        !findings.is_empty(),
        "Should detect taint from environ to subprocess"
    );
}

#[test]
fn test_sys_argv_to_exec() {
    let findings = analyze_code(
        r#"
import sys

code = sys.argv[1]
exec(code)
"#,
    );
    assert!(
        !findings.is_empty(),
        "Should detect taint from sys.argv to exec"
    );
}

// ============================================================================
// Propagation Detection
// ============================================================================

#[test]
fn test_assignment_propagation() {
    let findings = analyze_code(
        r#"
user_input = input()
x = user_input
y = x
z = y
eval(z)
"#,
    );
    assert!(
        !findings.is_empty(),
        "Taint should propagate through assignments"
    );
}

#[test]
fn test_string_concat_propagation() {
    let findings = analyze_code(
        r#"
user_input = input()
cmd = "echo " + user_input
os.system(cmd)
"#,
    );
    assert!(
        !findings.is_empty(),
        "Taint should propagate through string concatenation"
    );
}

#[test]
fn test_fstring_propagation() {
    let findings = analyze_code(
        r#"
user_input = input()
query = f"SELECT * FROM users WHERE id = {user_input}"
cursor.execute(query)
"#,
    );
    assert!(
        !findings.is_empty(),
        "Taint should propagate through f-strings"
    );
}

#[test]
fn test_conditional_flow() {
    let findings = analyze_code(
        r#"
user_input = input()
if True:
    x = user_input
else:
    x = "safe"
eval(x)
"#,
    );
    assert!(
        !findings.is_empty(),
        "Should detect taint through conditional branches"
    );
}

#[test]
fn test_loop_flow() {
    let findings = analyze_code(
        r#"
user_input = input()
result = ""
for char in user_input:
    result += char
eval(result)
"#,
    );
    assert!(
        !findings.is_empty(),
        "Should detect taint through loop iteration"
    );
}
