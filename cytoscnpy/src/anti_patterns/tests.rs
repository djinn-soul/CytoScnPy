//! Unit tests for anti-pattern detection.

use super::python::detect_anti_patterns;
use super::types::AntiPatternKind;
use std::path::Path;

#[test]
fn test_magic_number_in_comparison() {
    let source = r#"
def handle_response(status):
    if status == 404:
        return "Not found"
    return "OK"
"#;
    let matches = detect_anti_patterns(source, Path::new("test.py"));
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].kind, AntiPatternKind::MagicNumber);
    assert_eq!(matches[0].line, 3);
    assert_eq!(matches[0].pattern_name, "magic_number");
}

#[test]
fn test_magic_number_greater_than() {
    let source = r#"
def check_limit(count):
    if count > 500:
        raise ValueError("Too many items")
"#;
    let matches = detect_anti_patterns(source, Path::new("test.py"));
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].kind, AntiPatternKind::MagicNumber);
}

#[test]
fn test_magic_number_in_while() {
    let source = r"
def retry_loop():
    attempts = 0
    while attempts < 1000:
        attempts += 1
";
    let matches = detect_anti_patterns(source, Path::new("test.py"));
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].kind, AntiPatternKind::MagicNumber);
}

#[test]
fn test_magic_number_float() {
    let source = r"
def check_ratio(val):
    if val >= 100.5:
        return True
    return False
";
    let matches = detect_anti_patterns(source, Path::new("test.py"));
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].kind, AntiPatternKind::MagicNumber);
}

#[test]
fn test_constant_assignment_not_flagged() {
    let source = r"
MAX_LIMIT = 500
HTTP_NOT_FOUND: int = 404
DEFAULT_TIMEOUT = 3600

def process():
    return MAX_LIMIT
";
    let matches = detect_anti_patterns(source, Path::new("test.py"));
    assert!(
        matches.is_empty(),
        "Constant definitions should not be flagged as magic numbers"
    );
}

#[test]
fn test_small_numbers_not_flagged() {
    let source = r"
def clean_logic(x, y):
    if x == 0:
        return 1
    if y == 2:
        return -1
    for i in range(10):
        pass
    return x + y
";
    let matches = detect_anti_patterns(source, Path::new("test.py"));
    assert!(
        matches.is_empty(),
        "Small numbers (0, 1, 2, 10) in logic should not be flagged"
    );
}

#[test]
fn test_deeply_nested_callback_def() {
    let source = concat!(
        "def outer():\n",
        "    def mid():\n",
        "        def inner():\n",
        "            def deep():\n",
        "                def deepest():\n",
        "                    pass\n",
    );
    let matches = detect_anti_patterns(source, Path::new("test.py"));
    let nested = matches
        .iter()
        .filter(|m| m.kind == AntiPatternKind::DeeplyNestedCallback)
        .collect::<Vec<_>>();
    assert!(!nested.is_empty());
    assert_eq!(nested[0].line, 5);
}

#[test]
fn test_deeply_nested_control_structure() {
    let source = concat!(
        "def deep_logic():\n",
        "    if True:\n",
        "        for item in [1]:\n",
        "            while False:\n",
        "                if item > 1:\n",
        "                    print(item)\n",
    );
    let matches = detect_anti_patterns(source, Path::new("test.py"));
    let nested = matches
        .iter()
        .filter(|m| m.kind == AntiPatternKind::DeeplyNestedCallback)
        .collect::<Vec<_>>();
    assert!(!nested.is_empty());
    assert_eq!(nested[0].line, 5);
}

#[test]
fn test_deeply_nested_callback_then() {
    let source = concat!(
        "def setup_chain():\n",
        "    if ready:\n",
        "        for task in tasks:\n",
        "            while waiting:\n",
        "                .then(lambda res: res)\n",
    );
    let matches = detect_anti_patterns(source, Path::new("test.py"));
    let nested = matches
        .iter()
        .filter(|m| m.kind == AntiPatternKind::DeeplyNestedCallback)
        .collect::<Vec<_>>();
    assert!(!nested.is_empty());
    assert_eq!(nested[0].line, 5);
}

#[test]
fn test_comments_ignored_for_nesting() {
    let source = "def foo():\n    if True:\n        for x in [1]:\n            while True:\n                # if commented out:\n                pass\n";
    let matches = detect_anti_patterns(source, Path::new("test.py"));
    let nested = matches
        .iter()
        .filter(|m| m.kind == AntiPatternKind::DeeplyNestedCallback)
        .collect::<Vec<_>>();
    assert!(nested.is_empty());
}

#[test]
fn test_clean_file() {
    let source = r"
def calculate_sum(a, b):
    return a + b

def is_positive(x):
    return x > 0
";
    let matches = detect_anti_patterns(source, Path::new("test.py"));
    assert!(matches.is_empty());
}

#[test]
fn test_combined_patterns() {
    let source = "def combined():\n    if x > 300:\n        for i in range(5):\n            while True:\n                if flag:\n                    return i\n";
    let matches = detect_anti_patterns(source, Path::new("test.py"));
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].kind, AntiPatternKind::MagicNumber);
    assert_eq!(matches[1].kind, AntiPatternKind::DeeplyNestedCallback);
}
