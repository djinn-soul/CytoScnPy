//! specific Radon ported complexity tests.

use cytoscnpy::complexity::analyze_complexity;
use std::path::PathBuf;

#[test]
fn test_radon_simple_blocks() {
    // Ported from radon/tests/test_complexity_visitor.py SIMPLE_BLOCKS
    let cases = vec![
        (
            r"
def f(a, b, c):
    if a:
        if b:
            print(c)
    elif c:
        return 2
            ",
            "f",
            4,
        ),
        (
            r"
def f(a, b):
    return a and b
            ",
            "f",
            2,
        ),
        (
            r"
def f(a, b):
    return a or b
            ",
            "f",
            2,
        ),
        (
            r"
def f(a):
    if a:
        return
    elif a:
        return
    else:
        return
            ",
            "f",
            3,
        ),
    ];

    for (code, name, expected_cc) in cases {
        let findings = analyze_complexity(code, &PathBuf::from("test.py"), false);
        let func = findings
            .iter()
            .find(|f| f.name == name)
            .expect("Function not found");
        assert_eq!(func.complexity, expected_cc, "Failed for code:\n{code}");
    }
}

#[test]
fn test_radon_complex_blocks() {
    // Ported from radon/tests/test_complexity_visitor.py COMPLEX_BLOCKS (subset)
    let cases = vec![
        (
            r"
class A(object):
    def m(self, a):
        if a:
            return
            ",
            "m",
            2,
        ),
        (
            r"
class A(object):
    def m(self, a):
        if a:
            return
        elif b:
            return
            ",
            "m",
            3,
        ),
    ];

    for (code, name, expected_cc) in cases {
        let findings = analyze_complexity(code, &PathBuf::from("test.py"), false);
        let func = findings
            .iter()
            .find(|f| f.name == name)
            .expect("Function not found");
        assert_eq!(func.complexity, expected_cc, "Failed for code:\n{code}");
    }
}
