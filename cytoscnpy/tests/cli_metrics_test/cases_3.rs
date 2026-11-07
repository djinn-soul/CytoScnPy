use super::*;

#[test]
fn test_cli_stats_markdown_with_danger_findings() {
    let dir = project_tempdir();
    let file_path = dir.path().join("dangerous.py");
    let mut file = File::create(&file_path).unwrap();
    // Code with dangerous eval usage
    writeln!(file, "result = eval(user_input)").unwrap();

    let mut buffer = Vec::new();
    run_stats_v2(
        dir.path(),
        &[dir.path().to_path_buf()],
        ScanOptions {
            all: false,
            inspections: Inspections {
                secrets: false,
                danger: true,
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
    assert!(output.contains("Dangerous Code"));
}

#[test]
fn test_cli_stats_markdown_with_quality_findings() {
    let dir = project_tempdir();
    let file_path = dir.path().join("complex.py");
    let mut file = File::create(&file_path).unwrap();
    // Code with high cyclomatic complexity
    writeln!(
        file,
        r"def complex_func(a, b, c, d, e, f, g, h, i, j, k):
    if a:
        if b:
            if c:
                if d:
                    if e:
                        if f:
                            if g:
                                if h:
                                    if i:
                                        if j:
                                            return k
    return None
"
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
                quality: true,
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
    assert!(output.contains("Quality Issues"));
}

#[test]
fn test_cli_stats_json_with_output_file() {
    let dir = project_tempdir();
    let file_path = dir.path().join("app.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "def main():\n    pass").unwrap();

    let output_path = dir.path().join("report.json");
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
        Some(output_path.to_string_lossy().to_string()),
        &[],
        false,
        &[],
        false,
        cytoscnpy::config::Config::default(),
        &mut buffer,
    )
    .unwrap();

    // Verify the file was written
    assert!(output_path.exists());
    let content = fs::read_to_string(&output_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(parsed["total_files"].as_u64().is_some());

    // Verify the buffer contains the "written to" message
    let console_output = String::from_utf8(buffer).unwrap();
    assert!(console_output.contains("Report written to"));
}

#[test]
fn test_cli_stats_include_tests_flag() {
    let dir = project_tempdir();

    // Create a regular file
    let main_file = dir.path().join("main.py");
    let mut f1 = File::create(&main_file).unwrap();
    writeln!(f1, "def main():\n    pass").unwrap();

    // Create a test file
    let test_file = dir.path().join("test_main.py");
    let mut f2 = File::create(&test_file).unwrap();
    writeln!(f2, "def test_main():\n    assert True").unwrap();

    // Without include_tests - should only include main.py
    let mut buffer1 = Vec::new();
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
        false, // include_tests = false
        &[],
        false,
        cytoscnpy::config::Config::default(),
        &mut buffer1,
    )
    .unwrap();

    let output1 = String::from_utf8(buffer1).unwrap();
    let parsed1: serde_json::Value = serde_json::from_str(&output1).unwrap();
    let files_without_tests = parsed1["total_files"].as_u64().unwrap();

    // With include_tests - should include both files
    let mut buffer2 = Vec::new();
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
        true, // include_tests = true
        &[],
        false,
        cytoscnpy::config::Config::default(),
        &mut buffer2,
    )
    .unwrap();

    let output2 = String::from_utf8(buffer2).unwrap();
    let parsed2: serde_json::Value = serde_json::from_str(&output2).unwrap();
    let files_with_tests = parsed2["total_files"].as_u64().unwrap();

    assert!(
        files_with_tests >= files_without_tests,
        "Including tests should show at least as many files"
    );
}
