use super::*;

#[test]
fn test_cli_fix_apply_json_outputs_single_fix_payload() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.py");
    std::fs::write(&file_path, "def unused():\n    return 1\n").unwrap();

    let mut buffer = Vec::new();
    let result = run_with_args_to(
        vec![
            "--fix".to_owned(),
            "--apply".to_owned(),
            "--json".to_owned(),
            file_path.to_string_lossy().to_string(),
        ],
        &mut buffer,
    )
    .unwrap();
    assert_eq!(result, 0);

    let output = String::from_utf8(buffer).unwrap();
    let payload: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(payload["kind"], "dead_code_fix_report");
}
