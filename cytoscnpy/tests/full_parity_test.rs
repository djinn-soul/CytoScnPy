//! Tests for full parity with Python implementation.

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::config::Config;
use std::path::PathBuf;

fn analyze_code(code: &str, config: Config) -> cytoscnpy::analyzer::AnalysisResult {
    let analyzer = CytoScnPy::default()
        .with_confidence(0)
        .with_secrets(true)
        .with_danger(true)
        .with_quality(true)
        .with_tests(false)
        .with_config(config);
    analyzer.analyze_code(code, PathBuf::from("test.py"))
}

#[test]
fn test_mutable_default_argument() {
    let code = "def foo(x=[]): pass";
    let config = Config::default();
    let result = analyze_code(code, config);
    assert!(result.quality.iter().any(|f| f.rule_id == "CSP-L001"));
}

#[test]
fn test_bare_except() {
    let code = "try: pass\nexcept: pass";
    let config = Config::default();
    let result = analyze_code(code, config);
    assert!(result.quality.iter().any(|f| f.rule_id == "CSP-L002"));
}

#[test]
fn test_dangerous_comparison() {
    let code = "if x == True: pass";
    let config = Config::default();
    let result = analyze_code(code, config);
    assert!(result.quality.iter().any(|f| f.rule_id == "CSP-L003"));
}

#[test]
fn test_argument_count() {
    let code = "def foo(a, b, c, d, e, f): pass";
    let config = Config::default(); // default max_args is 5
    let result = analyze_code(code, config);
    assert!(result.quality.iter().any(|f| f.rule_id == "CSP-C303"));
}

#[test]
fn test_function_length() {
    let code = "def foo():\n".to_owned() + &"    pass\n".repeat(51);
    let config = Config::default(); // default max_lines is 50
    let result = analyze_code(&code, config);
    assert!(result.quality.iter().any(|f| f.rule_id == "CSP-C304"));
}

#[test]
fn test_complexity() {
    let code = "
def foo(x):
    if x: pass
    if x: pass
    if x: pass
    if x: pass
    if x: pass
    if x: pass
    if x: pass
    if x: pass
    if x: pass
    if x: pass
";
    let config = Config::default(); // default complexity is 10
    let result = analyze_code(code, config);
    assert!(result.quality.iter().any(|f| f.rule_id == "CSP-Q301"));
}

#[test]
fn test_nesting() {
    let code = "
def foo():
    if True:
        if True:
            if True:
                if True:
                    pass
";
    let config = Config::default(); // default nesting is 3
    let result = analyze_code(code, config);
    assert!(result.quality.iter().any(|f| f.rule_id == "CSP-Q302"));
}

#[test]
fn test_path_traversal() {
    let code = "open(user_input)";
    let config = Config::default();
    let result = analyze_code(code, config);
    assert!(result.danger.iter().any(|f| f.rule_id == "CSP-D501"));
}

#[test]
fn test_ssrf() {
    let code = "requests.get(user_url)";
    let config = Config::default();
    let result = analyze_code(code, config);
    assert!(result.danger.iter().any(|f| f.rule_id == "CSP-D402"));
}

#[test]
fn test_sqli_raw() {
    let code = "sqlalchemy.text(user_sql)";
    let config = Config::default();
    let result = analyze_code(code, config);
    assert!(result.danger.iter().any(|f| f.rule_id == "CSP-D102"));
}

#[test]
fn test_xss() {
    let code = "flask.render_template_string(user_template)";
    let config = Config::default();
    let result = analyze_code(code, config);
    assert!(result.danger.iter().any(|f| f.rule_id == "CSP-D103"));
}

#[test]
fn test_complex_complexity() {
    let code = "
def complex_function(x, y):
    for i in range(10):                  # +1
        if x > 5:                        # +1
            try:
                print(i)
            except ValueError:           # +1
                pass
            except TypeError:            # +1
                pass
        elif y < 3:                      # +1
            while True:                  # +1
                if i == 5:               # +1
                    break
        else:
            with open('file') as f:
                if f.read():             # +1
                    pass
                if x == y:               # +1
                    return
    if x == 0:                           # +1
        pass
";
    // Total: 1 (base) + 10 = 11.
    // Default threshold is 10. So 11 > 10 should trigger.

    let config = Config::default();
    let result = analyze_code(code, config);

    assert!(
        result.quality.iter().any(|f| f.rule_id == "CSP-Q301"),
        "Should detect high complexity"
    );

    let finding = result
        .quality
        .iter()
        .find(|f| f.rule_id == "CSP-Q301")
        .unwrap();
    assert!(
        finding.message.contains("McCabe="),
        "Message should contain complexity score"
    );
}
