//! Integration tests for the `unreferenced` CLI subcommand and deslop integration.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;

#[test]
fn test_cli_unreferenced_clean_project() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("entrypoint.py"),
        concat!(
            "def calculate_total_sales(orders):\n",
            "    total = 0\n",
            "    for o in orders:\n",
            "        total += o.price\n",
            "    return total\n",
        ),
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "unreferenced".to_owned(),
            root.to_string_lossy().into_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Unreferenced Large Functions in Isolated Files"));
    assert!(output_str.contains("No unreferenced large functions detected"));
}

#[test]
fn test_cli_unreferenced_detects_functions() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Active file that is imported/connected
    fs::write(
        root.join("main.py"),
        "import app_module\n\ndef run():\n    app_module.start()\n",
    )
    .unwrap();

    fs::write(
        root.join("app_module.py"),
        "def start():\n    print('started')\n",
    )
    .unwrap();

    // Isolated file never imported or mentioned
    let mut dead_code =
        String::from("def calculate_legacy_financial_batch(records, fiscal_year):\n");
    for _ in 0..20 {
        dead_code.push_str("    record_val = records.get(0, 0) * fiscal_year\n");
    }
    dead_code.push_str("    return record_val\n");

    fs::write(root.join("old_accounting_system.py"), dead_code).unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "unreferenced".to_owned(),
            root.to_string_lossy().into_owned(),
            "--min-lines".to_owned(),
            "15".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Unreferenced functions : 1"));
    assert!(output_str.contains("calculate_legacy_financial_batch"));
    assert!(output_str.contains("old_accounting_system.py"));
}

#[test]
fn test_cli_unreferenced_fail_on_any() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let mut dead_code = String::from("def calculate_redundant_score_table(dataset):\n");
    for _ in 0..20 {
        dead_code.push_str("    step_val = dataset.value * 42\n");
    }
    dead_code.push_str("    return step_val\n");

    fs::write(root.join("abandoned_module.py"), dead_code).unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "unreferenced".to_owned(),
            root.to_string_lossy().into_owned(),
            "--min-lines".to_owned(),
            "15".to_owned(),
            "--fail-on-any".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 1);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("[GATE] 1 unreferenced large function(s)"));
}

#[test]
fn test_cli_unreferenced_max_limits() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let mut dead_code = String::from("def compute_internal_deprecations(items):\n");
    for _ in 0..20 {
        dead_code.push_str("    item_val = items.cost * 42\n");
    }
    dead_code.push_str("    return item_val\n");

    fs::write(root.join("unused_calc.py"), dead_code).unwrap();

    // Limit 1 passes
    let mut out1 = Cursor::new(Vec::new());
    let code1 = entry_point::run_with_args_to(
        vec![
            "unreferenced".to_owned(),
            root.to_string_lossy().into_owned(),
            "--min-lines".to_owned(),
            "15".to_owned(),
            "--max-unreferenced".to_owned(),
            "1".to_owned(),
        ],
        &mut out1,
    )
    .unwrap();
    assert_eq!(code1, 0);

    // Limit 0 fails
    let mut out2 = Cursor::new(Vec::new());
    let code2 = entry_point::run_with_args_to(
        vec![
            "unreferenced".to_owned(),
            root.to_string_lossy().into_owned(),
            "--min-lines".to_owned(),
            "15".to_owned(),
            "--max-unreferenced".to_owned(),
            "0".to_owned(),
        ],
        &mut out2,
    )
    .unwrap();
    assert_eq!(code2, 1);
}

#[test]
fn test_cli_unreferenced_json_output() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let mut dead_code = String::from("def extract_custom_audit_history(payload):\n");
    for _ in 0..20 {
        dead_code.push_str("    entry_val = payload.meta[0]\n");
    }
    dead_code.push_str("    return entry_val\n");

    fs::write(root.join("dead_audit.py"), dead_code).unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "unreferenced".to_owned(),
            root.to_string_lossy().into_owned(),
            "--min-lines".to_owned(),
            "15".to_owned(),
            "--json".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    assert_eq!(parsed["stats"]["total_unreferenced_functions"], 1);
    assert_eq!(parsed["items"][0]["name"], "extract_custom_audit_history");
    assert_eq!(parsed["isolated_files"].as_array().unwrap().len(), 1);
}

#[test]
fn test_cli_unreferenced_aliases() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("clean.py"), "x = 42\n").unwrap();

    // Test alias `dead-functions`
    let mut out1 = Cursor::new(Vec::new());
    let code1 = entry_point::run_with_args_to(
        vec![
            "dead-functions".to_owned(),
            root.to_string_lossy().into_owned(),
        ],
        &mut out1,
    )
    .unwrap();
    assert_eq!(code1, 0);

    // Test alias `isolated-functions`
    let mut out2 = Cursor::new(Vec::new());
    let code2 = entry_point::run_with_args_to(
        vec![
            "isolated-functions".to_owned(),
            root.to_string_lossy().into_owned(),
        ],
        &mut out2,
    )
    .unwrap();
    assert_eq!(code2, 0);
}

#[test]
fn test_deslop_unified_includes_unreferenced() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("app.py"),
        concat!("def run():\n", "    print('app running')\n",),
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            root.to_string_lossy().into_owned(),
            "--no-git".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("# Unreferenced Large Functions in Isolated Files"));
}
