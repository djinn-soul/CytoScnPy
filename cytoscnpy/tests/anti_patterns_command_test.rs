//! Integration tests for the `anti-patterns` CLI subcommand.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;

#[test]
fn test_cli_anti_patterns_clean_project() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("clean.py"),
        concat!(
            "MAX_RETRIES = 500\n",
            "def add(a, b):\n",
            "    if a > 0:\n",
            "        return a + b\n",
            "    return b\n",
        ),
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "anti-patterns".to_owned(),
            root.to_string_lossy().into_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Anti-Pattern Scan Results"));
    assert!(output_str.contains("No anti-patterns detected"));
}

#[test]
fn test_cli_anti_patterns_detects_patterns() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("logic.py"),
        concat!(
            "def verify_status(code):\n",
            "    if code == 404:\n",
            "        for i in range(1):\n",
            "            while False:\n",
            "                if code > 500:\n",
            "                    return 'Server error'\n",
            "    return 'OK'\n",
        ),
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "anti-patterns".to_owned(),
            root.to_string_lossy().into_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("magic_number"));
    assert!(output_str.contains("deeply_nested_callback"));
}

#[test]
fn test_cli_anti_patterns_fail_on_any() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("bad.py"),
        concat!(
            "def check_limit(x):\n",
            "    if x > 300:\n",
            "        return False\n",
            "    return True\n",
        ),
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "anti-patterns".to_owned(),
            "--fail-on-any".to_owned(),
            root.to_string_lossy().into_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 1);
}

#[test]
fn test_cli_anti_patterns_max_limits() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("limits.py"),
        concat!(
            "def foo(x):\n",
            "    if x > 1000:\n",
            "        return 1\n",
            "    if x < 200:\n",
            "        return 2\n",
            "    return 0\n",
        ),
    )
    .unwrap();

    // Limit of 5 allows 2 magic numbers
    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "anti-patterns".to_owned(),
            "--max-magic-numbers".to_owned(),
            "5".to_owned(),
            root.to_string_lossy().into_owned(),
        ],
        &mut out,
    )
    .unwrap();
    assert_eq!(code, 0);

    // Limit of 1 fails on 2 magic numbers
    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "anti-patterns".to_owned(),
            "--max-magic-numbers".to_owned(),
            "1".to_owned(),
            root.to_string_lossy().into_owned(),
        ],
        &mut out,
    )
    .unwrap();
    assert_eq!(code, 1);
}

#[test]
fn test_cli_anti_patterns_json_output() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("data.py"),
        concat!(
            "def parse_port(port):\n",
            "    if port == 8080:\n",
            "        return True\n",
            "    return False\n",
        ),
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "anti-patterns".to_owned(),
            "--json".to_owned(),
            root.to_string_lossy().into_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    assert_eq!(json["stats"]["magic_numbers"], 1);
    assert_eq!(json["stats"]["total"], 1);
}

#[test]
fn test_cli_anti_patterns_aliases() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("test.py"), "def f():\n    pass\n").unwrap();

    for alias in ["antipatterns", "magic-numbers", "callbacks"] {
        let mut out = Cursor::new(Vec::new());
        let code = entry_point::run_with_args_to(
            vec![alias.to_owned(), root.to_string_lossy().into_owned()],
            &mut out,
        )
        .unwrap();
        assert_eq!(code, 0, "Alias '{alias}' failed with non-zero exit code");
    }
}

#[test]
fn test_cli_deslop_unified_includes_anti_patterns() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("bad.py"),
        concat!(
            "def check_status(code):\n",
            "    if code == 404:\n",
            "        return False\n",
            "    return True\n",
        ),
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
            "--no-git".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    assert!(parsed.get("anti_patterns").is_some());
    assert_eq!(parsed["anti_patterns"]["stats"]["magic_numbers"], 1);
}

#[test]
fn test_cli_deslop_max_anti_patterns_gate() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("bad.py"),
        concat!(
            "def check_status(code):\n",
            "    if code == 404:\n",
            "        return False\n",
            "    return True\n",
        ),
    )
    .unwrap();

    fs::write(
        root.join("pyproject.toml"),
        concat!(
            "[tool.cytoscnpy.deslop]\n",
            "min_health_score = 0\n",
            "max_anti_patterns = 0\n",
        ),
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            root.to_string_lossy().into_owned(),
            "--fail-on-any".to_owned(),
            "--no-git".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 1);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("- anti_patterns: 1 (limit <= 0)"));
}
