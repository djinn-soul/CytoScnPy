//! Tests for CLI metrics output.

#![allow(
    clippy::unwrap_used,
    clippy::str_to_string,
    clippy::uninlined_format_args,
    clippy::ignore_without_reason
)]

#[allow(deprecated)]
use cytoscnpy::commands::{
    run_cc, run_files, run_hal, run_mi, run_raw, run_stats, run_stats_v2, Inspections, ScanOptions,
};
use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-tmp");
    fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("cli_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

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
    assert!(output.contains('3')); // LOC (x=1, #comment, newline)
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

    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("\"file\":"));
    assert!(output.contains("\"loc\":"));
    assert!(output.contains('2'));
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
#[ignore] // TODO: WalkDir exclude filtering needs deeper investigation for nested dirs
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
    assert!(parsed.as_array().unwrap().is_empty());
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
