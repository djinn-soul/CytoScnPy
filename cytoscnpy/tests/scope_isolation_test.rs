//! Tests for scope isolation in type inference.

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
fn test_lambda_scope_isolation() {
    let code = r#"
x = "outer"
f = lambda x: x.append(1) # x inside lambda is 'unknown', should not error as 'str'
x.strip() # x outside lambda is 'str', should be fine
x.append(1) # should error: 'str' has no 'append'
"#;
    let findings = analyze_code(code);
    // Findings should only include the outer x.append(1)
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].line, 5);
    assert!(findings[0].message.contains("'str'"));
}

#[test]
fn test_comprehension_scope_isolation() {
    let code = r#"
x = "outer"
l = [x for x in [1, 2, 3]] # x inside is 'unknown'
x.strip() # x outside is still 'str'
x.append(1) # should error: 'str' has no 'append'
"#;
    let findings = analyze_code(code);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].line, 5);
}

#[test]
fn test_nested_scope_isolation() {
    let code = r#"
x = "outer"
f = lambda: [x for x in [1, 2]] # Nested scope
x.append(1) # Error
"#;
    let findings = analyze_code(code);
    assert_eq!(findings.len(), 1);
}

#[test]
fn test_dict_comp_scope_isolation() {
    let code = r#"
k = "outer_k"
v = "outer_v"
d = {k: v for k, v in [("a", 1)]}
k.strip() # OK
v.strip() # OK
k.append(1) # Error
v.append(1) # Error
"#;
    let findings = analyze_code(code);
    assert_eq!(findings.len(), 2);
}
