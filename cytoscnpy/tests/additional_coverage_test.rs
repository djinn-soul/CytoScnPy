//! Additional CLI, Test-Aware, and Integration tests

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::str_to_string,
    clippy::uninlined_format_args,
    clippy::needless_raw_string_hashes
)]

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::config::Config;
use cytoscnpy::test_utils::TestAwareVisitor;
use cytoscnpy::utils::LineIndex;
use ruff_python_parser::{parse, Mode};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-additional-tmp");
    fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("additional_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

// =============================================================================
// CLI FEATURE TESTS
// =============================================================================

#[test]
fn test_json_output_structure() {
    // Test that JSON output has proper structure
    let dir = project_tempdir();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "def unused_func(): pass").unwrap();

    let config = Config::default();
    let mut analyzer = CytoScnPy::new(
        50,     // confidence
        false,  // secrets
        false,  // danger
        false,  // quality
        false,  // include_tests
        vec![], // exclude_folders
        vec![], // include_folders
        false,  // include_ipynb
        false,  // ipynb_cells
        config,
    );

    let result = analyzer.analyze(dir.path());
    let json_str = serde_json::to_string_pretty(&result).unwrap();

    // Verify JSON structure
    assert!(json_str.contains("\"unused_functions\""));
    assert!(json_str.contains("\"unused_classes\""));
    assert!(json_str.contains("\"unused_imports\""));
    assert!(json_str.contains("\"quality\""));
}

#[test]
fn test_exit_code_on_findings() {
    // Analyzer returns findings that can be used for exit codes
    let dir = project_tempdir();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "def unused(): pass").unwrap();

    let config = Config::default();
    let mut analyzer = CytoScnPy::new(
        50,
        false,
        false,
        false,
        false,
        vec![],
        vec![],
        false,
        false,
        config,
    );

    let result = analyzer.analyze(dir.path());

    // Would use this for exit code
    let has_unused = !result.unused_functions.is_empty();
    assert!(
        has_unused,
        "Should detect unused function for exit code logic"
    );
}

#[test]
fn test_error_on_invalid_path() {
    let config = Config::default();
    let mut analyzer = CytoScnPy::new(
        50,
        false,
        false,
        false,
        false,
        vec![],
        vec![],
        false,
        false,
        config,
    );

    let result = analyzer.analyze(std::path::Path::new("/nonexistent/path/xyz"));
    // Should handle gracefully (empty result or error)
    // The implementation returns empty result for non-existent paths
    assert_eq!(result.analysis_summary.total_files, 0);
}

// =============================================================================
// TEST-AWARE FEATURE TESTS
// =============================================================================

#[test]
fn test_pytest_fixture_detection() {
    let source = r"
import pytest

@pytest.fixture
def setup_db():
    return {}

@pytest.fixture(scope='session')
def session_fixture():
    return []
";

    let tree = parse(source, Mode::Module.into()).expect("Failed to parse");
    let line_index = LineIndex::new(source);
    let mut visitor = TestAwareVisitor::new(&PathBuf::from("conftest.py"), &line_index);

    if let ruff_python_ast::Mod::Module(module) = tree.into_syntax() {
        for stmt in &module.body {
            visitor.visit_stmt(stmt);
        }
    }

    // conftest.py should be detected as test file
    assert!(
        visitor.is_test_file,
        "conftest.py should be detected as test file"
    );
}

#[test]
fn test_unittest_lifecycle_methods() {
    let source = r"
import unittest

class TestCase(unittest.TestCase):
    def setUp(self):
        self.data = []
    
    def tearDown(self):
        self.data.clear()
    
    def test_something(self):
        self.assertTrue(True)
";

    let tree = parse(source, Mode::Module.into()).expect("Failed to parse");
    let line_index = LineIndex::new(source);
    let mut visitor = TestAwareVisitor::new(&PathBuf::from("test_unit.py"), &line_index);

    if let ruff_python_ast::Mod::Module(module) = tree.into_syntax() {
        for stmt in &module.body {
            visitor.visit_stmt(stmt);
        }
    }

    // Should detect test file and have test lines
    assert!(
        visitor.is_test_file,
        "test_unit.py should be detected as test file"
    );
    assert!(
        !visitor.test_decorated_lines.is_empty(),
        "Should detect test methods"
    );
}

