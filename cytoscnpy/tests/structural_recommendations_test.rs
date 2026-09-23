//! Integration tests for top-level directory breakdowns, function collision disambiguation,
//! and stable library extraction recommendations.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fmt::Write as _;
use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;

#[test]
fn test_doctor_and_score_top_level_directories() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let src_dir = root.join("src");
    let tests_dir = root.join("tests");
    let docs_dir = root.join("docs");

    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&tests_dir).unwrap();
    fs::create_dir_all(&docs_dir).unwrap();

    fs::write(src_dir.join("main.py"), "def start(): pass\n").unwrap();
    fs::write(tests_dir.join("test_main.py"), "def test_start(): pass\n").unwrap();
    fs::write(docs_dir.join("conf.py"), "# Sphinx configuration\n").unwrap();

    // 1. Doctor command JSON output
    let mut out_doctor_json = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "doctor".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out_doctor_json,
    )
    .unwrap();
    assert_eq!(code, 0);

    let doc_output = String::from_utf8(out_doctor_json.into_inner()).unwrap();
    let doc_json: serde_json::Value = serde_json::from_str(&doc_output).unwrap();
    let top_dirs: Vec<String> = doc_json["structure"]["top_level_dirs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();

    assert!(top_dirs.contains(&"docs".to_owned()));
    assert!(top_dirs.contains(&"src".to_owned()));
    assert!(top_dirs.contains(&"tests".to_owned()));

    // 2. Doctor terminal table output
    let mut out_doctor_txt = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec!["doctor".to_owned(), root.to_string_lossy().into_owned()],
        &mut out_doctor_txt,
    )
    .unwrap();
    assert_eq!(code, 0);
    let doc_txt = String::from_utf8(out_doctor_txt.into_inner()).unwrap();
    assert!(doc_txt.contains("Top-Level Dirs"));
    assert!(doc_txt.contains("docs"));
    assert!(doc_txt.contains("src"));
    assert!(doc_txt.contains("tests"));

    // 3. Score command JSON output
    let mut out_score_json = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "score".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out_score_json,
    )
    .unwrap();
    assert_eq!(code, 0);

    let score_output = String::from_utf8(out_score_json.into_inner()).unwrap();
    let score_json: serde_json::Value = serde_json::from_str(&score_output).unwrap();
    let summary_top_dirs: Vec<String> = score_json["summary"]["top_level_dirs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();

    assert_eq!(summary_top_dirs, top_dirs);
}

#[test]
fn test_disambiguate_colliding_functions_recommendation() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create 3 separate modules defining the same non-dunder function name
    for dir in ["pkg_a", "pkg_b", "pkg_c"] {
        let dir_path = root.join(dir);
        fs::create_dir_all(&dir_path).unwrap();
        fs::write(
            dir_path.join("task.py"),
            "def execute_task():\n    return 'done'\n",
        )
        .unwrap();
    }

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "score".to_owned(),
            root.to_string_lossy().into_owned(),
            "--format".to_owned(),
            "llm".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();

    assert!(
        output_str.contains("disambiguate-function-names")
            || output_str.contains("Disambiguate 1 colliding function name(s)"),
        "Expected disambiguate function names recommendation in:\n{output_str}"
    );
}

#[test]
fn test_extract_stable_library_recommendation() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create a large frozen utility file with over 500 lines
    let lib_dir = root.join("lib");
    fs::create_dir_all(&lib_dir).unwrap();

    let mut big_code = String::new();
    big_code.push_str("def process_large_batch():\n");
    for i in 0..520 {
        let _ = writeln!(big_code, "    line_{i} = {i}");
    }
    big_code.push_str("    return 0\n");

    fs::write(lib_dir.join("legacy_engine.py"), big_code).unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "score".to_owned(),
            root.to_string_lossy().into_owned(),
            "--format".to_owned(),
            "llm".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();

    assert!(
        output_str.contains("extract-stable-library")
            || output_str.contains("Extract 1 stable/frozen file(s) into independent package(s)"),
        "Expected extract-stable-library recommendation in:\n{output_str}"
    );
}
