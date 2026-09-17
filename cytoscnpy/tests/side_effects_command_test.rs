//! Integration tests for the `side-effects` CLI subcommand and deslop integration.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;

#[test]
fn test_cli_side_effects_clean_project() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("pure.py"),
        concat!(
            "import os\n",
            "import logging\n",
            "logger = logging.getLogger(__name__)\n",
            "def compute(a, b):\n",
            "    return a + b\n",
            "if __name__ == '__main__':\n",
            "    compute(1, 2)\n",
        ),
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "side-effects".to_owned(),
            root.to_string_lossy().into_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Module-Level Side Effects Scan Results"));
    assert!(output_str.contains("No module-level import-time side effects detected"));
}

#[test]
fn test_cli_side_effects_detects_python_calls_and_loops() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("impure.py"),
        concat!(
            "def worker():\n",
            "    pass\n",
            "setup_database('postgres://localhost')\n",
            "for i in range(5):\n",
            "    warm_cache(i)\n",
        ),
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "side-effects".to_owned(),
            root.to_string_lossy().into_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0); // without --fail-on-any
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Total side effects  : 2"));
    assert!(output_str.contains("python_toplevel_call"));
    assert!(output_str.contains("python_toplevel_loop"));
}

#[test]
fn test_cli_side_effects_fail_on_any() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("bad.py"), "connect_remote_server()\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "side-effects".to_owned(),
            root.to_string_lossy().into_owned(),
            "--fail-on-any".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 1);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("[GATE] 1 module-level side effect(s) detected - FAILED"));
}

#[test]
fn test_cli_side_effects_max_limit() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("two.py"), "do_step_one()\ndo_step_two()\n").unwrap();

    // Limit 2 passes
    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "side-effects".to_owned(),
            root.to_string_lossy().into_owned(),
            "--max-side-effects".to_owned(),
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
            "side-effects".to_owned(),
            root.to_string_lossy().into_owned(),
            "--max-side-effects".to_owned(),
            "1".to_owned(),
        ],
        &mut out2,
    )
    .unwrap();
    assert_eq!(code2, 1);
}

#[test]
fn test_cli_side_effects_json_output() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("script.py"), "sync_telemetry()\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "side-effects".to_owned(),
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
    assert_eq!(parsed["stats"]["python_calls"], 1);
}

#[test]
fn test_cli_side_effects_polyglot_js() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("server.js"),
        "const express = require('express');\nconst app = express();\napp.use(express.json());\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "side-effects".to_owned(),
            root.to_string_lossy().into_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("js_toplevel_call"));
}
