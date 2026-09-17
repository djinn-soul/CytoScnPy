//! Integration tests for the `singletons` CLI subcommand and deslop integration.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;

#[test]
fn test_cli_singletons_clean_project() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("models.py"),
        concat!(
            "class User:\n",
            "    def __init__(self, username):\n",
            "        self.username = username\n",
        ),
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec!["singletons".to_owned(), root.to_string_lossy().into_owned()],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Singleton Pattern Scan Results"));
    assert!(output_str.contains("No singleton patterns detected"));
}

#[test]
fn test_cli_singletons_detects_patterns() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("services.py"),
        concat!(
            "@singleton\n",
            "class Cache:\n",
            "    pass\n\n",
            "class Database:\n",
            "    _instance = None\n",
            "    @classmethod\n",
            "    def get_instance(cls):\n",
            "        return cls._instance\n",
        ),
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec!["singletons".to_owned(), root.to_string_lossy().into_owned()],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Total singletons    : 2"));
    assert!(output_str.contains("class Cache"));
    assert!(output_str.contains("class Database"));
}

#[test]
fn test_cli_singletons_fail_on_any() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("bad.py"),
        concat!("class Registry(metaclass=Singleton):\n", "    pass\n",),
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "singletons".to_owned(),
            root.to_string_lossy().into_owned(),
            "--fail-on-any".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 1);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("[GATE] 1 singleton pattern(s) detected - FAILED"));
}

#[test]
fn test_cli_singletons_max_limit() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("two.py"),
        concat!(
            "@singleton\nclass One: pass\n\n",
            "@singleton\nclass Two: pass\n",
        ),
    )
    .unwrap();

    // Limit 2 passes
    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "singletons".to_owned(),
            root.to_string_lossy().into_owned(),
            "--max-singletons".to_owned(),
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
            "singletons".to_owned(),
            root.to_string_lossy().into_owned(),
            "--max-singletons".to_owned(),
            "1".to_owned(),
        ],
        &mut out2,
    )
    .unwrap();
    assert_eq!(code2, 1);
}

#[test]
fn test_cli_singletons_json_output() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("config.py"),
        "@singleton\nclass GlobalConfig: pass\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "singletons".to_owned(),
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
    assert_eq!(parsed["matches"][0]["class_name"], "GlobalConfig");
}

#[test]
fn test_cli_singletons_alias_singleton() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("clean.py"), "x = 1\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec!["singleton".to_owned(), root.to_string_lossy().into_owned()],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Singleton Pattern Scan Results"));
}
