//! Tests using complex real-world data examples.

use cytoscnpy::analyzer::CytoScnPy;
use std::path::PathBuf;

#[test]
fn test_complex_real_data() {
    let mut root_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root_path.push("tests");
    root_path.push("data");
    root_path.push("complex_dynamic");

    // Enable include_tests=true to ensure we scan everything (though these are not technically test files)
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
        .map(|f| f.simple_name.clone())
        .collect();

    // Debug
    println!("Unused functions: {unused_funcs:?}");
    println!("Unused methods: {unused_methods:?}");

    // Scenario A: Handlers (hasattr) - these are METHODS in Handler class
    assert!(
        !unused_methods.contains(&"handle_login".to_owned()),
        "handle_login should be used"
    );
    assert!(
        !unused_methods.contains(&"handle_logout".to_owned()),
        "handle_logout should be used"
    );
    assert!(
        unused_methods.contains(&"handle_unused".to_owned()),
        "handle_unused should be unused"
    );

    // Scenario B: State handlers - these are standalone FUNCTIONS in state_handlers.py
    assert!(
        !unused_funcs.contains(&"handle_state_start".to_owned()),
        "handle_state_start should be used"
    );
    assert!(
        !unused_funcs.contains(&"handle_state_end".to_owned()),
        "handle_state_end should be used"
    );
    assert!(
        unused_funcs.contains(&"handle_state_unused".to_owned()),
        "handle_state_unused should be unused"
    );
}
