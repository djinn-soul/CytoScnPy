//! Tests for import resolution and cross-file analysis.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_cross_module_alias_resolution() {
    let dir = tempdir().unwrap();

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
    let dir = tempdir().unwrap();

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
    let dir = tempdir().unwrap();

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
