//! Integration tests for the `duplicates` CLI subcommand and deslop integration.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;

#[test]
fn test_cli_duplicates_clean_project() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("math_utils.py"),
        concat!(
            "def add_numbers(a, b):\n",
            "    return a + b\n\n",
            "def multiply_numbers(a, b):\n",
            "    return a * b\n",
        ),
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec!["duplicates".to_owned(), root.to_string_lossy().into_owned()],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Duplicate Code Clusters & Line Totals"));
    assert!(output_str.contains("No duplicate clusters detected"));
}

#[test]
fn test_cli_duplicates_detects_clones() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let dup_code = concat!(
        "def process_pipeline(records):\n",
        "    valid_records = []\n",
        "    for record in records:\n",
        "        if record.get('active'):\n",
        "            valid_records.append(record['id'])\n",
        "    return valid_records\n",
    );

    fs::write(root.join("pipeline_a.py"), dup_code).unwrap();
    fs::write(root.join("pipeline_b.py"), dup_code).unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "duplicates".to_owned(),
            root.to_string_lossy().into_owned(),
            "--min-lines".to_owned(),
            "4".to_owned(),
            "--min-similarity".to_owned(),
            "0.80".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("Duplicate clusters  : 1"));
    assert!(output_str.contains("Duplicate lines     :"));
    assert!(output_str.contains("pipeline_a.py"));
    assert!(output_str.contains("pipeline_b.py"));
}

#[test]
fn test_cli_duplicates_fail_on_any() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let dup_code = concat!(
        "def sync_data(source, target):\n",
        "    buf = []\n",
        "    for item in source:\n",
        "        if item.is_valid():\n",
        "            buf.append(item.data)\n",
        "    return target.write(buf)\n",
    );

    fs::write(root.join("sync_one.py"), dup_code).unwrap();
    fs::write(root.join("sync_two.py"), dup_code).unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "duplicates".to_owned(),
            root.to_string_lossy().into_owned(),
            "--min-lines".to_owned(),
            "4".to_owned(),
            "--fail-on-any".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 1);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    assert!(output_str.contains("[GATE] Duplicate code threshold exceeded"));
}

#[test]
fn test_cli_duplicates_max_thresholds() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let dup_code = concat!(
        "def compute_hash(data):\n",
        "    h = 0\n",
        "    for b in data:\n",
        "        h = (h * 31 + b) & 0xFFFFFFFF\n",
        "    return h\n",
    );

    fs::write(root.join("h1.py"), dup_code).unwrap();
    fs::write(root.join("h2.py"), dup_code).unwrap();

    // With max_clusters=1 -> passes
    let mut out1 = Cursor::new(Vec::new());
    let code1 = entry_point::run_with_args_to(
        vec![
            "duplicates".to_owned(),
            root.to_string_lossy().into_owned(),
            "--min-lines".to_owned(),
            "4".to_owned(),
            "--max-clusters".to_owned(),
            "1".to_owned(),
        ],
        &mut out1,
    )
    .unwrap();
    assert_eq!(code1, 0);

    // With max_clusters=0 -> fails
    let mut out2 = Cursor::new(Vec::new());
    let code2 = entry_point::run_with_args_to(
        vec![
            "duplicates".to_owned(),
            root.to_string_lossy().into_owned(),
            "--min-lines".to_owned(),
            "4".to_owned(),
            "--max-clusters".to_owned(),
            "0".to_owned(),
        ],
        &mut out2,
    )
    .unwrap();
    assert_eq!(code2, 1);
}

#[test]
fn test_cli_duplicates_json_output() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let dup_code = concat!(
        "def transform(items):\n",
        "    out = []\n",
        "    for x in items:\n",
        "        if x > 0:\n",
        "            out.append(x * 2)\n",
        "    return out\n",
    );

    fs::write(root.join("m1.py"), dup_code).unwrap();
    fs::write(root.join("m2.py"), dup_code).unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "duplicates".to_owned(),
            root.to_string_lossy().into_owned(),
            "--min-lines".to_owned(),
            "4".to_owned(),
            "--json".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output_str).unwrap();
    assert_eq!(parsed["stats"]["cluster_count"], 1);
    assert!(parsed["stats"]["total_duplicate_lines"].as_u64().unwrap() > 0);
    assert_eq!(parsed["clusters"].as_array().unwrap().len(), 1);
}

#[test]
fn test_cli_duplicates_aliases() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("clean.py"), "val = 42\n").unwrap();

    // Test alias `dupes`
    let mut out1 = Cursor::new(Vec::new());
    let code1 = entry_point::run_with_args_to(
        vec!["dupes".to_owned(), root.to_string_lossy().into_owned()],
        &mut out1,
    )
    .unwrap();
    assert_eq!(code1, 0);

    // Test alias `clones-summary`
    let mut out2 = Cursor::new(Vec::new());
    let code2 = entry_point::run_with_args_to(
        vec![
            "clones-summary".to_owned(),
            root.to_string_lossy().into_owned(),
        ],
        &mut out2,
    )
    .unwrap();
    assert_eq!(code2, 0);
}

#[test]
fn test_deslop_unified_includes_duplicates() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(
        root.join("app.py"),
        concat!("def run():\n", "    print('hello world')\n",),
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
    assert!(output_str.contains("# Duplicate Code Clusters"));
}
