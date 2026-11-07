use super::*;

#[test]
fn test_deeply_nested_code_detection() {
    let source = r#"
def deeply_nested():
    if True:
        if True:
            if True:
                if True:
                    if True:
                        if True:
                            print("too deep")
"#;
    let linter = run_linter(source, Config::default());

    assert!(
        !linter.findings.is_empty(),
        "Should detect deeply nested code"
    );
    assert!(linter
        .findings
        .iter()
        .any(|f| f.message.contains("Deeply nested code")));
}

#[test]
fn test_acceptable_nesting() {
    let source = r"
def normal_function():
    if True:
        for item in range(10):
            print(item)
";
    let linter = run_linter(source, Config::default());

    // Should not flag normal nesting (depth <= 5)
    assert_eq!(
        linter.findings.len(),
        0,
        "Should not flag acceptable nesting"
    );
}

#[test]
fn test_mutable_default_arguments() {
    let source = r#"
def bad_defaults(x=[], y={}, z=set()):
    pass

def good_defaults(x=None, y=1, z="string"):
    pass
"#;
    let linter = run_linter(source, Config::default());

    assert!(linter
        .findings
        .iter()
        .any(|f| f.message.contains("Mutable default argument")));
    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.message.contains("Mutable default argument"))
        .collect();
    assert_eq!(
        findings.len(),
        3,
        "Should detect list, dict, and set defaults"
    );
}

#[test]
fn test_bare_except() {
    let source = r"
try:
    pass
except:
    pass

try:
    pass
except ValueError:
    pass
";
    let linter = run_linter(source, Config::default());

    assert!(linter
        .findings
        .iter()
        .any(|f| f.message.contains("Bare except block")));
    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.message.contains("Bare except block"))
        .collect();
    assert_eq!(findings.len(), 1, "Should detect exactly one bare except");
}

#[test]
fn test_dangerous_comparison() {
    let source = r"
if x == True: pass
if x == False: pass
if x == None: pass
if x is None: pass  # OK
if x: pass          # OK
";
    let linter = run_linter(source, Config::default());

    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.message.contains("Dangerous comparison"))
        .collect();
    assert_eq!(
        findings.len(),
        3,
        "Should detect == True, == False, == None"
    );
}

#[test]
fn test_argument_count() {
    let source = r"
def too_many(a, b, c, d, e, f):
    pass

def okay(a, b, c, d, e):
    pass
";
    let mut config = Config::default();
    config.cytoscnpy.max_args = Some(5);
    let linter = run_linter(source, config);

    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.message.contains("Too many arguments"))
        .collect();
    assert_eq!(findings.len(), 1, "Should detect function with > 5 args");
}

#[test]
fn test_function_length() {
    // 5 lines
    let source = r"
def short_function():
    print(1)
    print(2)
    print(3)
";
    let mut config = Config::default();
    config.cytoscnpy.max_lines = Some(3); // Set low limit for testing
    let linter = run_linter(source, config);

    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.message.contains("Function too long"))
        .collect();
    assert_eq!(
        findings.len(),
        1,
        "Should detect function exceeding line limit"
    );
}

#[test]
fn test_complexity() {
    let source = r"
def complex_function(x):
    if x:
        if x:
            if x:
                print(x)
    elif x:
        for i in x:
            while True:
                pass
";
    let mut config = Config::default();
    config.cytoscnpy.max_complexity = Some(5); // Set low limit
    let linter = run_linter(source, config);

    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.message.contains("Function is too complex"))
        .collect();
    assert_eq!(findings.len(), 1, "Should detect complex function");
}

// =============================================================================
// Comprehensive Logic Rule Tests
// =============================================================================

// --- Mutable Default Argument Tests (CSP-L001) ---

#[test]
fn test_list_default() {
    let source = "def bad(x=[]): pass";
    let linter = run_linter(source, Config::default());

    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.message.contains("Mutable default argument"))
        .collect();
    assert_eq!(findings.len(), 1, "Should detect list default []");
}

#[test]
fn test_dict_default() {
    let source = "def bad(x={}): pass";
    let linter = run_linter(source, Config::default());

    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.message.contains("Mutable default argument"))
        .collect();
    assert_eq!(findings.len(), 1, "Should detect dict default {{}}");
}

#[test]
fn test_set_default() {
    let source = "def bad(x={1}): pass";
    let linter = run_linter(source, Config::default());

    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.message.contains("Mutable default argument"))
        .collect();
    assert_eq!(findings.len(), 1, "Should detect set default {{1}}");
}

#[test]
fn test_valid_default() {
    let source = r"
def good(x=None, y=1, z='string'):
    pass
";
    let linter = run_linter(source, Config::default());

    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.message.contains("Mutable default argument"))
        .collect();
    assert_eq!(
        findings.len(),
        0,
        "Should not flag None, int, or string defaults"
    );
}

#[test]
fn test_kwonly_defaults() {
    let source = "def bad(*, x=[]): pass";
    let linter = run_linter(source, Config::default());

    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.message.contains("Mutable default argument"))
        .collect();
    assert_eq!(
        findings.len(),
        1,
        "Should detect keyword-only mutable default"
    );
}

#[test]
fn test_async_function_mutable() {
    let source = "async def bad(x=[]): pass";
    let linter = run_linter(source, Config::default());

    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.message.contains("Mutable default argument"))
        .collect();
    assert_eq!(
        findings.len(),
        1,
        "Should detect mutable default in async function"
    );
}
