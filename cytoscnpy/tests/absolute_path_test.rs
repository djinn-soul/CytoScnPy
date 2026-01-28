//! Regression test for absolute positional paths from different CWD.

// Test-specific lint suppressions
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use cytoscnpy::entry_point::run_with_args_to;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_absolute_path_from_different_cwd() -> anyhow::Result<()> {
    let project_dir = tempdir()?;
    let notebook_path = project_dir.path().join("test.ipynb");

    // Create a simple valid notebook
    let notebook_json = r#"{
        "cells": [
            {
                "cell_type": "code",
                "execution_count": null,
                "metadata": {},
                "outputs": [],
                "source": ["print('hello')"]
            }
        ],
        "metadata": {"kernelspec": {"display_name": "Python 3", "language": "python", "name": "python3"}},
        "nbformat": 4,
        "nbformat_minor": 4
    }"#;
    fs::write(&notebook_path, notebook_json)?;

    // Create a different directory for CWD using RAII guard
    let other_dir = tempdir()?;
    let _guard = cytoscnpy::test_utils::CwdGuard::new(other_dir.path())?;

    let mut buffer = Vec::new();
    // Provide absolute path to the notebook
    let args = vec![
        notebook_path.to_string_lossy().to_string(),
        "--include-ipynb".to_owned(),
    ];

    // This should NOT fail with "Path traversal detected" or similar
    let result = run_with_args_to(args, &mut buffer);

    // RAII guard will restore CWD automatically

    match result {
        Ok(code) => {
            if code != 0 {
                let output = String::from_utf8_lossy(&buffer);
                panic!("Command failed with code {code}. Output:\n{output}");
            }
        }
        Err(e) => {
            panic!("Command returned error: {e:?}");
        }
    }

    Ok(())
}

#[test]
fn test_absolute_path_fix_from_different_cwd() -> anyhow::Result<()> {
    let project_dir = tempdir()?;
    let file_path = project_dir.path().join("dead.py");
    fs::write(&file_path, "def unused():\n    pass\n")?;

    let other_dir = tempdir()?;
    let _guard = cytoscnpy::test_utils::CwdGuard::new(other_dir.path())?;

    let mut buffer = Vec::new();
    let args = vec![
        file_path.to_string_lossy().to_string(),
        "--fix".to_owned(),
        "--apply".to_owned(),
    ];

    let result = run_with_args_to(args, &mut buffer);
    // RAII guard restores CWD here

    assert!(
        result.is_ok(),
        "Fix --apply should work with absolute path from different CWD"
    );

    // Verify file was actually modified (fix applied)
    let content = fs::read_to_string(&file_path)?;
    assert!(
        !content.contains("def unused"),
        "Unused function should have been removed"
    );

    Ok(())
}
