//! Overlapping inputs must behave like one scan of each physical source.

#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::entry_point::run_with_args_to;
use std::fs;
use tempfile::tempdir;

#[test]
fn overlapping_inputs_preserve_findings_and_metrics() {
    let dir = tempdir().unwrap();
    let nested = dir.path().join("src");
    fs::create_dir(&nested).unwrap();
    let file = nested.join("app.py");
    fs::write(&file, "import os\ndef unused():\n    return 1\n").unwrap();
    let baseline = CytoScnPy::default().analyze(dir.path());
    for paths in [
        vec![dir.path().to_path_buf(), file.clone()],
        vec![dir.path().to_path_buf(), nested.clone()],
        vec![dir.path().to_path_buf(), dir.path().to_path_buf()],
        vec![file, nested.join("./app.py")],
    ] {
        let actual = CytoScnPy::default().analyze_paths(&paths);
        assert!(actual.parse_errors.is_empty());
        assert_eq!(actual.analysis_summary.total_files, 1);
        assert_eq!(
            actual.unused_functions.len(),
            baseline.unused_functions.len()
        );
        assert_eq!(actual.unused_imports.len(), baseline.unused_imports.len());
        assert_eq!(
            actual.analysis_summary.total_definitions,
            baseline.analysis_summary.total_definitions
        );
        assert_eq!(
            actual.analysis_summary.total_lines_analyzed,
            baseline.analysis_summary.total_lines_analyzed
        );
        if paths[0].is_dir() {
            assert_eq!(
                actual.analysis_summary.total_directories,
                baseline.analysis_summary.total_directories
            );
        }
    }
}

#[test]
fn overlapping_inputs_produce_one_fix_plan_and_apply_once() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("app.py");
    fs::write(&file, "import os\ndef unused():\n    return 1\n").unwrap();
    for apply in [false, true] {
        let mut args = vec![
            dir.path().to_string_lossy().into_owned(),
            file.to_string_lossy().into_owned(),
            "--fix".to_owned(),
            "--json".to_owned(),
        ];
        if apply {
            args.push("--apply".to_owned());
        }
        let mut output = Vec::new();
        assert_eq!(run_with_args_to(args, &mut output).unwrap(), 0);
        let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
        let count = if apply {
            "applied_files"
        } else {
            "planned_files"
        };
        assert_eq!(report[count], 1);
        if apply {
            assert_eq!(report["items_removed"], 2);
            assert!(!fs::read_to_string(&file).unwrap().contains("unused"));
        } else {
            assert_eq!(report["planned_items"], 2);
        }
    }
}

#[test]
fn explicit_nested_directory_bypasses_parent_excludes() {
    let dir = tempdir().unwrap();
    let vendor = dir.path().join("vendor");
    fs::create_dir(&vendor).unwrap();
    fs::write(dir.path().join("app.py"), "def unused_app():\n    pass\n").unwrap();
    fs::write(vendor.join("lib.py"), "def unused_lib():\n    pass\n").unwrap();
    let analyzer = || CytoScnPy::default().with_excludes(vec!["vendor".to_owned()]);

    let parent_only = analyzer().analyze_paths(&[dir.path().to_path_buf()]);
    assert_eq!(parent_only.analysis_summary.total_files, 1);

    let both = analyzer().analyze_paths(&[dir.path().to_path_buf(), vendor]);
    assert!(both.parse_errors.is_empty());
    assert_eq!(both.analysis_summary.total_files, 2);
    assert_eq!(both.unused_functions.len(), 2);
}

#[test]
fn overlapping_inputs_measure_each_source_once() {
    let dir = tempdir().unwrap();
    let lib = dir.path().join("lib");
    fs::create_dir(&lib).unwrap();
    fs::write(lib.join("m.py"), "def f():\n    return 1\n").unwrap();
    let root = dir.path().to_string_lossy().into_owned();
    let nested = lib.to_string_lossy().into_owned();
    let run = |command: &str| {
        let mut output = Vec::new();
        let args = [command, &root, &nested, "--json"].map(str::to_owned);
        assert_eq!(run_with_args_to(args.to_vec(), &mut output).unwrap(), 0);
        serde_json::from_slice::<serde_json::Value>(&output).unwrap()
    };
    for command in ["raw", "cc", "hal", "mi", "files"] {
        assert_eq!(run(command).as_array().unwrap().len(), 1, "{command}");
    }
    let stats = run("stats");
    assert_eq!(stats["total_files"], 1);
    assert_eq!(stats["total_functions"], 1);
}

#[cfg(unix)]
#[test]
fn nested_input_symlinks_may_resolve_inside_enclosing_input() {
    let dir = tempdir().unwrap();
    let app = dir.path().join("app");
    let shared = dir.path().join("shared");
    fs::create_dir(&app).unwrap();
    fs::create_dir(&shared).unwrap();
    fs::write(shared.join("real.py"), "def unused_real():\n    pass\n").unwrap();
    std::os::unix::fs::symlink(shared.join("real.py"), app.join("link.py")).unwrap();

    let result = CytoScnPy::default().analyze_paths(&[app, dir.path().to_path_buf()]);
    assert!(result.parse_errors.is_empty());
    assert_eq!(result.analysis_summary.total_files, 1);
}

#[cfg(unix)]
#[test]
fn symlinked_sources_keep_project_relative_spelling() {
    let dir = tempdir().unwrap();
    let app = dir.path().join("app");
    let vendored = dir.path().join("vendored");
    fs::create_dir(&app).unwrap();
    fs::create_dir(&vendored).unwrap();
    fs::write(vendored.join("real.py"), "def unused_real():\n    pass\n").unwrap();
    std::os::unix::fs::symlink(vendored.join("real.py"), app.join("link.py")).unwrap();

    let result = CytoScnPy::default()
        .with_excludes(vec!["vendored".to_owned()])
        .analyze(dir.path());
    assert!(result.parse_errors.is_empty());
    let unused = &result.unused_functions[0];
    assert_eq!(unused.full_name, "vendored.real.unused_real");
    assert!(unused.file.starts_with(dir.path()));
}
