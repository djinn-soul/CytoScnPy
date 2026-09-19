//! Integration tests for repeatable `--ignore` patterns in `cytoscnpy score` and `cytoscnpy deslop`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fmt::Write as _;
use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;

#[test]
fn test_score_repeatable_ignore_patterns() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Clean source file
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/main.py"), "def run():\n    return 42\n").unwrap();

    // Problematic directory 1: legacy with many globals
    fs::create_dir_all(root.join("src/legacy")).unwrap();
    let mut globals_code = String::new();
    for i in 0..25 {
        let _ = writeln!(globals_code, "GLOBAL_{i} = [{i}]");
    }
    globals_code.push_str("def fail():\n    pass\n");
    fs::write(root.join("src/legacy/dirty.py"), globals_code).unwrap();

    // Problematic directory 2: generated code
    fs::create_dir_all(root.join("src/generated")).unwrap();
    fs::write(
        root.join("src/generated/code.py"),
        r#"
GEN_GLOBAL = {"x": 1}

def gen_func():
    try:
        pass
    except:
        pass
"#,
    )
    .unwrap();

    // 1. Without ignore, dirty.py and code.py are scanned
    let mut out_all = Cursor::new(Vec::new());
    let code_all = entry_point::run_with_args_to(
        vec![
            "score".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
            "--no-git".to_owned(),
        ],
        &mut out_all,
    )
    .unwrap();
    assert_eq!(code_all, 0);
    let parsed_all: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out_all.into_inner()).unwrap()).unwrap();
    let raw_all = parsed_all["raw_score"].as_f64().unwrap();

    // 2. With repeatable --ignore for both legacy and generated
    let mut out_ignored = Cursor::new(Vec::new());
    let code_ignored = entry_point::run_with_args_to(
        vec![
            "score".to_owned(),
            root.to_string_lossy().into_owned(),
            "--ignore".to_owned(),
            "legacy".to_owned(),
            "-i".to_owned(),
            "generated/*".to_owned(),
            "--json".to_owned(),
            "--no-git".to_owned(),
        ],
        &mut out_ignored,
    )
    .unwrap();
    assert_eq!(code_ignored, 0);
    let parsed_ignored: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out_ignored.into_inner()).unwrap()).unwrap();
    let raw_ignored = parsed_ignored["raw_score"].as_f64().unwrap();

    // Ignoring the problematic files should strictly reduce raw score and friction
    assert!(
        raw_ignored < raw_all,
        "Expected raw_ignored ({raw_ignored}) < raw_all ({raw_all})"
    );

    // Verify recommendations do not mention ignored files
    if let Some(recs) = parsed_ignored["recommendations"].as_array() {
        for rec in recs {
            if let Some(files) = rec["affected_files"].as_array() {
                for file in files {
                    let file_str = file.as_str().unwrap();
                    assert!(!file_str.contains("dirty.py"), "dirty.py should be ignored");
                    assert!(!file_str.contains("code.py"), "code.py should be ignored");
                }
            }
        }
    }
}

#[test]
fn test_deslop_ignore_flag() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/main.py"), "def run():\n    return 42\n").unwrap();

    fs::create_dir_all(root.join("src/vendor")).unwrap();
    fs::write(
        root.join("src/vendor/dirty.py"),
        r"
def fail():
    try:
        pass
    except:
        pass
",
    )
    .unwrap();

    // deslop with -i vendor
    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            root.to_string_lossy().into_owned(),
            "-i".to_owned(),
            "vendor".to_owned(),
            "--json".to_owned(),
            "--no-git".to_owned(),
        ],
        &mut out,
    )
    .unwrap();
    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output_str).unwrap();

    // Exceptions count should be 0 because dirty.py in vendor was ignored
    let exceptions_count = parsed["exceptions"]["stats"]["bare_except_count"]
        .as_u64()
        .unwrap();
    assert_eq!(exceptions_count, 0);
}
