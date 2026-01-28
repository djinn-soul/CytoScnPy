//! Tests for scope resolution (shadowing, closures, etc.).

use cytoscnpy::analyzer::CytoScnPy;
use std::path::PathBuf;

#[test]
fn test_scope_shadowing() {
    let dir_path = PathBuf::from("tests/python_files/scope");
    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(true);
    let result = cytoscnpy.analyze(&dir_path);

    let unused_vars: Vec<String> = result
        .unused_variables
        .iter()
        .map(|v| v.full_name.clone())
        .collect();

    // 1. func() has local x. print(x) should ref local x.
    // If resolved correctly, func.x is USED.
    assert!(
        !unused_vars.contains(&"shadowing.func.x".to_owned()),
        "func.x should be used"
    );

    // 2. func2() has no local x. print(x) should ref global x.
    // If resolved correctly, global x is USED.
    assert!(
        !unused_vars.contains(&"shadowing.x".to_owned()),
        "global x should be used"
    );

    // 3. method() has local x. print(x) should ref local x.
    // If resolved correctly, method.x is USED.
    assert!(
        !unused_vars.contains(&"shadowing.C.method.x".to_owned()),
        "method.x should be used"
    );
}

#[test]
fn test_scope_closures() {
    let dir_path = PathBuf::from("tests/python_files/scope");
    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(true);
    let result = cytoscnpy.analyze(&dir_path);

    let unused_vars: Vec<String> = result
        .unused_variables
        .iter()
        .map(|v| v.full_name.clone())
        .collect();

    // outer() has x. inner() uses x.
    // inner() print(x) should ref outer.x.
    // So outer.x should be USED.
    assert!(
        !unused_vars.contains(&"closures.outer.x".to_owned()),
        "outer.x should be used by inner function"
    );
}

#[test]
fn test_scope_classes() {
    let dir_path = PathBuf::from("tests/python_files/scope");
    let mut cytoscnpy = CytoScnPy::default().with_confidence(10).with_tests(true);
    let result = cytoscnpy.analyze(&dir_path);

    let unused_vars: Vec<String> = result
        .unused_variables
        .iter()
        .map(|v| v.full_name.clone())
        .collect();

    // class A has x. method m() prints x.
    // In Python, m() cannot see A.x directly. It sees global x (if any).
    // Here there is no global x.
    // So m() print(x) should NOT ref A.x.
    // A.x should be UNUSED.

    assert!(
        unused_vars.contains(&"classes.A._class_unique_x".to_owned()),
        "A._class_unique_x should be unused because methods cannot see class scope directly"
    );
}
