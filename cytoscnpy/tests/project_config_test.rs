//! Editors scan files and nested projects one at a time; each scan must follow
//! the configuration of the project it belongs to.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use cytoscnpy::entry_point::run_with_args_to;
use std::fs;
use std::path::Path;

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn run(args: &[&str]) -> serde_json::Value {
    let mut out = Vec::new();
    run_with_args_to(args.iter().map(|&arg| arg.to_owned()).collect(), &mut out).unwrap();
    serde_json::from_slice(&out).unwrap()
}

fn unused_imports(result: &serde_json::Value) -> Vec<String> {
    let mut names: Vec<String> = result["unused_imports"]
        .as_array()
        .unwrap()
        .iter()
        .map(|def| def["simple_name"].as_str().unwrap().to_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn force_exclude_applies_folder_exclusions_to_named_files() {
    let dir = tempfile::tempdir().unwrap();
    write(
        &dir.path().join("pyproject.toml"),
        "[tool.cytoscnpy]\nexclude_folders = [\"legacy\"]\n",
    );
    let excluded = dir.path().join("legacy/old.py");
    let kept = dir.path().join("app/main.py");
    write(&excluded, "import os\n");
    write(&kept, "import sys\n");
    let excluded = excluded.to_str().unwrap();
    let kept = kept.to_str().unwrap();

    // A named file is analyzed as before unless the editor asks otherwise.
    assert_eq!(unused_imports(&run(&[excluded, "--json"])), ["os"]);
    assert_eq!(
        unused_imports(&run(&[excluded, "--json", "--force-exclude"])),
        Vec::<String>::new()
    );
    assert_eq!(
        unused_imports(&run(&[kept, "--json", "--force-exclude"])),
        ["sys"]
    );
}

#[test]
fn force_exclude_ignores_excluded_names_above_the_project() {
    // A project checked out under `build/` must not exclude itself.
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("build/project");
    write(&project.join("pyproject.toml"), "[tool.cytoscnpy]\n");
    let file = project.join("main.py");
    write(&file, "import os\n");
    assert_eq!(
        unused_imports(&run(&[file.to_str().unwrap(), "--json", "--force-exclude"])),
        ["os"]
    );
}

#[test]
fn projects_lists_folders_with_their_own_configuration() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        &root.join("pyproject.toml"),
        "[tool.cytoscnpy]\nexclude_folders = [\"skipme\"]\n",
    );
    write(
        &root.join("backend/pyproject.toml"),
        "[tool.cytoscnpy]\nconfidence = 90\n",
    );
    write(&root.join("lib/.cytoscnpy.toml"), "confidence = 50\n");
    // Not projects: no [tool.cytoscnpy] table, or in a folder scans skip.
    write(
        &root.join("frontend/pyproject.toml"),
        "[project]\nname = \"fe\"\n",
    );
    write(&root.join("broken/pyproject.toml"), "not = [valid");
    for skipped in [".venv/pkg", "node_modules/x", "skipme/inner"] {
        write(
            &root.join(skipped).join("pyproject.toml"),
            "[tool.cytoscnpy]\n",
        );
    }

    let listed = run(&["projects", root.to_str().unwrap(), "--json"]);
    let projects: Vec<&str> = listed["projects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|project| project.as_str().unwrap())
        .collect();
    let expected: Vec<String> = [root.to_path_buf(), root.join("backend"), root.join("lib")]
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    assert_eq!(projects, expected);
}

#[test]
fn a_nested_project_scanned_on_its_own_uses_its_configuration() {
    let dir = tempfile::tempdir().unwrap();
    let backend = dir.path().join("backend");
    write(
        &backend.join("pyproject.toml"),
        "[tool.cytoscnpy]\nexclude_folders = [\"generated\"]\n",
    );
    write(&backend.join("generated/models.py"), "import os\n");
    write(&backend.join("api.py"), "import sys\n");

    assert_eq!(
        unused_imports(&run(&[backend.to_str().unwrap(), "--json"])),
        ["sys"]
    );
}

#[test]
fn explicit_no_flags_override_scans_the_project_enables() {
    let dir = tempfile::tempdir().unwrap();
    write(
        &dir.path().join("pyproject.toml"),
        "[tool.cytoscnpy]\nsecrets = true\ndanger = true\nquality = true\nmax_args = 1\n\
         clones = true\ninclude_tests = true\ninclude_ipynb = true\n",
    );
    let body = "AWS_SECRET_ACCESS_KEY = 'wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY'\n\
                def f(a, b):\n    return eval(a + b)\n\
                def g(a, b):\n    return eval(a + b)\n";
    let clone =
        "def h(a, b):\n    x = a + b\n    y = x * 2\n    z = y - a\n    w = z + b\n    return w\n";
    write(&dir.path().join("app.py"), &format!("{body}{clone}"));
    write(&dir.path().join("copy.py"), &format!("{body}{clone}"));
    write(&dir.path().join("tests/test_x.py"), "import os\n");
    let notebook = serde_json::json!({
        "cells": [{"cell_type": "code", "execution_count": null, "id": "c", "metadata": {},
                   "outputs": [], "source": "import re\n"}],
        "metadata": {}, "nbformat": 4, "nbformat_minor": 5,
    });
    write(&dir.path().join("nb.ipynb"), &notebook.to_string());
    let target = dir.path().to_str().unwrap();

    let files = |result: &serde_json::Value| -> Vec<String> {
        result["unused_imports"]
            .as_array()
            .unwrap()
            .iter()
            .map(|def| def["file"].as_str().unwrap().replace('\\', "/"))
            .collect()
    };
    let count = |result: &serde_json::Value, key: &str| result[key].as_array().map_or(0, Vec::len);

    let on = run(&[target, "--json"]);
    for key in ["secrets", "danger", "quality", "clones"] {
        assert!(count(&on, key) > 0, "project config should enable {key}");
    }
    assert!(files(&on)
        .iter()
        .any(|file| file.ends_with("tests/test_x.py")));
    assert!(files(&on).iter().any(|file| file.ends_with("nb.ipynb")));

    let off = run(&[
        target,
        "--json",
        "--no-secrets",
        "--no-danger",
        "--no-quality",
        "--no-clones",
        "--no-include-tests",
        "--no-include-ipynb",
    ]);
    for key in ["secrets", "danger", "quality", "clones", "taint_findings"] {
        assert_eq!(count(&off, key), 0, "--no-{key} should disable it");
    }
    assert!(files(&off)
        .iter()
        .all(|file| !file.ends_with("test_x.py") && !file.ends_with("nb.ipynb")));

    // The last of a flag and its negation wins.
    let last = run(&[target, "--json", "--no-secrets", "--secrets"]);
    assert!(count(&last, "secrets") > 0);
}
