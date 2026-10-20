//! Unit tests for repository structure scanning and test-ratio classification.

use std::fs;
use tempfile::TempDir;

use super::super::structure::scan_repo_structure;

#[test]
fn test_scan_repo_structure() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();

    fs::write(root.join("src/main.py"), "def run():\n    print('hello')\n").unwrap();
    fs::write(root.join("src/lib.py"), "x = 42\n").unwrap();
    fs::write(
        root.join("tests/test_main.py"),
        "def test_run():\n    assert True\n",
    )
    .unwrap();
    fs::write(root.join("README.md"), "# Title\n").unwrap();

    let stats = scan_repo_structure(root, &[]);

    assert_eq!(stats.total_files, 4);
    assert_eq!(stats.source_files, 2); // main.py, lib.py; README.md is not source
    assert_eq!(stats.test_files, 1); // test_main.py
    assert!(stats.test_to_source_ratio > 0.0);
    assert!(stats.languages.iter().any(|l| l.language == "Python"));
    assert!(stats.languages.iter().any(|l| l.language == "Markdown"));
    assert!(stats.max_directory_depth >= 2);
}

#[test]
fn test_ratio_ignores_ancestor_test_dirs_and_non_python_files() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("tests").join("project");
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("app.py"), "x = 1\n").unwrap();
    fs::write(root.join("test_app.py"), "def test_x():\n    pass\n").unwrap();
    fs::write(root.join("docs/test_plan.md"), "# Plan\n").unwrap();
    fs::write(root.join("data.json"), "{}\n").unwrap();

    let stats = scan_repo_structure(&root, &[]);
    assert_eq!(stats.total_files, 4);
    assert_eq!(stats.source_files, 1);
    assert_eq!(stats.test_files, 1);
    assert_eq!(stats.test_directories, vec![".".to_owned()]);
}

#[test]
fn test_scan_repo_structure_respects_exclusions() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("vendor")).unwrap();

    fs::write(root.join("src/main.py"), "def run(): pass\n").unwrap();
    fs::write(root.join("vendor/third_party.py"), "def ext(): pass\n").unwrap();

    let excludes = vec!["vendor".to_owned()];
    let stats = scan_repo_structure(root, &excludes);

    assert_eq!(stats.total_files, 1);
    assert_eq!(stats.source_files, 1);
}

#[test]
fn test_doctor_exclusions_do_not_overmatch_unrelated_paths() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let src_dir = root.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("contests.py"), "def contest(): pass\n").unwrap();
    fs::write(src_dir.join("library.py"), "def library(): pass\n").unwrap();

    let tests_dir = root.join("tests");
    fs::create_dir_all(&tests_dir).unwrap();
    fs::write(tests_dir.join("test_app.py"), "def test_app(): pass\n").unwrap();

    let stats = scan_repo_structure(root, &["tests".to_owned(), "lib".to_owned()]);
    // tests/test_app.py excluded; src/contests.py and src/library.py MUST be kept
    assert_eq!(stats.total_files, 2);
    assert_eq!(stats.source_files, 2);
    assert_eq!(stats.test_files, 0);
}

#[test]
fn test_doctor_exclusions_with_trailing_and_leading_slashes() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/main.py"), "def main(): pass\n").unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();
    fs::write(root.join("tests/test_main.py"), "def test_main(): pass\n").unwrap();

    let stats1 = scan_repo_structure(root, &["tests/".to_owned()]);
    assert_eq!(stats1.total_files, 1);
    let stats2 = scan_repo_structure(root, &["./tests".to_owned()]);
    assert_eq!(stats2.total_files, 1);
}

#[test]
fn test_scan_repo_structure_special_filenames() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("Makefile"), "all:\n\t@echo ok\n").unwrap();
    fs::write(
        root.join("Dockerfile"),
        "FROM python:3.11-slim\nCMD [\"python\"]\n",
    )
    .unwrap();
    fs::write(root.join("app.py"), "print('hello')\n").unwrap();
    fs::write(root.join("pyproject.toml"), "[project]\nname = \"demo\"\n").unwrap();

    let stats = scan_repo_structure(root, &[]);
    assert_eq!(stats.total_files, 4);

    let langs: std::collections::HashMap<_, _> = stats
        .languages
        .into_iter()
        .map(|l| (l.language, l.file_count))
        .collect();

    assert_eq!(langs.get("Shell"), Some(&1));
    assert_eq!(langs.get("Docker"), Some(&1));
    assert_eq!(langs.get("Python"), Some(&1));
    assert_eq!(langs.get("TOML"), Some(&1));
}

#[test]
fn test_largest_file_ignores_lockfiles_and_data() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("app.py"), "a = 1\nb = 2\n").unwrap();
    fs::write(root.join("package-lock.json"), "{\n}\n".repeat(2000)).unwrap();
    fs::write(root.join("notes.md"), "# Notes\n".repeat(1500)).unwrap();

    let stats = scan_repo_structure(root, &[]);
    assert_eq!(stats.largest_file_path, "app.py");
    assert_eq!(stats.largest_file_lines, 2);
}
