use super::*;

/// Test that `analyze_paths` works with danger scanning on specific files
#[test]
fn test_analyze_paths_with_danger_scanning() {
    let dir = project_tempdir();

    // Create a file with dangerous code
    let file_path = dir.path().join("dangerous.py");
    let mut file = File::create(&file_path).unwrap();
    write!(
        file,
        r#"
import os
user_input = input()
os.system(user_input)
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default()
        .with_confidence(60)
        .with_danger(true)
        .with_tests(false);

    let paths = vec![file_path];
    let result = analyzer.analyze_paths(&paths);

    // Should find dangerous patterns
    assert!(result.analysis_summary.danger_count > 0);
}

/// Test that `analyze_paths` handles cross-file references
#[test]
fn test_analyze_paths_cross_file_references() {
    let dir = project_tempdir();

    // Create module.py with a function
    let module_path = dir.path().join("module.py");
    let mut module_file = File::create(&module_path).unwrap();
    write!(module_file, "def helper_function(): return 42").unwrap();

    // Create main.py that uses the function
    let main_path = dir.path().join("main.py");
    let mut main_file = File::create(&main_path).unwrap();
    write!(
        main_file,
        r#"
from module import helper_function
result = helper_function()
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);

    // Analyze both files
    let paths = vec![module_path, main_path];
    let result = analyzer.analyze_paths(&paths);

    let unused_funcs: Vec<String> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    // helper_function should NOT be unused (it's imported and called in main.py)
    assert!(!unused_funcs.contains(&"helper_function".to_owned()));
}

/// Test that `analyze_paths` includes notebooks when enabled
#[test]
fn test_analyze_paths_with_notebooks() {
    let dir = project_tempdir();

    // Create a simple notebook file
    let notebook_path = dir.path().join("notebook.ipynb");
    {
        let mut notebook = File::create(&notebook_path).unwrap();
        write!(
            notebook,
            r#"{{
        "cells": [
            {{
                "cell_type": "code",
                "source": ["def notebook_func(): pass"]
            }}
        ],
        "metadata": {{}},
        "nbformat": 4,
        "nbformat_minor": 4
    }}"#
        )
        .unwrap();
    }

    let mut analyzer = CytoScnPy::default()
        .with_confidence(60)
        .with_tests(false)
        .with_ipynb(true);

    let paths = vec![notebook_path];
    let result = analyzer.analyze_paths(&paths);

    // Should analyze the notebook
    assert_eq!(result.analysis_summary.total_files, 1);
}

/// Test pre-commit style usage: analyze only specific changed files
#[test]
fn test_analyze_paths_precommit_style() {
    let dir = project_tempdir();

    // Simulate a project with many files
    let files: Vec<PathBuf> = (0..5)
        .map(|i| {
            let path = dir.path().join(format!("file{i}.py"));
            let mut f = File::create(&path).unwrap();
            write!(f, "def func_in_file{i}(): pass").unwrap();
            f.sync_all().unwrap();
            path
        })
        .collect();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);

    // Simulate pre-commit: only analyze "changed" files (file1 and file3)
    let changed_files = vec![files[1].clone(), files[3].clone()];
    let result = analyzer.analyze_paths(&changed_files);

    // Should only analyze 2 files
    assert_eq!(result.analysis_summary.total_files, 2);

    let unused_funcs: Vec<String> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    // Should find unused functions from changed files only
    assert!(unused_funcs.contains(&"func_in_file1".to_owned()));
    assert!(unused_funcs.contains(&"func_in_file3".to_owned()));

    // Should NOT find functions from unchanged files
    assert!(!unused_funcs.contains(&"func_in_file0".to_owned()));
    assert!(!unused_funcs.contains(&"func_in_file2".to_owned()));
    assert!(!unused_funcs.contains(&"func_in_file4".to_owned()));
}

