//! Tests running on real data scenarios.

use cytoscnpy::analyzer::CytoScnPy;
use std::path::PathBuf;

#[test]
fn test_real_data_scenarios() {
    let mut root_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root_path.push("tests");
    root_path.push("data");
    root_path.push("complex_dynamic");

    // Enable include_tests=true to ensure we scan everything
    let mut cytoscnpy = CytoScnPy::default().with_confidence(100).with_tests(true);
    let result = cytoscnpy.analyze(&root_path).unwrap();

    let unused_funcs: Vec<String> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    let unused_methods: Vec<String> = result
        .unused_methods
        .iter()
        .map(|m| m.simple_name.clone())
        .collect();

    // 1. Check hidden_gem (function) should be used via globals()
    assert!(
        !unused_funcs.contains(&"hidden_gem".to_owned()),
        "hidden_gem should be used"
    );

    // 2. Check User.save (method) should be used via processor.py hasattr()
    println!("Unused functions: {unused_funcs:?}");
    println!("Unused methods: {unused_methods:?}");
    assert!(
        !unused_methods.contains(&"save".to_owned()),
        "User.save should be used"
    );
    assert!(
        unused_methods.contains(&"delete".to_owned()),
        "User.delete should be unused"
    );

    // 3. Check local variables in script.py (should be used via eval())
    let unused_vars: Vec<String> = result
        .unused_variables
        .iter()
        .map(|v| v.simple_name.clone())
        .collect();

    assert!(!unused_vars.contains(&"x".to_owned()), "x should be used");
    assert!(!unused_vars.contains(&"y".to_owned()), "y should be used");
}
