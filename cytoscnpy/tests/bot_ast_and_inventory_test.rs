//! Regression coverage for guards, test selection, and dependency groups.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use cytoscnpy::entry_point::run_with_args_to;
use std::fs;
use std::path::Path;

#[test]
fn only_exact_main_equalities_suppress_import_time_findings() {
    for condition in ["__name__ == '__main__'", "'__main__' == __name__"] {
        let source = format!("if {condition}:\n    STATE = []\n    sys.exit(1)\n");
        assert!(
            cytoscnpy::globals::python::detect_python_globals(&source, Path::new("app.py"))
                .is_empty()
        );
        assert!(cytoscnpy::side_effects::python::detect_python_side_effects(
            &source,
            Path::new("app.py")
        )
        .is_empty());
    }
    for condition in [
        "__name__ != '__main__'",
        "__name__ == 'app'",
        "'app' == __name__",
        "__name__ in ['app']",
        "__name__ == '__main__' == 'app'",
    ] {
        let source = format!("if {condition}:\n    STATE = []\n    sys.exit(1)\n");
        assert_eq!(
            cytoscnpy::globals::python::detect_python_globals(&source, Path::new("app.py")).len(),
            1,
            "{condition}"
        );
        assert_eq!(
            cytoscnpy::side_effects::python::detect_python_side_effects(
                &source,
                Path::new("app.py")
            )
            .len(),
            1,
            "{condition}"
        );
    }
}

#[test]
fn sys_calls_are_not_blanket_safe() {
    let source = "import sys\nsys.exit(1)\nsys.settrace(callback)\n";
    assert_eq!(
        cytoscnpy::side_effects::python::detect_python_side_effects(source, Path::new("app.py"))
            .len(),
        2
    );
}

#[test]
fn deslop_test_selection_applies_to_shared_inventory_and_context() {
    for from_config in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("tests")).unwrap();
        fs::write(temp.path().join("app.py"), "def run():\n    return 1\n").unwrap();
        fs::write(
            temp.path().join("tests/test_app.py"),
            "def test_run():\n    return 1\n",
        )
        .unwrap();
        for include_tests in [false, true] {
            let mut args = vec![
                "deslop".into(),
                temp.path().to_string_lossy().into_owned(),
                "--json".into(),
            ];
            if from_config {
                fs::write(
                    temp.path().join("pyproject.toml"),
                    format!("[tool.cytoscnpy]\ninclude_tests = {include_tests}\n"),
                )
                .unwrap();
            } else if include_tests {
                args.insert(0, "--include-tests".into());
            }
            let mut output = Vec::new();
            assert_eq!(run_with_args_to(args, &mut output).unwrap(), 0);
            let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
            let count = if include_tests { 2 } else { 1 };
            assert_eq!(report["scan_integrity"]["files_discovered"], count);
            assert_eq!(report["context"]["total_files"], count);
            assert_eq!(
                report["architecture"]["nodes"].as_array().unwrap().len(),
                count
            );
            assert_eq!(report["functions"]["files_scanned"], count);
        }
    }
}

#[test]
fn pdm_groups_contain_requirements_not_group_names() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("pyproject.toml");
    fs::write(
        &path,
        r#"[tool.pdm.dev-dependencies]
test = ['pytest>=9', 'coverage[toml]>=7; python_version >= "3.11"']
lint = ['ruff>=0.16']
[tool.poetry.dev-dependencies]
mypy = '*'
"#,
    )
    .unwrap();
    let dependencies = cytoscnpy::deps::declared::parse_pyproject(&path);
    let names: std::collections::BTreeSet<_> = dependencies
        .iter()
        .map(|dep| dep.normalized_name.as_str())
        .collect();
    assert_eq!(
        names,
        ["coverage", "mypy", "pytest", "ruff"].into_iter().collect()
    );
    assert!(dependencies.iter().all(|dep| dep.is_dev));
    assert!(dependencies
        .iter()
        .find(|dep| dep.normalized_name == "coverage")
        .unwrap()
        .marker
        .is_some());
}

#[test]
fn main_guard_else_branch_still_runs_during_import() {
    let source = "if __name__ == '__main__':\n    pass\nelse:\n    STATE = []\n    sys.exit(1)\n";
    assert_eq!(
        cytoscnpy::globals::python::detect_python_globals(source, Path::new("app.py")).len(),
        1
    );
    assert_eq!(
        cytoscnpy::side_effects::python::detect_python_side_effects(source, Path::new("app.py"))
            .len(),
        1
    );
}
