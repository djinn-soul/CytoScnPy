use super::*;

#[test]
fn test_cli_stats_with_classes() {
    let dir = project_tempdir();
    let file_path = dir.path().join("models.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(
        file,
        "class User:\n    def __init__(self):\n        pass\n\nclass Product:\n    def get_price(self):\n        return 0"
    )
    .unwrap();

    let mut buffer = Vec::new();
    run_stats_v2(
        dir.path(),
        &[dir.path().to_path_buf()],
        ScanOptions {
            all: false,
            inspections: Inspections {
                secrets: false,
                danger: false,
                quality: false,
            },
            json: true,
        },
        None,
        &[],
        false, // include_tests
        &[],   // include_folders
        false,
        cytoscnpy::config::Config::default(),
        &mut buffer,
    )
    .unwrap();

    let output = String::from_utf8(buffer).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["total_classes"].as_u64().unwrap(), 2);
    assert!(parsed["total_functions"].as_u64().unwrap() >= 2);
}

// ==================== FILES COMMAND TESTS ====================

#[test]
fn test_cli_files_table_output() {
    let dir = project_tempdir();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "x = 1\n# comment\n\ny = 2").unwrap();

    let mut buffer = Vec::new();
    run_files(&[dir.path().to_path_buf()], false, &[], false, &mut buffer).unwrap();

    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("test.py"));
    assert!(output.contains("Code"));
    assert!(output.contains("Comments"));
    assert!(output.contains("Empty"));
    assert!(output.contains("Total"));
    assert!(output.contains("Size"));
}

#[test]
fn test_cli_files_json_output() {
    let dir = project_tempdir();
    let file_path = dir.path().join("app.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "# Application\ndef run():\n    print('hello')").unwrap();

    let mut buffer = Vec::new();
    run_files(&[dir.path().to_path_buf()], true, &[], false, &mut buffer).unwrap();

    let output = String::from_utf8(buffer).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert!(parsed.is_array());
    let files = parsed.as_array().unwrap();
    assert_eq!(files.len(), 1);

    let file_metrics = &files[0];
    assert!(file_metrics["file"].as_str().unwrap().contains("app.py"));
    assert!(file_metrics["code_lines"].as_u64().is_some());
    assert!(file_metrics["comment_lines"].as_u64().is_some());
    assert!(file_metrics["size_kb"].as_f64().is_some());
}

#[test]
fn test_cli_files_multiple_files() {
    let dir = project_tempdir();

    let file1 = dir.path().join("small.py");
    let mut f1 = File::create(&file1).unwrap();
    writeln!(f1, "x = 1").unwrap();

    let file2 = dir.path().join("large.py");
    let mut f2 = File::create(&file2).unwrap();
    writeln!(
        f2,
        "# Large file\ndef func1():\n    pass\n\ndef func2():\n    pass"
    )
    .unwrap();

    let mut buffer = Vec::new();
    run_files(&[dir.path().to_path_buf()], true, &[], false, &mut buffer).unwrap();

    let output = String::from_utf8(buffer).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 2);
}

#[test]
fn test_cli_files_exclude_folder() {
    let dir = project_tempdir();

    let main_file = dir.path().join("main.py");
    let mut f = File::create(&main_file).unwrap();
    writeln!(f, "x = 1").unwrap();

    let excluded_dir = dir.path().join("node_modules"); // Use common excluded name
    fs::create_dir(&excluded_dir).unwrap();
    let excluded_file = excluded_dir.join("hidden.py");
    let mut ef = File::create(&excluded_file).unwrap();
    writeln!(ef, "y = 2").unwrap();

    let mut buffer = Vec::new();
    run_files(
        &[dir.path().to_path_buf()],
        true,
        &["node_modules".to_string()],
        false,
        &mut buffer,
    )
    .unwrap();

    let output = String::from_utf8(buffer).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let files = parsed.as_array().unwrap();
    // Should only have main.py, hidden.py should be excluded
    assert!(files
        .iter()
        .any(|f| f["file"].as_str().unwrap().contains("main.py")));
    assert!(!files
        .iter()
        .any(|f| f["file"].as_str().unwrap().contains("hidden.py")));
}

#[test]
fn test_cli_files_empty_directory() {
    let dir = project_tempdir();

    let mut buffer = Vec::new();
    run_files(&[dir.path().to_path_buf()], true, &[], false, &mut buffer).unwrap();

    let output = String::from_utf8(buffer).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(
        parsed.as_array().unwrap().is_empty(),
        "expected empty, got {:?}",
        parsed.as_array().unwrap()
    );
}

#[test]
fn test_cli_stats_empty_directory() {
    let dir = project_tempdir();

    let mut buffer = Vec::new();
    run_stats_v2(
        dir.path(),
        &[dir.path().to_path_buf()],
        ScanOptions {
            all: false,
            inspections: Inspections {
                secrets: false,
                danger: false,
                quality: false,
            },
            json: true,
        },
        None,
        &[],
        false, // include_tests
        &[],   // include_folders
        false,
        cytoscnpy::config::Config::default(),
        &mut buffer,
    )
    .unwrap();

    let output = String::from_utf8(buffer).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["total_files"].as_u64().unwrap(), 0);
    assert_eq!(parsed["total_functions"].as_u64().unwrap(), 0);
    assert_eq!(parsed["total_classes"].as_u64().unwrap(), 0);
}

// ==================== DEPRECATED RUN_STATS SHIM TESTS ====================

#[test]
#[allow(deprecated)]
fn test_deprecated_run_stats_shim() {
    let dir = project_tempdir();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "def foo():\n    pass").unwrap();

    let mut buffer = Vec::new();
    let result = run_stats(
        dir.path(),
        &[dir.path().to_path_buf()],
        false,
        false,
        false,
        false,
        true,
        None,
        &[],
        false,
        &mut buffer,
    );

    assert!(result.is_ok());
    let output = String::from_utf8(buffer).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(parsed["total_files"].as_u64().unwrap() >= 1);
}

// ==================== MARKDOWN OUTPUT WITH FINDINGS TESTS ====================

#[test]
fn test_cli_stats_markdown_with_secrets_findings() {
    let dir = project_tempdir();
    let file_path = dir.path().join("config.py");
    let mut file = File::create(&file_path).unwrap();
    // Code with a hardcoded secret-like string
    writeln!(
        file,
        r#"API_KEY = "sk_live_aBcDeFgHiJkLmNoPqRsTuVwXyZ1234567890abcdef""#
    )
    .unwrap();

    let mut buffer = Vec::new();
    run_stats_v2(
        dir.path(),
        &[dir.path().to_path_buf()],
        ScanOptions {
            all: false,
            inspections: Inspections {
                secrets: true,
                danger: false,
                quality: false,
            },
            json: false,
        },
        None,
        &[],
        false,
        &[],
        false,
        cytoscnpy::config::Config::default(),
        &mut buffer,
    )
    .unwrap();

    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("Secrets Scan"));
}
