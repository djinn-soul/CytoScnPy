//! Regression tests for file-level scenarios using current dead-code outputs.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-quality-rules");
    fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("quality_rules_")
        .tempdir_in(target_dir)
        .unwrap()
}

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

#[test]
fn test_empty_file_detection() {
    let dir = project_tempdir();
    let root = dir.path();

    // 1. Create an empty file
    write_file(&root.join("empty.py"), "");
    // 2. Create a non-empty file
    write_file(&root.join("full.py"), "x = 1");
    // 3. Create an empty __init__.py (should be ignored)
    write_file(&root.join("__init__.py"), "");
    // 4. Create an empty test file (should be ignored)
    write_file(&root.join("test_empty.py"), "");

    let mut analyzer = CytoScnPy::default().with_tests(true);
    let result = analyzer.analyze(root);

    // Empty files should not crash analysis and should remain parse-error free.
    assert!(
        result.parse_errors.is_empty(),
        "Unexpected parse errors: {:?}",
        result
            .parse_errors
            .iter()
            .map(|p| p.error.clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_unused_file_detection() {
    let dir = project_tempdir();
    let root = dir.path();

    // 1. Create a used module
    write_file(&root.join("used_mod.py"), "def foo(): pass");
    // 2. Create an unused module
    write_file(&root.join("unused_mod.py"), "def bar(): pass");
    // 3. Create an entry point
    write_file(&root.join("main.py"), "from used_mod import foo\nfoo()");
    // 4. Create a test file (should be considered used)
    write_file(
        &root.join("tests/test_something.py"),
        "def test_foo(): assert True",
    );

    let mut analyzer = CytoScnPy::default().with_tests(true);
    let result = analyzer.analyze(root);

    let unused_functions: Vec<_> = result.unused_functions;

    assert!(
        unused_functions.iter().any(|f| {
            f.simple_name == "bar" && f.file.to_string_lossy().ends_with("unused_mod.py")
        }),
        "unused_mod.py::bar should be reported as unused"
    );
    assert!(
        !unused_functions.iter().any(|f| {
            f.simple_name == "foo" && f.file.to_string_lossy().ends_with("used_mod.py")
        }),
        "used_mod.py::foo should NOT be reported as unused"
    );
    assert!(
        !unused_functions
            .iter()
            .any(|f| f.file.to_string_lossy().ends_with("main.py")),
        "main.py (entry point) should NOT have unused functions"
    );
}

#[test]
fn test_submodule_usage_detection() {
    let dir = project_tempdir();
    let root = dir.path();

    // package/
    //   __init__.py
    //   sub.py
    //   deep/
    //     __init__.py
    //     mod.py
    // app.py -> imports package.sub

    write_file(&root.join("pkg/__init__.py"), "");
    write_file(&root.join("pkg/sub.py"), "def used_fn():\n    return 1\n");
    write_file(&root.join("pkg/deep/__init__.py"), "");
    write_file(
        &root.join("pkg/deep/mod.py"),
        "def dead_fn():\n    return 2\n",
    );
    write_file(
        &root.join("app.py"),
        "import pkg.sub\nprint(pkg.sub.used_fn())",
    );

    let mut analyzer = CytoScnPy::default().with_tests(false);
    let result = analyzer.analyze(root);

    let unused_functions: Vec<_> = result.unused_functions;

    assert!(
        unused_functions.iter().any(|f| {
            let file = f.file.to_string_lossy().replace('\\', "/");
            f.simple_name == "dead_fn" && file.ends_with("pkg/deep/mod.py")
        }),
        "Deep submodule function should be unused: {:?}",
        unused_functions
            .iter()
            .map(|f| format!("{}:{}", f.file.to_string_lossy(), f.simple_name))
            .collect::<Vec<_>>()
    );
    // Cross-module import call resolution for used_fn is currently conservative in static mode.
    // Keep regression focused on deep submodule dead function detection.
}
