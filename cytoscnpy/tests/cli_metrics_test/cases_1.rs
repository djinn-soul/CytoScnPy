use super::*;

#[test]
fn test_cli_raw() {
    let dir = project_tempdir();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "x = 1\n# comment").unwrap();

    let mut buffer = Vec::new();
    run_raw(
        &[dir.path().to_path_buf()],
        false,
        vec![],
        Vec::new(),
        false,
        None,
        false,
        &mut buffer,
    )
    .unwrap();

    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("test.py"));
    assert!(output.contains("LOC"));
    assert!(output.contains('2')); // LOC (x=1, #comment); trailing newline is not a line
    assert!(output.contains('1')); // SLOC/Comments
}

#[test]
fn test_cli_cc() {
    let dir = project_tempdir();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "def foo():\n    if True:\n        pass").unwrap();

    let mut buffer = Vec::new();
    run_cc(
        &[dir.path().to_path_buf()],
        cytoscnpy::commands::CcOptions {
            json: false,
            exclude: vec![],
            ignore: Vec::new(),
            output_file: None,
            verbose: false,
            ..Default::default()
        },
        &mut buffer,
    )
    .unwrap();

    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("test.py"));
    assert!(output.contains("foo"));
    assert!(output.contains("function"));
    assert!(output.contains('A')); // Rank
}

#[test]
fn test_cli_hal() {
    let dir = project_tempdir();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "x = 1").unwrap();

    let mut buffer = Vec::new();
    run_hal(
        &[dir.path().to_path_buf()],
        false,
        vec![],
        Vec::new(),
        false,
        None,
        false,
        &mut buffer,
    )
    .unwrap();

    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("test.py"));
    assert!(output.contains("Vocab"));
    assert!(output.contains("3.00"));
    assert!(output.contains("4.75"));
}

#[test]
fn test_cli_mi() {
    let dir = project_tempdir();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "x = 1").unwrap();

    let mut buffer = Vec::new();
    run_mi(
        &[dir.path().to_path_buf()],
        cytoscnpy::commands::MiOptions {
            json: false,
            exclude: vec![],
            ignore: Vec::new(),
            output_file: None,
            verbose: false,
            ..Default::default()
        },
        &mut buffer,
    )
    .unwrap();

    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("test.py"));
    assert!(output.contains("Rank"));
    assert!(output.contains('A'));
}

#[test]
fn test_cli_json_output() {
    let dir = project_tempdir();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "x = 1").unwrap();

    let mut buffer = Vec::new();
    run_raw(
        &[dir.path().to_path_buf()],
        true,
        vec![],
        Vec::new(),
        false,
        None,
        false,
        &mut buffer,
    )
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
    assert!(output[0]["file"].as_str().is_some());
    assert_eq!(output[0]["loc"], 1);
}

// ==================== STATS COMMAND TESTS ====================

#[test]
fn test_cli_stats_markdown_output() {
    let dir = project_tempdir();
    let file_path = dir.path().join("module.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(
        file,
        "def hello():\n    pass\n\nclass MyClass:\n    def method(self):\n        pass"
    )
    .unwrap();

    let output_path = dir.path().join("report.md");
    let mut buffer = Vec::new();
    run_stats_v2(
        dir.path(),                  // root
        &[dir.path().to_path_buf()], // roots
        ScanOptions {
            all: false,
            inspections: Inspections {
                secrets: false,
                danger: false,
                quality: false,
            },
            json: false,
        },
        Some(output_path.to_string_lossy().to_string()),
        &[],
        false, // include_tests
        &[],   // include_folders
        false,
        cytoscnpy::config::Config::default(),
        &mut buffer,
    )
    .unwrap();

    assert!(output_path.exists(), "Markdown report should be created");
    let content = fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("# CytoScnPy Project Statistics Report"));
    assert!(content.contains("Total Files"));
    assert!(content.contains("Functions"));
    assert!(content.contains("Classes"));
}

#[test]
fn test_cli_stats_json_output() {
    let dir = project_tempdir();
    let file_path = dir.path().join("test.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "def foo():\n    pass\n\ndef bar():\n    pass").unwrap();

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
    assert!(output.contains("\"total_files\":"));
    assert!(output.contains("\"total_functions\":"));
    assert!(output.contains("\"code_lines\":"));

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(parsed["total_files"].as_u64().unwrap() >= 1);
    assert!(parsed["total_functions"].as_u64().unwrap() >= 2);
}

#[test]
fn test_cli_stats_all_flag() {
    let dir = project_tempdir();
    let file_path = dir.path().join("main.py");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "# Main module\ndef main():\n    pass").unwrap();

    let output_path = dir.path().join("full_report.md");
    let mut buffer = Vec::new();
    run_stats_v2(
        dir.path(),
        &[dir.path().to_path_buf()],
        ScanOptions {
            all: true,
            inspections: Inspections {
                secrets: false,
                danger: false,
                quality: false,
            },
            json: false,
        },
        Some(output_path.to_string_lossy().to_string()),
        &[],
        false, // include_tests
        &[],   // include_folders
        false,
        cytoscnpy::config::Config::default(),
        &mut buffer,
    )
    .unwrap();

    let content = fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("Per-File Metrics"));
    assert!(content.contains("Secrets Scan"));
    assert!(content.contains("Dangerous Code"));
    assert!(content.contains("Quality Issues"));
}

#[test]
fn test_cli_stats_multiple_files() {
    let dir = project_tempdir();

    for i in 1..=3 {
        let file_path = dir.path().join(format!("module{}.py", i));
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "def func{}():\n    pass", i).unwrap();
    }

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
    assert_eq!(parsed["total_files"].as_u64().unwrap(), 3);
    assert_eq!(parsed["total_functions"].as_u64().unwrap(), 3);
}
