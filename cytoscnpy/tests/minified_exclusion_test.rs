//! Integration tests for excluding minified files from AST analysis.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::commands::find_python_files;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_find_python_files_excludes_minified_by_name() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("normal.py"), "def run():\n    pass\n").unwrap();
    fs::write(root.join("bundle.min.py"), "x = 1\n").unwrap();
    fs::write(root.join("vendor-min.py"), "y = 2\n").unwrap();
    fs::write(root.join("packed_min.py"), "z = 3\n").unwrap();

    let files = find_python_files(&[root.to_path_buf()], &[], false);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].file_name().unwrap().to_str().unwrap(), "normal.py");
}

#[test]
fn test_find_python_files_excludes_minified_by_content_heuristics() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Normal python file
    fs::write(
        root.join("valid.py"),
        "def compute(a, b):\n    return a + b\n",
    )
    .unwrap();

    // Single-line packed file over 1024 bytes
    let packed_payload = "v = [i for i in range(10)]; ".repeat(50);
    assert!(packed_payload.len() > 1024);
    fs::write(root.join("packed.py"), &packed_payload).unwrap();

    let files = find_python_files(&[root.to_path_buf()], &[], false);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].file_name().unwrap().to_str().unwrap(), "valid.py");
}

#[test]
fn test_analyzer_process_single_file_skips_minified() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let min_path = root.join("app.min.py");
    fs::write(&min_path, "def minified(): pass\n").unwrap();

    let mut analyzer = CytoScnPy::default();
    analyzer.analysis_root = root.to_path_buf();
    let res = analyzer.process_single_file(&min_path, root);

    // Skipped minified files return empty analysis result
    assert!(res.definitions.is_empty());
    assert!(res.quality.is_empty());
    assert!(res.danger.is_empty());
    assert!(res.complexity.abs() < f64::EPSILON);
}

#[test]
fn test_analyze_paths_ignores_minified_files() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("main.py"), "def main():\n    return 0\n").unwrap();
    fs::write(root.join("script.min.py"), "x = 1\n").unwrap();

    let mut analyzer = CytoScnPy::default();
    analyzer.analysis_root = root.to_path_buf();
    let result = analyzer.analyze_paths(&[root.to_path_buf()]);

    assert_eq!(result.file_metrics.len(), 1);
    assert!(result
        .file_metrics
        .iter()
        .any(|f| f.file.ends_with("main.py")));
    assert!(!result
        .file_metrics
        .iter()
        .any(|f| f.file.ends_with("script.min.py")));
}