/// Regression: when the first path in `analyze_paths` is a test file (common in
/// pre-commit runs), `analysis_root` must not collapse to that file. Otherwise
/// `strip_prefix` returns an empty relative path and `is_test_path` mistakes
/// the test file for production code, allowing test-only references to mask
/// production dead code.
#[test]
fn test_analyze_paths_first_path_is_test_file_keeps_classification() {
    let dir = project_tempdir();
    let tests_dir = dir.path().join("tests");
    fs::create_dir_all(&tests_dir).unwrap();

    let test_path = tests_dir.join("test_thing.py");
    {
        let mut f = File::create(&test_path).unwrap();
        write!(
            f,
            r#"
from app import production_helper

def test_it():
    assert production_helper() == 1
"#
        )
        .unwrap();
        f.sync_all().unwrap();
    }

    let app_path = dir.path().join("app.py");
    {
        let mut f = File::create(&app_path).unwrap();
        write!(
            f,
            r#"
def production_helper():
    return 1
"#
        )
        .unwrap();
        f.sync_all().unwrap();
    }

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    // Order matters — putting the test file first is what used to break things.
    let result = analyzer.analyze_paths(&[test_path.clone(), app_path.clone()]);

    let unused_funcs: Vec<String> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    assert!(
        unused_funcs.contains(&"production_helper".to_owned()),
        "production_helper is only called from tests, so it should be flagged \
         unused in production; got {unused_funcs:?}"
    );
}

/// File-only analysis retains test directories inside the nearest project, but
/// not a `tests` directory containing that project (including Git worktrees).
#[test]
fn test_single_file_project_boundary_classification() {
    for marker in [".cytoscnpy.toml", "pyproject.toml", ".git"] {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("tests/project");
        fs::create_dir_all(project.join("tests")).unwrap();
        fs::create_dir_all(project.join("src")).unwrap();
        fs::write(project.join(marker), "").unwrap();
        let helper = project.join("tests/helpers.py");
        let production = project.join("src/main.py");
        for file in [&helper, &production] {
            fs::write(file, "assert True\n").unwrap();
        }

        for (file, is_test) in [(&helper, true), (&production, false)] {
            assert_eq!(
                cytoscnpy::utils::is_test_path_relative_to(file, file),
                is_test,
                "file root: {marker}, {file:?}",
            );
            for include_tests in [false, true] {
                let expected_files = usize::from(include_tests || !is_test);
                let mut analyzer = CytoScnPy::default()
                    .with_tests(include_tests)
                    .with_danger(true);
                let paths = [file.clone()];
                assert_eq!(analyzer.count_files(&paths), expected_files);
                let result = analyzer.analyze_paths(&paths);
                assert_eq!(analyzer.analysis_root, project);
                assert_eq!(result.analysis_summary.total_files, expected_files);
                assert!(result.parse_errors.is_empty());
                assert_eq!(result.danger.len(), usize::from(!is_test));

                // The single-path entry point must agree with analyze_paths.
                let direct = analyzer.analyze(file);
                assert_eq!(direct.analysis_summary.total_files, expected_files);
                assert_eq!(direct.danger.len(), result.danger.len());
            }
        }
    }
}

/// A test-directory input must be filtered relative to the shared project,
/// rather than stripping its own `tests` component before filtering.
#[test]
fn test_mixed_paths_share_classification_root() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("tests/project");
    let tests = project.join("tests");
    fs::create_dir_all(&tests).unwrap();
    let production = project.join("main.py");
    fs::write(&production, "assert True\n").unwrap();
    fs::write(tests.join("helpers.py"), "assert True\n").unwrap();

    for include_tests in [false, true] {
        for paths in [
            vec![tests.clone(), production.clone()],
            vec![production.clone(), tests.clone()],
            vec![project.clone()],
        ] {
            let mut analyzer = CytoScnPy::default()
                .with_tests(include_tests)
                .with_danger(true);
            let expected_files = 1 + usize::from(include_tests);
            assert_eq!(analyzer.count_files(&paths), expected_files);
            let result = analyzer.analyze_paths(&paths);
            assert_eq!(analyzer.analysis_root, project);
            assert_eq!(result.analysis_summary.total_files, expected_files);
            assert_eq!(result.danger.len(), 1);
        }
    }
}

/// Unmarked absolute paths cannot tell us whether an ancestor named `tests`
/// belongs to the project. Preserve the conservative containing-directory root.
#[test]
fn test_unmarked_single_file_root_is_conservative() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("tests/project");
    fs::create_dir_all(&project).unwrap();
    let production = project.join("main.py");
    fs::write(&production, "assert True\n").unwrap();
    let mut analyzer = CytoScnPy::default().with_tests(false).with_danger(true);
    assert_eq!(analyzer.count_files(std::slice::from_ref(&production)), 1);
    let result = analyzer.analyze_paths(&[production]);
    assert_eq!(analyzer.analysis_root, project);
    assert_eq!(result.analysis_summary.total_files, 1);
    assert_eq!(result.danger.len(), 1);
}
