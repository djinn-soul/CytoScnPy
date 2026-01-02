//! Tests for cyclomatic complexity calculation.

use cytoscnpy::complexity::analyze_complexity;
use std::path::Path;

#[test]
fn test_complexity_simple() {
    let code = r#"
def simple():
    if True:
        print("yes")
    else:
        print("no")
"#;
    let findings = analyze_complexity(code, Path::new("test.py"), false);
    assert_eq!(findings.len(), 1);
    let f = &findings[0];
    assert_eq!(f.name, "simple");
    assert_eq!(f.type_, "function");
    // Base 1 + If 1 = 2
    assert_eq!(f.complexity, 2);
    assert_eq!(f.rank, 'A');
}

#[test]
fn test_complexity_nested() {
    let code = r"
def nested(x):
    if x > 0:
        if x > 10:
            return 2
        else:
            return 1
    elif x < 0:
        return -1
    return 0
";
    let findings = analyze_complexity(code, Path::new("test.py"), false);
    assert_eq!(findings.len(), 1);
    let f = &findings[0];
    assert_eq!(f.name, "nested");
    // Base 1
    // If x > 0: +1
    //   If x > 10: +1
    // Elif x < 0: +1
    // Total = 4
    assert_eq!(f.complexity, 4);
    assert_eq!(f.rank, 'A');
}

#[test]
fn test_complexity_loops() {
    let code = r"
def loops():
    for i in range(10):
        while i > 0:
            i -= 1
";
    let findings = analyze_complexity(code, Path::new("test.py"), false);
    assert_eq!(findings.len(), 1);
    let f = &findings[0];
    assert_eq!(f.name, "loops");
    // Base 1 + For 1 + While 1 = 3
    assert_eq!(f.complexity, 3);
}

#[test]
fn test_complexity_boolean() {
    let code = r"
def boolean(a, b, c):
    if a and b or c:
        pass
";
    let findings = analyze_complexity(code, Path::new("test.py"), false);
    assert_eq!(findings.len(), 1);
    let f = &findings[0];
    assert_eq!(f.name, "boolean");
    // Base 1
    // If: +1
    // and: +1
    // or: +1
    // Total = 4
    assert_eq!(f.complexity, 4);
}

#[test]
fn test_complexity_class() {
    let code = r"
class MyClass:
    def method(self):
        if True:
            pass
";
    let findings = analyze_complexity(code, Path::new("test.py"), false);
    // Should find Class and Method
    assert_eq!(findings.len(), 2);

    // Order depends on traversal. Usually class first, then method.
    // My visitor pushes finding BEFORE recursing.
    // So Class first.

    let class_f = &findings[0];
    assert_eq!(class_f.name, "MyClass");
    assert_eq!(class_f.type_, "class");
    // Class body has `def method`.
    // Does `def` add complexity to class?
    // No, `def` is a definition.
    // So Class complexity = 1 (Base).
    assert_eq!(class_f.complexity, 1);

    let method_f = &findings[1];
    assert_eq!(method_f.name, "method");
    assert_eq!(method_f.type_, "method");
    // Method: Base 1 + If 1 = 2
    assert_eq!(method_f.complexity, 2);
}

#[test]
fn test_no_assert_flag() {
    let code = r"
def test_func():
    assert x > 0
    assert y > 0
    if z:
        pass
";
    // With no_assert=false, assert statements add to complexity
    let findings_with_assert = analyze_complexity(code, Path::new("test.py"), false);
    let f_with = &findings_with_assert[0];
    // Base 1 + assert 1 + assert 1 + if 1 = 4
    assert_eq!(
        f_with.complexity, 4,
        "With no_assert=false, asserts should add complexity"
    );

    // With no_assert=true, assert statements DON'T add to complexity
    let findings_no_assert = analyze_complexity(code, Path::new("test.py"), true);
    let f_no = &findings_no_assert[0];
    // Base 1 + if 1 = 2 (asserts ignored)
    assert_eq!(
        f_no.complexity, 2,
        "With no_assert=true, asserts should NOT add complexity"
    );
}
