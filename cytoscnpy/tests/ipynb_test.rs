//! Tests for ipynb.rs - Jupyter notebook parsing.
#![allow(clippy::unwrap_used)]

use cytoscnpy::ipynb::{extract_notebook_cells, extract_notebook_code};
use std::fs;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-ipynb-tmp");
    fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("ipynb_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

/// Create a simple valid notebook JSON structure for testing.
fn create_v4_notebook(cells: &[(&str, &str)]) -> String {
    let cell_json: Vec<String> = cells
        .iter()
        .map(|(cell_type, source)| {
            if *cell_type == "code" {
                format!(
                    r#"{{"cell_type": "code", "execution_count": null, "metadata": {{}}, "outputs": [], "source": ["{source}"]}}"#
                )
            } else {
                format!(
                    r#"{{"cell_type": "markdown", "metadata": {{}}, "source": ["{source}"]}}"#
                )
            }
        })
        .collect();

    format!(
        r#"{{
            "cells": [{}],
            "metadata": {{"kernelspec": {{"display_name": "Python 3", "language": "python", "name": "python3"}}}},
            "nbformat": 4,
            "nbformat_minor": 4
        }}"#,
        cell_json.join(",")
    )
}

#[test]
fn test_extract_notebook_code_single_cell() {
    let dir = project_tempdir();
    let notebook_path = dir.path().join("test.ipynb");

    let notebook_json = create_v4_notebook(&[("code", "print('hello')")]);
    fs::write(&notebook_path, notebook_json).unwrap();

    let result = extract_notebook_code(&notebook_path, None);
    assert!(result.is_ok());
    let code = result.unwrap();
    assert!(code.contains("print('hello')"));
}

#[test]
fn test_extract_notebook_code_multiple_cells() {
    let dir = project_tempdir();
    let notebook_path = dir.path().join("test.ipynb");

    let notebook_json = create_v4_notebook(&[
        ("code", "x = 1"),
        ("code", "y = 2"),
        ("code", "print(x + y)"),
    ]);
    fs::write(&notebook_path, notebook_json).unwrap();

    let result = extract_notebook_code(&notebook_path, None);
    assert!(result.is_ok());
    let code = result.unwrap();
    assert!(code.contains("x = 1"));
    assert!(code.contains("y = 2"));
    assert!(code.contains("print(x + y)"));
}

#[test]
fn test_extract_notebook_code_mixed_cells() {
    let dir = project_tempdir();
    let notebook_path = dir.path().join("test.ipynb");

    let notebook_json = create_v4_notebook(&[
        ("markdown", "# Header"),
        ("code", "x = 1"),
        ("markdown", "Some text"),
        ("code", "y = 2"),
    ]);
    fs::write(&notebook_path, notebook_json).unwrap();

    let result = extract_notebook_code(&notebook_path, None);
    assert!(result.is_ok());
    let code = result.unwrap();
    // Should only contain code cells
    assert!(code.contains("x = 1"));
    assert!(code.contains("y = 2"));
    assert!(!code.contains("# Header"));
}

#[test]
fn test_extract_notebook_code_empty_notebook() {
    let dir = project_tempdir();
    let notebook_path = dir.path().join("test.ipynb");

    let notebook_json = create_v4_notebook(&[]);
    fs::write(&notebook_path, notebook_json).unwrap();

    let result = extract_notebook_code(&notebook_path, None);
    assert!(result.is_ok());
    let code = result.unwrap();
    assert!(code.is_empty());
}

#[test]
fn test_extract_notebook_code_nonexistent_file() {
    let result = extract_notebook_code(std::path::Path::new("/nonexistent/path.ipynb"), None);
    assert!(result.is_err());
}

#[test]
fn test_extract_notebook_code_invalid_json() {
    let dir = project_tempdir();
    let notebook_path = dir.path().join("invalid.ipynb");
    fs::write(&notebook_path, "not valid json").unwrap();

    let result = extract_notebook_code(&notebook_path, None);
    assert!(result.is_err());
}

#[test]
fn test_extract_notebook_cells_single_cell() {
    let dir = project_tempdir();
    let notebook_path = dir.path().join("test.ipynb");

    let notebook_json = create_v4_notebook(&[("code", "x = 1")]);
    fs::write(&notebook_path, notebook_json).unwrap();

    let result = extract_notebook_cells(&notebook_path, None);
    assert!(result.is_ok());
    let cells = result.unwrap();
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].0, 0); // Index
    assert!(cells[0].1.contains("x = 1"));
}

#[test]
fn test_extract_notebook_cells_multiple_cells() {
    let dir = project_tempdir();
    let notebook_path = dir.path().join("test.ipynb");

    let notebook_json =
        create_v4_notebook(&[("code", "a = 1"), ("markdown", "text"), ("code", "b = 2")]);
    fs::write(&notebook_path, notebook_json).unwrap();

    let result = extract_notebook_cells(&notebook_path, None);
    assert!(result.is_ok());
    let cells = result.unwrap();
    // Should only return code cells with their original indices
    assert_eq!(cells.len(), 2);
    assert_eq!(cells[0].0, 0);
    assert_eq!(cells[1].0, 2); // Index 2 because markdown is at index 1
}

#[test]
fn test_extract_notebook_cells_nonexistent_file() {
    let result = extract_notebook_cells(std::path::Path::new("/nonexistent/path.ipynb"), None);
    assert!(result.is_err());
}

#[test]
fn test_extract_notebook_cells_invalid_json() {
    let dir = project_tempdir();
    let notebook_path = dir.path().join("invalid.ipynb");
    fs::write(&notebook_path, "{invalid").unwrap();

    let result = extract_notebook_cells(&notebook_path, None);
    assert!(result.is_err());
}
