//! Integration tests for the `functions` CLI subcommand.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;
use cytoscnpy::functions::FunctionsResult;

#[test]
fn test_cli_functions_basic_extraction() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("example.py"),
        concat!(
            "def simple_fn(a, b):\n",
            "    return a + b\n\n",
            "class Calculator:\n",
            "    def compute(self, x):\n",
            "        if x > 0:\n",
            "            return x * 2\n",
            "        return 0\n",
        ),
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec!["functions".to_owned(), root.to_string_lossy().into_owned()],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Function Metrics & Extraction Results"));
    assert!(output_str.contains("simple_fn"));
    assert!(output_str.contains("Calculator.compute"));
    assert!(output_str.contains("Total functions     : 2"));
}

#[test]
fn test_cli_functions_json_output() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("logic.py"),
        concat!(
            "def complex_logic(x, y):\n",
            "    if x > 0:\n",
            "        for i in range(x):\n",
            "            if y > i:\n",
            "                return True\n",
            "    return False\n",
        ),
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "functions".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    let res: FunctionsResult = serde_json::from_str(&output_str).unwrap();

    assert_eq!(res.functions.len(), 1);
    let f = &res.functions[0];
    assert_eq!(f.name, "complex_logic");
    assert_eq!(f.line_count, 6);
    assert_eq!(f.max_nesting, 3); // if -> for -> if
    assert!(f.cyclomatic_complexity >= 3);
    assert_eq!(res.stats.total_functions, 1);
    assert_eq!(res.stats.max_nesting, 3);
}

#[test]
fn test_cli_functions_threshold_failure() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("deep.py"),
        concat!(
            "def very_deep(x):\n",
            "    if x:\n",
            "        if x > 1:\n",
            "            if x > 2:\n",
            "                if x > 3:\n",
            "                    return x\n",
        ),
    )
    .unwrap();

    // With max-nesting 2, deep function (nesting 4) fails with code 1
    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "functions".to_owned(),
            root.to_string_lossy().into_owned(),
            "--max-nesting".to_owned(),
            "2".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 1);

    // With max-nesting 5, it passes with code 0
    let mut out_pass = Cursor::new(Vec::new());
    let code_pass = entry_point::run_with_args_to(
        vec![
            "functions".to_owned(),
            root.to_string_lossy().into_owned(),
            "--max-nesting".to_owned(),
            "5".to_owned(),
        ],
        &mut out_pass,
    )
    .unwrap();

    assert_eq!(code_pass, 0);
}

#[test]
fn test_cli_functions_aliases() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("dummy.py"), "def f():\n    pass\n").unwrap();

    for alias in &["fns", "func"] {
        let mut out = Cursor::new(Vec::new());
        let code = entry_point::run_with_args_to(
            vec![(*alias).to_owned(), root.to_string_lossy().into_owned()],
            &mut out,
        )
        .unwrap();
        assert_eq!(code, 0);
        let output_str = String::from_utf8(out.into_inner()).unwrap();
        assert!(output_str.contains("Function Metrics & Extraction Results"));
    }
}

#[test]
fn test_cli_functions_output_file() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let out_json_path = root.join("funcs.json");

    fs::write(root.join("app.py"), "def hello():\n    return 42\n").unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "functions".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
            "-o".to_owned(),
            out_json_path.to_string_lossy().into_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    assert!(out_json_path.exists());
    let content = fs::read_to_string(&out_json_path).unwrap();
    let res: FunctionsResult = serde_json::from_str(&content).unwrap();
    assert_eq!(res.functions.len(), 1);
    assert_eq!(res.functions[0].name, "hello");
}