#[test]
fn test_test_import_detection() {
    let source = r"
import pytest
from unittest import mock
from unittest.mock import patch, MagicMock
";

    let tree = parse(source, Mode::Module.into()).expect("Failed to parse");
    let line_index = LineIndex::new(source);
    let mut visitor = TestAwareVisitor::new(&PathBuf::from("test_imports.py"), &line_index);

    if let ruff_python_ast::Mod::Module(module) = tree.into_syntax() {
        for stmt in &module.body {
            visitor.visit_stmt(stmt);
        }
    }

    // Should detect as test file by filename
    assert!(
        visitor.is_test_file,
        "test_imports.py should be detected as test file"
    );
}

// =============================================================================
// INTEGRATION FEATURE TESTS
// =============================================================================

#[test]
fn test_multi_file_project_analysis() {
    let dir = project_tempdir();

    // Create multiple files
    let file1 = dir.path().join("module_a.py");
    let mut f1 = File::create(&file1).unwrap();
    writeln!(f1, "def helper(): return 42").unwrap();

    let file2 = dir.path().join("module_b.py");
    let mut f2 = File::create(&file2).unwrap();
    writeln!(f2, "from module_a import helper\nresult = helper()").unwrap();

    let config = Config::default();
    let mut analyzer = CytoScnPy::new(
        50,
        false,
        false,
        false,
        false,
        vec![],
        vec![],
        false,
        false,
        config,
    );

    let result = analyzer.analyze(dir.path());

    // helper() is used in module_b, so should NOT be in unused
    let unused_names: Vec<_> = result
        .unused_functions
        .iter()
        .map(|f| f.name.as_str())
        .collect();

    // Cross-file reference should be detected
    assert!(
        !unused_names.contains(&"helper"),
        "helper should be used cross-file"
    );
}

#[test]
fn test_config_file_precedence() {
    let dir = project_tempdir();

    // Create pyproject.toml with lower confidence
    let pyproject = dir.path().join("pyproject.toml");
    fs::write(
        &pyproject,
        r#"
[tool.cytoscnpy]
confidence = 50
"#,
    )
    .unwrap();

    // Create .cytoscnpy.toml with higher confidence (should take precedence)
    let cytoscnpy_toml = dir.path().join(".cytoscnpy.toml");
    fs::write(
        &cytoscnpy_toml,
        r#"
[cytoscnpy]
confidence = 90
"#,
    )
    .unwrap();

    let config = Config::load_from_path(dir.path());

    // .cytoscnpy.toml should take precedence
    assert_eq!(config.cytoscnpy.confidence, Some(90));
}

#[test]
fn test_exclude_folder_logic() {
    let dir = project_tempdir();

    // Create folder structure
    let src_dir = dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    let ignored_dir = dir.path().join("node_modules");
    fs::create_dir(&ignored_dir).unwrap();

    // File in src (should be analyzed)
    let src_file = src_dir.join("main.py");
    let mut f1 = File::create(&src_file).unwrap();
    writeln!(f1, "def my_src_func(): pass").unwrap();

    // File in excluded folder (should NOT be analyzed)
    let ignored_file = ignored_dir.join("ignored.py");
    let mut f2 = File::create(&ignored_file).unwrap();
    writeln!(f2, "def ignored_func(): pass").unwrap();

    let config = Config::default();
    let mut analyzer = CytoScnPy::new(
        50,
        false,
        false,
        false,
        false,
        vec!["node_modules".to_string()], // exclude node_modules
        vec![],
        false,
        false,
        config,
    );

    let result = analyzer.analyze(dir.path());

    // Should find my_src_func from src, but NOT ignored_func from node_modules
    let all_names: Vec<_> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.as_str())
        .collect();

    assert!(
        all_names.contains(&"my_src_func"),
        "Should analyze src/main.py. Found: {:?}",
        all_names
    );
    assert!(
        !all_names.contains(&"ignored_func"),
        "Should NOT analyze node_modules/ignored.py"
    );
}

#[test]
fn test_include_folder_overrides_exclude() {
    let dir = project_tempdir();

    // Create a folder that would normally be excluded by default
    let venv_dir = dir.path().join("venv");
    fs::create_dir(&venv_dir).unwrap();

    // File in venv (normally excluded)
    let venv_file = venv_dir.join("special.py");
    let mut f = File::create(&venv_file).unwrap();
    writeln!(f, "def venv_func(): pass").unwrap();

    let config = Config::default();
    let mut analyzer = CytoScnPy::new(
        50,
        false,
        false,
        false,
        false,
        vec![],                   // no extra excludes
        vec!["venv".to_string()], // force include venv
        false,
        false,
        config,
    );

    let result = analyzer.analyze(dir.path());

    // Should find venv_func because venv is force-included
    let all_names: Vec<_> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.as_str())
        .collect();

    assert!(
        all_names.contains(&"venv_func"),
        "Should analyze venv when force-included. Found: {:?}",
        all_names
    );
}
