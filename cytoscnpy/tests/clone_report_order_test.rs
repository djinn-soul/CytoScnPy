//! Regression tests for clone sections embedded in the main text report.
#![allow(clippy::unwrap_used)]

use cytoscnpy::entry_point::run_with_args_to;
use std::fs;

fn clone_project() -> tempfile::TempDir {
    let target = std::env::current_dir().unwrap().join("target");
    fs::create_dir_all(&target).unwrap();
    let project = tempfile::Builder::new()
        .prefix("clone_report_")
        .tempdir_in(target)
        .unwrap();
    let source = r"def exact_copy(value):
    first = 1
    second = 2
    total = first + second
    return value * total
";
    fs::write(project.path().join("first.py"), source).unwrap();
    fs::write(project.path().join("second.py"), source).unwrap();
    project
}

#[test]
fn clone_section_follows_main_report_and_uses_singular_pair_label() {
    let project = clone_project();
    let mut buffer = Vec::new();

    let exit_code = run_with_args_to(
        vec![
            "--clones".to_owned(),
            project.path().to_string_lossy().into_owned(),
        ],
        &mut buffer,
    )
    .unwrap();

    assert_eq!(exit_code, 0);
    let output = String::from_utf8(buffer).unwrap();
    let analysis_heading = output.find("Python Static Analysis Results").unwrap();
    let clone_heading = output.find("Clone Detection Results").unwrap();

    assert!(analysis_heading < clone_heading, "{output}");
    assert!(output.contains("1 clone pair"), "{output}");
    assert!(!output.contains("1 clone pairs"), "{output}");
}

#[test]
fn project_config_enables_clone_section_without_cli_flag() {
    let project = clone_project();
    fs::write(
        project.path().join("pyproject.toml"),
        "[tool.cytoscnpy]\nclones = true\nclone_similarity = 0.8\n",
    )
    .unwrap();
    let mut buffer = Vec::new();

    let exit_code = run_with_args_to(
        vec![project.path().to_string_lossy().into_owned()],
        &mut buffer,
    )
    .unwrap();

    assert_eq!(exit_code, 0);
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("Clone Detection Results"), "{output}");
    assert!(output.contains("1 clone pair"), "{output}");
}
