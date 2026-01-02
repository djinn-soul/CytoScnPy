//! Additional tests for `IPython` notebook processing.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use std::path::Path;

#[test]
fn test_additional_empty_notebook() {
    let notebook_path = Path::new("tests/data/notebooks/empty_notebook.ipynb");
    let code = cytoscnpy::ipynb::extract_notebook_code(notebook_path, None).unwrap();
    assert_eq!(code, "", "Empty notebook should produce empty code");
}

#[test]
fn test_additional_complex_unused() {
    let mut analyzer = CytoScnPy::default()
        .with_confidence(60)
        .with_tests(false)
        .with_ipynb(true);
    let notebook_dir = Path::new("tests/data/notebooks");
    let result = analyzer.analyze(notebook_dir);

    // With multiple notebooks, should analyze more files
    assert!(result.analysis_summary.total_files >= 3);
}

#[test]
fn test_additional_notebook_with_imports() {
    let notebook_path = Path::new("tests/data/notebooks/complex_unused.ipynb");
    let code = cytoscnpy::ipynb::extract_notebook_code(notebook_path, None).unwrap();

    // Should contain import statements
    assert!(code.contains("import os"));
    assert!(code.contains("import sys"));
    assert!(code.contains("def process_data"));
    assert!(code.contains("def unused_helper"));
}

#[test]
fn test_additional_notebook_file_count() {
    // Test that multiple notebooks are discovered
    let mut analyzer = CytoScnPy::default()
        .with_confidence(0)
        .with_tests(false)
        .with_ipynb(true);
    let notebook_dir = Path::new("tests/data/notebooks");
    let result = analyzer.analyze(notebook_dir);

    // Should find all .ipynb files (currently 5)
    assert!(
        result.analysis_summary.total_files >= 4,
        "Should find at least 4 notebook files"
    );
}
