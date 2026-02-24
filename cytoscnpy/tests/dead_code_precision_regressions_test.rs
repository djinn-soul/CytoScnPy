//! Regression tests for dead-code precision improvements.
#![allow(clippy::unwrap_used, clippy::uninlined_format_args)]

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::config::ProjectType;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-precision-tmp");
    std::fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("precision_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[test]
fn method_attr_reference_does_not_mark_same_named_function_used() {
    let code = r#"
def process():
    return 1

class Runner:
    def process(self):
        return 2

Runner().process()
"#;

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    analyzer.config.cytoscnpy.project_type = Some(ProjectType::Application);
    let result = analyzer.analyze_code(code, std::path::Path::new("test.py"));

    let unused_functions: Vec<_> = result
        .unused_functions
        .iter()
        .map(|d| d.simple_name.as_str())
        .collect();

    assert!(
        unused_functions.contains(&"process"),
        "top-level process() should stay unused; got {:?}",
        unused_functions
    );
}

#[test]
fn dynamic_import_makes_application_module_reachable() {
    let dir = project_tempdir();
    let mut main_file = File::create(dir.path().join("main.py")).unwrap();
    let mut plugin_file = File::create(dir.path().join("plugin.py")).unwrap();

    writeln!(
        main_file,
        r#"
import importlib

importlib.import_module("plugin")
"#
    )
    .unwrap();

    writeln!(
        plugin_file,
        r#"
def plugin_entry():
    return 42
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    analyzer.config.cytoscnpy.project_type = Some(ProjectType::Application);
    let result = analyzer.analyze(dir.path());

    let plugin_entries: Vec<_> = result
        .unused_functions
        .iter()
        .filter(|d| d.file.to_string_lossy().ends_with("plugin.py"))
        .filter(|d| d.simple_name == "plugin_entry")
        .collect();

    assert!(
        plugin_entries.iter().all(|d| !d.is_unreachable),
        "plugin_entry should be reachable (not unreachable) via dynamic import in application mode; got {:?}",
        plugin_entries
    );
}
