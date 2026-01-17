//! Tests for type inference method misuse detection.
#![allow(missing_docs)]

use cytoscnpy::analyzer::CytoScnPy;
use std::path::PathBuf;

fn analyze_code(code: &str) -> Vec<cytoscnpy::rules::Finding> {
    let analyzer = CytoScnPy::default()
        .with_danger(true)
        .with_quality(false)
        .with_secrets(false);

    let result = analyzer.analyze_code(code, &PathBuf::from("test.py"));
    result.danger
}

#[test]
fn test_str_misuse() {
    let code = r#"
s = "hello"
s.append("world")
"#;
    let findings = analyze_code(code);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "CSP-D601");
    assert!(findings[0]
        .message
        .contains("Method 'append' does not exist for inferred type 'str'"));
}

#[test]
fn test_list_misuse() {
    let code = r"
l = [1, 2]
l.strip()
";
    let findings = analyze_code(code);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "CSP-D601");
    assert!(findings[0]
        .message
        .contains("Method 'strip' does not exist for inferred type 'list'"));
}

#[test]
fn test_dict_misuse() {
    let code = r#"
d = {"a": 1}
d.add(2)
"#;
    let findings = analyze_code(code);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "CSP-D601");
    assert!(findings[0]
        .message
        .contains("Method 'add' does not exist for inferred type 'dict'"));
}

#[test]
fn test_scope_shadowing() {
    let code = r#"
x = "outer"
def func():
    x = []
    x.append(1) # Safe
x.append("fail") # Error
"#;
    let findings = analyze_code(code);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].line, 6); // 1-indexed: line 6 is x.append("fail")
    assert_eq!(findings[0].rule_id, "CSP-D601");
}

#[test]
fn test_reassignment() {
    let code = r#"
x = "str"
x.append(1) # Error 1
x = []
x.append(1) # Safe
"#;
    let findings = analyze_code(code);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "CSP-D601");
}

#[test]
fn test_comprehensions() {
    let code = r"
l = [x for x in range(10)]
l.strip() # Error
s = {x for x in range(10)}
s.append(1) # Error
d = {x: x for x in range(10)}
d.add(1) # Error
";
    let findings = analyze_code(code);
    assert_eq!(findings.len(), 3);
    for finding in findings {
        assert_eq!(finding.rule_id, "CSP-D601");
    }
}
