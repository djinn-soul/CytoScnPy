use super::*;

// --- Bare Except Tests (CSP-L002) ---

#[test]
fn test_bare_except_only() {
    let source = r"
try:
    pass
except:
    pass
";
    let linter = run_linter(source, Config::default());

    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.message.contains("Bare except block"))
        .collect();
    assert_eq!(findings.len(), 1, "Should detect bare except");
}

#[test]
fn test_specific_except_ok() {
    let source = r"
try:
    pass
except ValueError:
    pass
";
    let linter = run_linter(source, Config::default());

    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.message.contains("Bare except block"))
        .collect();
    assert_eq!(findings.len(), 0, "Should not flag specific exception");
}

#[test]
fn test_tuple_except_ok() {
    let source = r"
try:
    pass
except (ValueError, TypeError):
    pass
";
    let linter = run_linter(source, Config::default());

    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.message.contains("Bare except block"))
        .collect();
    assert_eq!(findings.len(), 0, "Should not flag tuple of exceptions");
}

// =============================================================================
// Comprehensive Structure Rule Tests (test_structure.py parity)
// =============================================================================

// --- Too Many Arguments Tests (CSP-C303) ---

#[test]
fn test_too_many_args_basic() {
    // 6 args should trigger when limit is 5
    let source = r"
def too_many(a, b, c, d, e, f):
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
    assert_eq!(findings.len(), 1, "Should detect function with 6 > 5 args");
}

#[test]
fn test_too_many_args_exactly_at_limit() {
    // 5 args should NOT trigger when limit is 5
    let source = r"
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
    assert_eq!(
        findings.len(),
        0,
        "Should not flag function with exactly 5 args"
    );
}

#[test]
fn test_too_many_args_with_star_args() {
    // *args and **kwargs are counted
    let source = r"
def with_stars(a, b, *args, **kwargs):
    pass
";
    let mut config = Config::default();
    config.cytoscnpy.max_args = Some(3);
    let linter = run_linter(source, config);

    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.message.contains("Too many arguments"))
        .collect();
    assert_eq!(findings.len(), 1, "Should count *args and **kwargs (4 > 3)");
}

#[test]
fn test_too_many_args_async_function() {
    let source = r"
async def async_many(a, b, c, d, e, f):
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
    assert_eq!(
        findings.len(),
        1,
        "Should detect async function with too many args"
    );
}

#[test]
fn test_too_many_args_kwonly() {
    // keyword-only args after *
    let source = r"
def kwonly(a, *, b, c, d, e, f):
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
    assert_eq!(findings.len(), 1, "Should count keyword-only args (6 > 5)");
}

// --- Function Too Long Tests (CSP-C304) ---

#[test]
fn test_function_too_long_basic() {
    let source = r"
def long_function():
    line1 = 1
    line2 = 2
    line3 = 3
    line4 = 4
    line5 = 5
    line6 = 6
";
    let mut config = Config::default();
    config.cytoscnpy.max_lines = Some(5);
    let linter = run_linter(source, config);

    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.message.contains("Function too long"))
        .collect();
    assert_eq!(
        findings.len(),
        1,
        "Should detect function exceeding 5 lines"
    );
}

#[test]
fn test_function_too_long_exactly_at_limit() {
    let source = r"def short():
    pass
    pass
";
    let mut config = Config::default();
    config.cytoscnpy.max_lines = Some(3);
    let linter = run_linter(source, config);

    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.message.contains("Function too long"))
        .collect();
    assert_eq!(
        findings.len(),
        0,
        "Should not flag function at exactly 3 lines"
    );
}

#[test]
fn test_function_too_long_async() {
    let source = r"
async def async_long():
    await line1()
    await line2()
    await line3()
    await line4()
    await line5()
    await line6()
";
    let mut config = Config::default();
    config.cytoscnpy.max_lines = Some(5);
    let linter = run_linter(source, config);

    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.message.contains("Function too long"))
        .collect();
    assert_eq!(
        findings.len(),
        1,
        "Should detect async function exceeding limit"
    );
}

#[test]
fn test_function_too_long_with_docstring() {
    // Note: Rust implementation includes docstring in line count
    let source = r#"
def with_docstring():
    """This is a docstring.

    It spans multiple lines.
    """
    pass
"#;
    let mut config = Config::default();
    config.cytoscnpy.max_lines = Some(5);
    let linter = run_linter(source, config);

    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.message.contains("Function too long"))
        .collect();
    // Docstring is included in count, so this should trigger
    assert_eq!(findings.len(), 1, "Should include docstring in line count");
}

#[test]
fn test_function_too_long_nested_not_double_counted() {
    // Nested function should be counted separately
    // Note: Rust counts outer function lines including nested definition
    let source = r"
def outer():
    def inner():
        pass
        pass
    pass
    pass
";
    let mut config = Config::default();
    config.cytoscnpy.max_lines = Some(5);
    let linter = run_linter(source, config);

    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.message.contains("Function too long"))
        .collect();
    // Outer is 7 lines (from def to last pass) - should trigger
    assert_eq!(
        findings.len(),
        1,
        "Should count outer function including nested"
    );
}
