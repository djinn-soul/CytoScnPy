//! Tests for CLI metrics output.
#![allow(clippy::unwrap_used)]

use cytoscnpy::commands::{run_cc, run_hal, run_mi, run_raw};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_cli_raw() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "x = 1\n# comment").unwrap();

    let mut buffer = Vec::new();
    run_raw(
        dir.path(),
        false,
        vec![],
        Vec::new(),
        false,
        None,
        &mut buffer,
    )
    .unwrap();

    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("test.py"));
    assert!(output.contains("LOC"));
    assert!(output.contains('3')); // LOC (x=1, #comment, newline)
    assert!(output.contains('1')); // SLOC/Comments
}

#[test]
fn test_cli_cc() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "def foo():\n    if True:\n        pass").unwrap();

    let mut buffer = Vec::new();
    run_cc(
        dir.path(),
        cytoscnpy::commands::CcOptions {
            json: false,
            exclude: vec![],
            ignore: Vec::new(),
            output_file: None,
            ..Default::default()
        },
        &mut buffer,
    )
    .unwrap();

    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("test.py"));
    assert!(output.contains("foo"));
    assert!(output.contains("function"));
    assert!(output.contains('A')); // Rank
}

#[test]
fn test_cli_hal() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "x = 1").unwrap();

    let mut buffer = Vec::new();
    run_hal(
        dir.path(),
        false,
        vec![],
        Vec::new(),
        false,
        None,
        &mut buffer,
    )
    .unwrap();

    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("test.py"));
    assert!(output.contains("Vocab"));
    // h1=1, h2=2, N1=1, N2=2
    // Vocab = 3.00
    // Volume = 3 * log2(3) = 3 * 1.58 = 4.75
    assert!(output.contains("3.00"));
    assert!(output.contains("4.75"));
}

#[test]
fn test_cli_mi() {
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
            ignore: Vec::new(),
            output_file: None,
            ..Default::default()
        },
        &mut buffer,
    )
    .unwrap();

    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("test.py"));
    assert!(output.contains("Rank"));
    assert!(output.contains('A'));
}

#[test]
fn test_cli_json_output() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "x = 1").unwrap();

    let mut buffer = Vec::new();
    run_raw(
        dir.path(),
        true,
        vec![],
        Vec::new(),
        false,
        None,
        &mut buffer,
    )
    .unwrap();

    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("\"file\":"));
    assert!(output.contains("\"loc\":"));
    assert!(output.contains('2'));
}
