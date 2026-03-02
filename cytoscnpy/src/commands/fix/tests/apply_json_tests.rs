use super::*;

#[test]
fn test_run_fix_deadcode_dry_run_json_output() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.py");
    std::fs::write(&file_path, "def unused_function():\n    pass\n").unwrap();

    let def = create_definition("unused_function", "function", file_path, 1);
    let mut results = AnalysisResult::default();
    results.unused_functions.push(def);

    let options = DeadCodeFixOptions {
        min_confidence: 80,
        dry_run: true,
        json_output: true,
        fix_functions: true,
        fix_methods: false,
        fix_classes: false,
        fix_imports: false,
        fix_variables: false,
        verbose: false,
        with_cst: false,
        analysis_root: dir.path().to_path_buf(),
    };

    let mut buffer = Vec::new();
    let fix_results = run_fix_deadcode(&results, &options, &mut buffer).unwrap();
    assert_eq!(fix_results.len(), 1);

    let payload: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
    assert_eq!(payload["schema_version"], "2");
    assert_eq!(payload["kind"], "dead_code_fix_plan");
    assert_eq!(payload["planned_items"], 1);
    assert!(payload["plans"][0]["planned_edits"][0]["stable_id"].is_string());

    let temp_path = dir.path().to_string_lossy().to_string();
    let escaped_temp_path = temp_path.replace('\\', "\\\\");
    let normalized = String::from_utf8(buffer)
        .unwrap()
        .replace(&temp_path, "[TMP]")
        .replace(&escaped_temp_path, "[TMP]")
        .replace("[TMP]\\\\", "[TMP]/");
    insta::assert_snapshot!("dead_code_fix_plan_json", normalized);
}

#[test]
fn test_run_fix_deadcode_apply_json_output() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.py");
    std::fs::write(&file_path, "def unused_function():\n    pass\n").unwrap();

    let def = create_definition("unused_function", "function", file_path.clone(), 1);
    let mut results = AnalysisResult::default();
    results.unused_functions.push(def);

    let options = DeadCodeFixOptions {
        min_confidence: 80,
        dry_run: false,
        json_output: true,
        fix_functions: true,
        fix_methods: false,
        fix_classes: false,
        fix_imports: false,
        fix_variables: false,
        verbose: false,
        with_cst: false,
        analysis_root: dir.path().to_path_buf(),
    };

    let mut buffer = Vec::new();
    let fix_results = run_fix_deadcode(&results, &options, &mut buffer).unwrap();
    assert_eq!(fix_results.len(), 1);

    let payload: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
    assert_eq!(payload["schema_version"], "2");
    assert_eq!(payload["kind"], "dead_code_fix_report");
    assert_eq!(payload["applied_files"], 1);
    assert_eq!(payload["items_removed"], 1);
    assert!(payload["results"][0]["file"].is_string());

    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(content.trim().is_empty());
}
