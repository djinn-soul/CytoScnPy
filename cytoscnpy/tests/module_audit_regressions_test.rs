//! Behavioral contracts for the static-analysis module audit.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use cytoscnpy::entry_point::run_with_args_to;
use std::fs;
use std::path::Path;

#[test]
fn doctor_finds_the_project_for_files_and_package_subdirectories() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("pyproject.toml"),
        "[project]\nname='example'\nversion='1.0'\n",
    )
    .unwrap();
    fs::write(temp.path().join("README.md"), "# Example\n").unwrap();
    fs::create_dir(temp.path().join("src")).unwrap();
    let file = temp.path().join("src/app.py");
    fs::write(&file, "value = 1\n").unwrap();
    for target in [temp.path().join("src"), file] {
        let root = cytoscnpy::doctor::resolve_doctor_target(&target, temp.path());
        assert_eq!(root, temp.path().canonicalize().unwrap());
        let result =
            cytoscnpy::doctor::run_doctor(&root, &cytoscnpy::doctor::DoctorConfig::default());
        assert!(result.pyproject.is_some());
        let mut output = Vec::new();
        assert_eq!(
            run_with_args_to(
                vec![
                    "deslop".into(),
                    target.to_string_lossy().into_owned(),
                    "--json".into()
                ],
                &mut output
            )
            .unwrap(),
            0
        );
        let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(
            report["health"][0]["root_path"],
            root.to_string_lossy().as_ref()
        );
        assert_eq!(report["health"].as_array().unwrap().len(), 1);
    }
}

#[test]
fn scoring_and_gate_status_agree_for_passing_and_failing_reports() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("app.py"),
        "# TODO: replace placeholder\nvalue = 1\n",
    )
    .unwrap();
    for limit in [0, 100] {
        fs::write(
            temp.path().join("pyproject.toml"),
            format!("[tool.cytoscnpy.deslop]\nmax_todos = {limit}\n"),
        )
        .unwrap();
        let mut output = Vec::new();
        let mut args = vec![
            "deslop".into(),
            temp.path().to_string_lossy().into_owned(),
            "--json".into(),
        ];
        if limit == 0 {
            args.push("--fail-on-any".into());
        }
        let code = run_with_args_to(args, &mut output).unwrap();
        let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(report["scoring"]["passed_gate"], report["gates"]["passed"]);
        assert_eq!(report["gates"]["passed"], code == 0);
        assert_eq!(code, i32::from(limit == 0));
    }
}

#[test]
fn ordinary_new_method_local_variables_are_not_singleton_caches() {
    let ordinary = "class Widget:\n    def __new__(cls):\n        instance = super().__new__(cls)\n        instance.init_widget()\n        return instance\n";
    assert!(
        cytoscnpy::singletons::python::detect_python_singletons(ordinary, Path::new("app.py"))
            .is_empty()
    );
    let cached = "class Widget:\n    _instance = None\n    def __new__(cls):\n        if cls._instance is None:\n            cls._instance = super().__new__(cls)\n        return cls._instance\n";
    assert_eq!(
        cytoscnpy::singletons::python::detect_python_singletons(cached, Path::new("app.py")).len(),
        1
    );
}

#[test]
fn interface_methods_do_not_collide_but_module_functions_still_do() {
    let mut methods = Vec::new();
    let mut standalone = Vec::new();
    for name in ["a.py", "b.py", "c.py"] {
        methods.extend(
            cytoscnpy::searchability::functions::extract_functions_from_source(
                "class Serializer:\n    def to_dict(self):\n        return {}\n",
                Path::new(name),
            ),
        );
        standalone.extend(
            cytoscnpy::searchability::functions::extract_functions_from_source(
                "def to_dict():\n    return {}\n",
                Path::new(name),
            ),
        );
    }
    assert!(
        cytoscnpy::searchability::functions::find_function_collisions(&methods)
            .2
            .is_empty()
    );
    assert_eq!(
        cytoscnpy::searchability::functions::find_function_collisions(&standalone)
            .2
            .len(),
        1
    );
    let naming = cytoscnpy::naming::scanner::analyze_naming_functions(&methods);
    assert!(naming.outliers.is_empty());
}

#[test]
fn control_flow_comments_and_strings_do_not_count_as_callbacks() {
    let source = "class Service:\n    def run(self):\n        if ready:\n            for order in orders:\n                if order.valid:\n                    order.save()\n                    text = '''\n                    if imaginary:\n                        task.then(callback)\n                    '''\n                    # task.add_done_callback(callback)\n";
    assert!(
        cytoscnpy::anti_patterns::callbacks::detect_nested_callbacks(source, Path::new("app.py"))
            .is_empty()
    );
    let callbacks = "def start():\n    return lambda x: lambda y: lambda z: x + y + z\n";
    assert!(
        !cytoscnpy::anti_patterns::callbacks::detect_nested_callbacks(
            callbacks,
            Path::new("app.py")
        )
        .is_empty()
    );
}

#[test]
fn intra_file_helper_calls_and_script_entry_points_are_referenced() {
    let temp = tempfile::tempdir().unwrap();
    let helper = "def helper_function(value):\n    value += 1\n    value += 1\n    value += 1\n    return value\n";
    let options = cytoscnpy::unreferenced::UnreferencedOptions {
        min_lines: 4,
        min_name_len: 3,
        ..Default::default()
    };
    for name in [
        "utility.py",
        "main.py",
        "cli.py",
        "app.py",
        "manage.py",
        "setup.py",
    ] {
        let file = temp.path().join(name);
        fs::write(&file, format!("{helper}\nhelper_function(1)\n")).unwrap();
        let result = cytoscnpy::unreferenced::analyze_unreferenced_files(
            std::slice::from_ref(&file),
            &options,
        );
        assert!(result.items.is_empty(), "{name}");
    }
    let unused = temp.path().join("utility.py");
    fs::write(&unused, helper).unwrap();
    assert_eq!(
        cytoscnpy::unreferenced::analyze_unreferenced_files(&[unused], &options)
            .items
            .len(),
        1
    );
}
