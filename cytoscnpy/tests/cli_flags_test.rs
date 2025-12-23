//! Tests for CLI flags and options.
#![allow(clippy::unwrap_used)]

use cytoscnpy::commands::{run_cc, run_hal, run_mi, run_raw};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_cc_min_max() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    // Function with complexity 1 (Rank A)
    writeln!(file, "def foo(): pass").unwrap();
    // Function with complexity 6 (Rank B)
    writeln!(file, "def bar(x):\n    if x: pass\n    if x: pass\n    if x: pass\n    if x: pass\n    if x: pass").unwrap();

    let mut buffer = Vec::new();
    // Min rank B should exclude foo
    run_cc(
        dir.path(),
        cytoscnpy::commands::CcOptions {
            json: false,
            exclude: vec![],
            ignore: vec![],
            min_rank: Some('B'),
            output_file: None,
            ..Default::default()
        },
        &mut buffer,
    )
    .unwrap();
    let output = String::from_utf8(buffer).unwrap();
    assert!(!output.contains("foo"));
    assert!(output.contains("bar"));

    let mut buffer = Vec::new();
    // Max rank A should exclude bar
    run_cc(
        dir.path(),
        cytoscnpy::commands::CcOptions {
            json: false,
            exclude: vec![],
            ignore: vec![],
            max_rank: Some('A'),
            output_file: None,
            ..Default::default()
        },
        &mut buffer,
    )
    .unwrap();
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("foo"));
    assert!(!output.contains("bar"));
}

#[test]
fn test_cc_average() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "def foo(): pass\ndef bar(): pass").unwrap();

    let mut buffer = Vec::new();
    run_cc(
        dir.path(),
        cytoscnpy::commands::CcOptions {
            json: false,
            exclude: vec![],
            ignore: vec![],
            average: true,
            output_file: None,
            ..Default::default()
        },
        &mut buffer,
    )
    .unwrap();
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("Average complexity: 1.00"));
}

#[test]
fn test_mi_show() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "x = 1").unwrap();

    let mut buffer = Vec::new();
    run_mi(
        dir.path(),
        cytoscnpy::commands::MiOptions {
            json: false,
            exclude: vec![],
            ignore: vec![],
            show: true,
            output_file: None,
            ..Default::default()
        },
        &mut buffer,
    )
    .unwrap();
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("100.00")); // MI value
}

#[test]
fn test_hal_functions() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "def foo():\n    x = 1\n    y = 2").unwrap();

    let mut buffer = Vec::new();
    // With functions=true
    run_hal(dir.path(), false, vec![], vec![], true, None, &mut buffer).unwrap();
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("foo")); // Function name should be present
}

#[test]
fn test_raw_summary() {
    let dir = tempdir().unwrap();
    let file_path1 = dir.path().join("test1.py");
    let mut file1 = File::create(&file_path1).unwrap();
    writeln!(file1, "x = 1").unwrap();
    let file_path2 = dir.path().join("test2.py");
    let mut file2 = File::create(&file_path2).unwrap();
    writeln!(file2, "y = 2").unwrap();

    let mut buffer = Vec::new();
    run_raw(dir.path(), false, vec![], vec![], true, None, &mut buffer).unwrap();
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("Files"));
    assert!(output.contains('2')); // Total files
}
