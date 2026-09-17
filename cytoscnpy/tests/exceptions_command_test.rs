//! Integration tests for the `exceptions` CLI subcommand and deslop integration.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;

#[test]
fn test_cli_exceptions_clean_project() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("clean.py"),
        concat!(
            "def calculate(a, b):\n",
            "    try:\n",
            "        return a / b\n",
            "    except ZeroDivisionError as e:\n",
            "        logger.error(f'Division by zero: {e}')\n",
            "        return 0\n",
        ),
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec!["exceptions".to_owned(), root.to_string_lossy().into_owned()],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Exception Handler Anti-Pattern Results"));
    assert!(output_str.contains("No bare-except or empty exception handlers detected"));
}

#[test]
fn test_cli_exceptions_detects_patterns() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("bad.py"),
        concat!(
            "def risky():\n",
            "    try:\n",
            "        do_something()\n",
            "    except:\n",
            "        print('bare')\n",
            "    try:\n",
            "        other()\n",
            "    except ValueError:\n",
            "        pass\n",
        ),
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec!["exceptions".to_owned(), root.to_string_lossy().into_owned()],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0); // without --fail-on-any
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Total anti-patterns : 2"));
    assert!(output_str.contains("bare_except"));
    assert!(output_str.contains("empty_handler"));
}

#[test]
fn test_cli_exceptions_fail_on_any() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("bad.py"),
        "try:\n    risky()\nexcept:\n    pass\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "exceptions".to_owned(),
            root.to_string_lossy().into_owned(),
            "--fail-on-any".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 1);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("[GATE] 1 exception anti-pattern(s) detected - FAILED"));
}

#[test]
fn test_cli_exceptions_alias_bare_except() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("bad.py"),
        "try:\n    run()\nexcept:\n    print('error')\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "bare-except".to_owned(),
            root.to_string_lossy().into_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Exception Handler Anti-Pattern Results"));
    assert!(output_str.contains("bare_except"));
}

#[test]
fn test_cli_exceptions_max_bare_excepts() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("sample.py"),
        "try:\n    a()\nexcept:\n    handle()\n",
    )
    .unwrap();

    // Limit 1 passes
    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "exceptions".to_owned(),
            root.to_string_lossy().into_owned(),
            "--max-bare-excepts".to_owned(),
            "1".to_owned(),
        ],
        &mut out,
    )
    .unwrap();
    assert_eq!(code, 0);

    // Limit 0 fails
    let mut out2 = Cursor::new(Vec::new());
    let code2 = entry_point::run_with_args_to(
        vec![
            "exceptions".to_owned(),
            root.to_string_lossy().into_owned(),
            "--max-bare-excepts".to_owned(),
            "0".to_owned(),
        ],
        &mut out2,
    )
    .unwrap();
    assert_eq!(code2, 1);
}

#[test]
fn test_cli_exceptions_max_empty_handlers() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("sample.py"),
        "try:\n    a()\nexcept ValueError:\n    pass\n",
    )
    .unwrap();

    // Limit 1 passes
    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "exceptions".to_owned(),
            root.to_string_lossy().into_owned(),
            "--max-empty-handlers".to_owned(),
            "1".to_owned(),
        ],
        &mut out,
    )
    .unwrap();
    assert_eq!(code, 0);

    // Limit 0 fails
    let mut out2 = Cursor::new(Vec::new());
    let code2 = entry_point::run_with_args_to(
        vec![
            "exceptions".to_owned(),
            root.to_string_lossy().into_owned(),
            "--max-empty-handlers".to_owned(),
            "0".to_owned(),
        ],
        &mut out2,
    )
    .unwrap();
    assert_eq!(code2, 1);
}

#[test]
fn test_cli_exceptions_json_output() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("bad.py"), "try:\n    a()\nexcept:\n    pass\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "exceptions".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    assert!(parsed.get("stats").is_some());
    assert_eq!(parsed["stats"]["bare_except_count"], 1);
}

#[test]
fn test_cli_deslop_unified_includes_exceptions() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("bad.py"), "try:\n    f()\nexcept:\n    pass\n").unwrap();

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
    assert!(parsed.get("exceptions").is_some());
    assert_eq!(parsed["exceptions"]["stats"]["bare_except_count"], 1);
}

#[test]
fn test_cli_deslop_max_exceptions_gate() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("bad.py"), "try:\n    f()\nexcept:\n    pass\n").unwrap();

    fs::write(
        root.join("pyproject.toml"),
        concat!(
            "[tool.cytoscnpy.deslop]\n",
            "min_health_score = 0\n",
            "max_bare_excepts = 0\n",
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
    assert!(output_str.contains("- bare_excepts: 1 (limit <= 0)"));
}
