//! Integration tests for the `wildcards` CLI subcommand and deslop integration.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;

#[test]
fn test_cli_wildcards_clean_project() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("engine.py"),
        "import os\nfrom sys import argv\ndef run():\n    return 42\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec!["wildcards".to_owned(), root.to_string_lossy().into_owned()],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Wildcard Import Scan Results"));
    assert!(output_str.contains("No wildcard imports"));
}

#[test]
fn test_cli_wildcards_detects_star_imports() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("bad.py"),
        "from os.path import *\nfrom typing import *\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec!["wildcards".to_owned(), root.to_string_lossy().into_owned()],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0); // without --fail-on-any
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Total wildcard imports : 2"));
    assert!(output_str.contains("from os.path import *"));
    assert!(output_str.contains("from typing import *"));
}

#[test]
fn test_cli_wildcards_fail_on_any() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("bad.py"), "from math import *\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "wildcards".to_owned(),
            root.to_string_lossy().into_owned(),
            "--fail-on-any".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 1);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("[GATE] 1 wildcard import(s) detected - FAILED"));
}

#[test]
fn test_cli_wildcards_max_limit() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("two.py"), "from os import *\nfrom sys import *\n").unwrap();

    // Limit 2 passes
    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "wildcards".to_owned(),
            root.to_string_lossy().into_owned(),
            "--max-wildcards".to_owned(),
            "2".to_owned(),
        ],
        &mut out,
    )
    .unwrap();
    assert_eq!(code, 0);

    // Limit 1 fails
    let mut out2 = Cursor::new(Vec::new());
    let code2 = entry_point::run_with_args_to(
        vec![
            "wildcards".to_owned(),
            root.to_string_lossy().into_owned(),
            "--max-wildcards".to_owned(),
            "1".to_owned(),
        ],
        &mut out2,
    )
    .unwrap();
    assert_eq!(code2, 1);
}

#[test]
fn test_cli_wildcards_json_output() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("calc.py"), "from math import *\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "wildcards".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    assert_eq!(parsed["stats"]["total"], 1);
    assert_eq!(parsed["matches"][0]["module"], "math");
}

#[test]
fn test_cli_wildcards_alias_star_imports() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("clean.py"), "x = 10\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "star-imports".to_owned(),
            root.to_string_lossy().into_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Wildcard Import Scan Results"));
}
