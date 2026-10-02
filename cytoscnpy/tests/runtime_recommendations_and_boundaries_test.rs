//! Integration tests for runtime anti-pattern recommendations and dependency boundary refinements.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

use cytoscnpy::entry_point;

#[test]
fn test_runtime_antipattern_and_singleton_recommendations() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Setup git repo to allow context & scoring analysis
    let git_dir = root.join(".git");
    fs::create_dir_all(&git_dir).unwrap();

    let src_dir = root.join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // 1. Python file with magic numbers, deep nesting, and singleton pattern
    let anti_py = r"
class DatabasePool:
    _instance = None

    @classmethod
    def get_instance(cls):
        if cls._instance is None:
            cls._instance = cls()
        return cls._instance

def check_value(x):
    if x > 4299:
        if x < 9999:
            while x > 5000:
                for i in range(10):
                    if i == 7:
                        return i
    return 0
";
    fs::write(src_dir.join("pool.py"), anti_py).unwrap();

    // Run the unified deslop report with JSON output.
    let mut out_score_json = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            root.to_string_lossy().into_owned(),
            "--format".to_owned(),
            "json".to_owned(),
        ],
        &mut out_score_json,
    )
    .unwrap();
    assert_eq!(code, 0);

    let score_output = String::from_utf8(out_score_json.into_inner()).unwrap();
    let score_json: serde_json::Value = serde_json::from_str(&score_output).unwrap();

    let recs = score_json["recommendations"].as_array().unwrap();
    let rec_ids: Vec<&str> = recs
        .iter()
        .map(|r| r["id"].as_str().unwrap_or_default())
        .collect();

    // Validate that the anti-pattern recommendations are generated
    assert!(
        rec_ids.contains(&"replace-magic-numbers"),
        "Expected replace-magic-numbers in recommendations: {rec_ids:?}"
    );
    assert!(
        rec_ids.contains(&"refactor-nested-callbacks"),
        "Expected refactor-nested-callbacks in recommendations: {rec_ids:?}"
    );
    assert!(
        rec_ids.contains(&"refactor-singletons"),
        "Expected refactor-singletons in recommendations: {rec_ids:?}"
    );

    // Verify LLM formatting includes the action items
    let mut out_score_llm = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            root.to_string_lossy().into_owned(),
            "--format".to_owned(),
            "llm".to_owned(),
        ],
        &mut out_score_llm,
    )
    .unwrap();
    assert_eq!(code, 0);

    let llm_output = String::from_utf8(out_score_llm.into_inner()).unwrap();
    assert!(llm_output.contains("magic number"));
    assert!(llm_output.contains("singleton"));
}

#[test]
fn test_generated_directories_skipped_by_default() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let gen_dir = root.join("generated");
    let codegen_dir = root.join("codegen");
    let src_dir = root.join("src");

    fs::create_dir_all(&gen_dir).unwrap();
    fs::create_dir_all(&codegen_dir).unwrap();
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(gen_dir.join("schema_pb2.py"), "x = 1\n").unwrap();
    fs::write(codegen_dir.join("api_client.py"), "y = 2\n").unwrap();
    fs::write(src_dir.join("app.py"), "def start(): pass\n").unwrap();

    let mut out_doctor_json = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out_doctor_json,
    )
    .unwrap();
    assert_eq!(code, 0);

    let doc_output = String::from_utf8(out_doctor_json.into_inner()).unwrap();
    let doc_json: serde_json::Value = serde_json::from_str(&doc_output).unwrap();

    // Only app.py should be counted as source file
    let source_files = doc_json["health"][0]["structure"]["source_files"]
        .as_u64()
        .unwrap();
    assert_eq!(source_files, 1);
}

#[test]
fn test_dependency_boundaries_clean_vendor_separation() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let src_dir = root.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("lib.py"), "val = 10\n").unwrap();
    fs::write(root.join(".gitignore"), "target/\n").unwrap();
    fs::write(root.join("Cargo.lock"), "").unwrap();

    let mut out_score_json = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            root.to_string_lossy().into_owned(),
            "--format".to_owned(),
            "json".to_owned(),
        ],
        &mut out_score_json,
    )
    .unwrap();
    assert_eq!(code, 0);

    let score_output = String::from_utf8(out_score_json.into_inner()).unwrap();
    let score_json: serde_json::Value = serde_json::from_str(&score_output).unwrap();

    let dims = score_json["dimensions"].as_array().unwrap();
    let dep_dim = dims
        .iter()
        .find(|d| d["name"].as_str() == Some("Dependency boundaries"))
        .unwrap();

    let evidence = dep_dim["evidence"].as_str().unwrap();
    assert!(evidence.contains("clean vendor/source separation"));
}
