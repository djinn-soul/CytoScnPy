#![allow(clippy::panic)]

use lsp_types::{DiagnosticSeverity, DiagnosticTag, NumberOrString, Position, Range};

use super::{entries, suppression_code};
use crate::report::Report;

fn report(json: &str) -> Report {
    serde_json::from_str(json).unwrap_or_default()
}

fn code(entry: &super::Entry) -> Option<&NumberOrString> {
    entry.diagnostic.code.as_ref()
}

#[test]
fn unused_definitions_cover_their_name_and_carry_fixes() {
    let text = "import os\n\ndef  unused():\n    return 1\n";
    let found = entries(
        &report(
            r#"{"unused_functions": [{"line": 3, "simple_name": "unused",
                "start_byte": 16, "def_type": "function",
                "message": "'unused' is defined but never used",
                "fix": {"start_byte": 11, "end_byte": 39, "replacement": ""}}]}"#,
        ),
        text,
    );
    let [entry] = found.as_slice() else {
        panic!("expected one diagnostic, got {found:?}");
    };
    assert_eq!(
        entry.diagnostic.range,
        Range::new(Position::new(2, 5), Position::new(2, 11))
    );
    assert_eq!(entry.diagnostic.severity, Some(DiagnosticSeverity::WARNING));
    assert_eq!(
        entry.diagnostic.tags,
        Some(vec![DiagnosticTag::UNNECESSARY])
    );
    assert_eq!(
        code(entry),
        Some(&NumberOrString::String("unused-function".into()))
    );
    let (title, fix) = entry.fix.as_ref().map(|(t, f)| (t.as_str(), f)).unzip();
    assert_eq!(title, Some("Remove unused function 'unused'"));
    assert_eq!(fix.map(|f| f.end_byte), Some(39));
}

#[test]
fn stale_offsets_fall_back_to_the_line_and_drop_the_fix() {
    let text = "x = 1\n";
    let found = entries(
        &report(
            r#"{"unused_variables": [{"line": 1, "simple_name": "x",
                "start_byte": 3, "def_type": "variable",
                "fix": {"start_byte": 0, "end_byte": 99, "replacement": "_"}}]}"#,
        ),
        text,
    );
    assert_eq!(
        found[0].diagnostic.range,
        Range::new(Position::new(0, 0), Position::new(0, 5))
    );
    assert!(found[0].fix.is_none());
    assert_eq!(found[0].diagnostic.message, "'x' is defined but never used");
}

#[test]
fn variables_are_renamed_rather_than_removed() {
    let found = entries(
        &report(
            r#"{"unused_variables": [{"line": 1, "simple_name": "x",
                "start_byte": 0, "def_type": "variable",
                "fix": {"start_byte": 0, "end_byte": 1, "replacement": "_"}}]}"#,
        ),
        "x = f()\n",
    );
    assert_eq!(
        found[0].fix.as_ref().map(|(title, _)| title.as_str()),
        Some("Rename unused variable 'x' to '_'")
    );
}

#[test]
fn taint_replaces_the_matching_danger_finding() {
    let text = "import os\ncmd = input()\nos.system(cmd)\neval(cmd)\n";
    let found = entries(
        &report(
            r#"{"danger": [
                {"rule_id": "CSP-D003", "message": "Command injection", "severity": "CRITICAL", "line": 3},
                {"rule_id": "CSP-D001", "message": "Avoid using eval", "severity": "HIGH", "line": 4}],
               "quality": [{"rule_id": "CSP-Q302", "message": "Deep", "severity": "LOW", "line": 2}],
               "taint_findings": [{"rule_id": "CSP-D003", "severity": "Critical",
                "vuln_type": "CommandInjection", "source": "input()", "source_line": 2,
                "sink": "os.system", "sink_line": 3, "flow_path": [],
                "remediation": "Use subprocess.run()."}]}"#,
        ),
        text,
    );
    let messages: Vec<_> = found
        .iter()
        .map(|e| e.diagnostic.message.lines().next())
        .collect();
    assert_eq!(
        messages,
        [
            Some("Avoid using eval"),
            Some("Deep"),
            Some("CommandInjection: Tainted data from input() (line 2) reaches sink os.system."),
        ]
    );
    let severities: Vec<_> = found.iter().map(|e| e.diagnostic.severity).collect();
    assert_eq!(
        severities,
        [
            Some(DiagnosticSeverity::ERROR),
            Some(DiagnosticSeverity::WARNING),
            Some(DiagnosticSeverity::ERROR),
        ]
    );
    assert_eq!(found[2].diagnostic.range.start, Position::new(2, 0));
}

#[test]
fn parse_errors_point_at_their_line() {
    let found = entries(
        &report(r#"{"parse_errors": [{"file": "a.py", "error": "Expected ':' at line 2"}]}"#),
        "def f()\n  pass\n",
    );
    assert_eq!(found[0].diagnostic.range.start, Position::new(1, 2));
    assert_eq!(
        found[0].diagnostic.message,
        "Parse error: Expected ':' at line 2"
    );
}

#[test]
fn noqa_names_rule_codes_and_falls_back_to_csp() {
    let code = |code: &str| suppression_code(code);
    assert_eq!(code("CSP-D001").as_deref(), Some("CSP-D001"));
    assert_eq!(code("csp-q302").as_deref(), Some("csp-q302"));
    assert_eq!(code("unused-import").as_deref(), Some("unused-import"));
    assert_eq!(code("clone-detected").as_deref(), Some("CSP"));
    assert_eq!(code("CSP-D1").as_deref(), Some("CSP"));
    assert_eq!(code("parse-error"), None);
}
