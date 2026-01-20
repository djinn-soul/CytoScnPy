//! Unit tests for quality rules
//! Tests code quality checks like nesting depth
#![allow(clippy::expect_used)]

use cytoscnpy::config::Config;
use cytoscnpy::linter::LinterVisitor;
use cytoscnpy::rules::quality::get_quality_rules;
use cytoscnpy::utils::LineIndex;
use ruff_python_parser::{parse, Mode};
use std::path::PathBuf;

fn run_linter(source: &str, config: Config) -> LinterVisitor {
    let tree = parse(source, Mode::Module.into()).expect("Failed to parse");
    let line_index = LineIndex::new(source);
    let rules = get_quality_rules(&config);
    let mut linter = LinterVisitor::new(rules, PathBuf::from("test.py"), line_index, config);

    if let ruff_python_ast::Mod::Module(module) = tree.into_syntax() {
        for stmt in &module.body {
            linter.visit_stmt(stmt);
        }
    }
    linter
}

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
