//! Integration tests for the `graph` CLI subcommand.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;

#[test]
fn test_cli_graph_clean_project() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let app_dir = root.join("app");
    fs::create_dir_all(&app_dir).unwrap();
    fs::write(app_dir.join("__init__.py"), "").unwrap();
    fs::write(app_dir.join("models.py"), "class User: pass\n").unwrap();
    fs::write(
        app_dir.join("services.py"),
        "from app.models import User\ndef get_user(): return User()\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "graph".to_owned(),
            root.to_string_lossy().into_owned(),
            "--fail-on-cycles".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("No circular module dependencies detected"));
    assert!(output_str.contains("Total Modules:"));
}

#[test]
fn test_cli_graph_circular_dependency_gate_failure() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let pkg = root.join("cycle_pkg");
    fs::create_dir_all(&pkg).unwrap();
    fs::write(pkg.join("__init__.py"), "").unwrap();
    fs::write(
        pkg.join("alpha.py"),
        "from cycle_pkg.beta import b_fn\ndef a_fn(): return b_fn()\n",
    )
    .unwrap();
    fs::write(
        pkg.join("beta.py"),
        "from cycle_pkg.alpha import a_fn\ndef b_fn(): return a_fn()\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "graph".to_owned(),
            root.to_string_lossy().into_owned(),
            "--fail-on-cycles".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    // Should fail with exit code 1 due to --fail-on-cycles
    assert_eq!(code, 1);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("circular dependency component(s)"));
    assert!(output_str.contains("Cycle #1"));
    assert!(output_str.contains("[GATE] Circular dependencies: 1 cycle(s) found - FAILED"));
}

#[test]
fn test_cli_graph_json_output() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let pkg = root.join("json_pkg");
    fs::create_dir_all(&pkg).unwrap();
    fs::write(pkg.join("__init__.py"), "").unwrap();
    fs::write(pkg.join("util.py"), "import json\ndef load(): pass\n").unwrap();
    fs::write(
        pkg.join("main.py"),
        "from json_pkg.util import load\nload()\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "graph".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&output_str).expect("Output must be valid JSON");

    assert!(parsed.get("stats").is_some());
    assert!(parsed.get("nodes").is_some());
    assert!(parsed.get("edges").is_some());
    assert!(parsed.get("external_imports").is_some());
    assert_eq!(parsed["stats"]["total_modules"], 3);
    assert_eq!(parsed["stats"]["circular_dependency_count"], 0);
}

#[test]
fn test_cli_graph_cycles_only_flag() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let pkg = root.join("demo_pkg");
    fs::create_dir_all(&pkg).unwrap();
    fs::write(pkg.join("__init__.py"), "").unwrap();
    fs::write(pkg.join("a.py"), "x = 1\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "graph".to_owned(),
            root.to_string_lossy().into_owned(),
            "--cycles-only".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(!output_str.contains("Repository Metrics:"));
    assert!(output_str.contains("No circular module dependencies detected"));
}

#[test]
fn test_cli_graph_god_module_gate_failure() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let pkg = root.join("hub_pkg");
    fs::create_dir_all(&pkg).unwrap();
    fs::write(pkg.join("__init__.py"), "").unwrap();
    fs::write(pkg.join("god.py"), "def shared(): pass\n").unwrap();

    // Create 6 modules importing god.py (total modules = 8, 6/7 = 85% > 40%)
    for i in 0..6 {
        fs::write(
            pkg.join(format!("m{i}.py")),
            "from hub_pkg.god import shared\n",
        )
        .unwrap();
    }

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "graph".to_owned(),
            root.to_string_lossy().into_owned(),
            "--fail-on-god-modules".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 1);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("potential God Module(s)"));
    assert!(output_str.contains("hub_pkg.god"));
    assert!(output_str.contains("[GATE] God modules: 1 module(s) found - FAILED"));
}
