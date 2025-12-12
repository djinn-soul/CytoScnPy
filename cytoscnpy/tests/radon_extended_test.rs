//! Extended Radon complexity tests for modern Python features.

use cytoscnpy::complexity::analyze_complexity;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_modern_python_features() {
    let path = PathBuf::from("../benchmark/examples/complex/radon_complex/modern_python.py");
    let code = fs::read_to_string(&path).expect("Failed to read modern_python.py");
    let findings = analyze_complexity(&code, &path, false);

    // async_func
    let async_func = findings
        .iter()
        .find(|f| f.name == "async_func")
        .expect("async_func not found");
    // 1 (base) + 1 (async for) + 1 (if) = 3
    assert_eq!(async_func.complexity, 3);

    // match_example
    let match_ex = findings
        .iter()
        .find(|f| f.name == "match_example")
        .expect("match_example not found");
    // 1 (base) + 4 (cases) = 5
    assert_eq!(match_ex.complexity, 5);

    // walrus_example
    let walrus = findings
        .iter()
        .find(|f| f.name == "walrus_example")
        .expect("walrus_example not found");
    // 1 (base) + 1 (if) = 2
    assert_eq!(walrus.complexity, 2);

    // pos_only
    let pos = findings
        .iter()
        .find(|f| f.name == "pos_only")
        .expect("pos_only not found");
    // 1 (base) = 1
    assert_eq!(pos.complexity, 1);
}
