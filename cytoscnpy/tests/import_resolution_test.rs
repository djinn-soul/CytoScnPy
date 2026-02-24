//! Tests for import resolution and cross-file analysis.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-import-tmp");
    std::fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("import_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[test]
fn test_cross_module_alias_resolution() {
    let dir = project_tempdir();

    // Create lib.py with a function
    let lib_path = dir.path().join("lib.py");
    let mut lib_file = File::create(&lib_path).unwrap();
    write!(lib_file, "def my_func(): pass").unwrap();

    // Create main.py that imports lib as l and uses l.my_func()
    let main_path = dir.path().join("main.py");
    let mut main_file = File::create(&main_path).unwrap();
    write!(
        main_file,
        r"
import lib as l
l.my_func()
"
    )
    .unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    // Check if my_func in lib.py is marked as unused
    // It SHOULD be marked as used because it's called via l.my_func()
    // But without alias resolution, l.my_func() doesn't map to lib.my_func()

    let unused_funcs: Vec<String> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    assert!(
        !unused_funcs.contains(&"my_func".to_owned()),
        "lib.my_func should be used via alias l.my_func"
    );
}

#[test]
fn test_from_import_resolution() {
    let dir = project_tempdir();

    // Create lib.py with a function
    let lib_path = dir.path().join("lib.py");
    let mut lib_file = File::create(&lib_path).unwrap();
    write!(lib_file, "def my_func(): pass").unwrap();

    // Create main.py that imports my_func from lib as f
    let main_path = dir.path().join("main.py");
    let mut main_file = File::create(&main_path).unwrap();
    write!(
        main_file,
        r"
from lib import my_func as f
f()
"
    )
    .unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    let unused_funcs: Vec<String> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    assert!(
        !unused_funcs.contains(&"my_func".to_owned()),
        "lib.my_func should be used via alias f"
    );
}

#[test]
fn test_chained_alias_resolution() {
    let dir = project_tempdir();

    // Create pandas.py (simulated)
    let lib_path = dir.path().join("pandas.py");
    let mut lib_file = File::create(&lib_path).unwrap();
    write!(lib_file, "def read_csv(): pass").unwrap();

    // Create main.py
    let main_path = dir.path().join("main.py");
    let mut main_file = File::create(&main_path).unwrap();
    write!(
        main_file,
        r#"
import pandas as pd
pd.read_csv("data.csv")
"#
    )
    .unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    let unused_funcs: Vec<String> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    // 'read_csv' should be marked as used because 'pd.read_csv' -> 'pandas.read_csv'
    assert!(
        !unused_funcs.contains(&"read_csv".to_owned()),
        "pandas.read_csv should be used via alias pd.read_csv"
    );
}

#[test]
fn test_relative_from_import_resolution_for_symbols() {
    let dir = project_tempdir();
    let pkg_dir = dir.path().join("tmc");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    File::create(pkg_dir.join("__init__.py")).unwrap();

    let mut am_file = File::create(pkg_dir.join("am.py")).unwrap();
    write!(
        am_file,
        r#"
FIELD_REGIRSTRY: dict[str, str] = {{"key": "value"}}

def helper() -> dict[str, str]:
    return FIELD_REGIRSTRY
"#
    )
    .unwrap();

    let mut ma_file = File::create(pkg_dir.join("ma.py")).unwrap();
    write!(
        ma_file,
        r#"
from .am import FIELD_REGIRSTRY, helper

def use_both() -> str:
    _data = helper()
    return FIELD_REGIRSTRY.get("key", "default")
"#
    )
    .unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    let unused_vars: Vec<String> = result
        .unused_variables
        .iter()
        .map(|v| v.simple_name.clone())
        .collect();
    let unused_funcs: Vec<String> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    assert!(
        !unused_vars.contains(&"FIELD_REGIRSTRY".to_owned()),
        "tmc.am.FIELD_REGIRSTRY should be used via relative import"
    );
    assert!(
        !unused_funcs.contains(&"helper".to_owned()),
        "tmc.am.helper should be used via relative import"
    );
}

#[test]
fn test_unreachable_private_functions_reported_at_default_confidence() {
    let dir = project_tempdir();
    let pkg_dir = dir.path().join("tmc");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    File::create(pkg_dir.join("__init__.py")).unwrap();

    let mut am_file = File::create(pkg_dir.join("am.py")).unwrap();
    write!(
        am_file,
        r#"
FIELD_REGIRSTRY: dict[str, str] = {{"key": "value"}}
"#
    )
    .unwrap();

    let mut ma_file = File::create(pkg_dir.join("ma.py")).unwrap();
    write!(
        ma_file,
        r#"
from .am import FIELD_REGIRSTRY

def _get_field_registry() -> dict[str, str]:
    return FIELD_REGIRSTRY

async def _process_single_field(field: str) -> str:
    registry = _get_field_registry()
    return registry.get(field, "default_value")
"#
    )
    .unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    let unused_funcs: Vec<String> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    assert!(
        unused_funcs.contains(&"_get_field_registry".to_owned()),
        "unreachable private helper should be reported at default confidence"
    );
    assert!(
        unused_funcs.contains(&"_process_single_field".to_owned()),
        "unreachable private async function should be reported at default confidence"
    );
}

#[test]
fn test_unreachable_private_functions_reported_even_above_private_penalty_threshold() {
    let dir = project_tempdir();
    let pkg_dir = dir.path().join("tmc");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    File::create(pkg_dir.join("__init__.py")).unwrap();

    let mut am_file = File::create(pkg_dir.join("am.py")).unwrap();
    write!(
        am_file,
        r#"
FIELD_REGIRSTRY: dict[str, str] = {{"key": "value"}}
"#
    )
    .unwrap();

    let mut ma_file = File::create(pkg_dir.join("ma.py")).unwrap();
    write!(
        ma_file,
        r#"
from .am import FIELD_REGIRSTRY

def _get_field_registry() -> dict[str, str]:
    return FIELD_REGIRSTRY

async def _process_single_field(field: str) -> str:
    registry = _get_field_registry()
    return registry.get(field, "default_value")
"#
    )
    .unwrap();

    // Private-name penalty drives these functions to confidence 20.
    // They should still be reported because reachability proves they are unreachable.
    let mut cytoscnpy = CytoScnPy::default().with_confidence(90).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    let unused_funcs: Vec<String> = result
        .unused_functions
        .iter()
        .map(|f| f.simple_name.clone())
        .collect();

    assert!(unused_funcs.contains(&"_get_field_registry".to_owned()));
    assert!(unused_funcs.contains(&"_process_single_field".to_owned()));
}
