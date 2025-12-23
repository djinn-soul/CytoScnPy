//! Unit tests for test awareness
//! Tests detection of test files and test functions
#![allow(clippy::expect_used)]

use cytoscnpy::test_utils::TestAwareVisitor;
use cytoscnpy::utils::LineIndex;
use ruff_python_parser::{parse, Mode};
use std::path::PathBuf;

#[test]
fn test_pytest_function_detection() {
    let source = r"
def test_something():
    assert True

def test_another_thing():
    assert 1 + 1 == 2

def regular_function():
    return 42
";

    let tree = parse(source, Mode::Module.into()).expect("Failed to parse");
    let line_index = LineIndex::new(source);
    let mut visitor = TestAwareVisitor::new(&PathBuf::from("test_file.py"), &line_index);

    if let ruff_python_ast::Mod::Module(module) = tree.into_syntax() {
        for stmt in &module.body {
            visitor.visit_stmt(stmt);
        }
    }

    // Should detect test functions
    assert!(
        visitor.test_decorated_lines.len() >= 2,
        "Should detect test functions"
    );
}

#[test]
fn test_file_name_detection() {
    let test_files = vec![
        "module_test.py",
        "tests/something.py", // Correct regex matches tests/ or test/
    ];

    for filename in test_files {
        let source = "def foo(): pass";
        let _tree = parse(source, Mode::Module.into()).expect("Failed to parse");
        let line_index = LineIndex::new(source);
        let visitor = TestAwareVisitor::new(&PathBuf::from(filename), &line_index);

        assert!(
            visitor.is_test_file,
            "Should detect {filename} as test file"
        );
    }
}

#[test]
fn test_non_test_file_detection() {
    let source = "def foo(): pass";
    let _tree = parse(source, Mode::Module.into()).expect("Failed to parse");
    let line_index = LineIndex::new(source);
    let visitor = TestAwareVisitor::new(&PathBuf::from("regular_module.py"), &line_index);

    assert!(
        !visitor.is_test_file,
        "Should not detect regular file as test file"
    );
}
