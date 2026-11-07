use super::*;

#[test]
fn test_github_report_coverage() {
    let result = create_mock_result();
    let mut buffer = Vec::new();
    github::print_github(&mut buffer, &result).unwrap();
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains(
        "::error file=test.py,line=10,col=5,title=CSP-D001::Dangerous function call (test.py:10)"
    ));
    assert!(output.contains("UnusedMethod"));
    assert!(output.contains("UnusedClass"));
    assert!(output.contains("UnusedImport"));
    assert!(output.contains("UnusedVariable"));
    assert!(output.contains("UnusedParameter"));
}

#[test]
fn test_junit_report_coverage() {
    let result = create_mock_result();
    let mut buffer = Vec::new();
    junit::print_junit(&mut buffer, &result).unwrap();
    let output = String::from_utf8(buffer).unwrap();
    // mock_result has: 1 danger + 1 secret + 1 quality + 1 taint + 1 parse_error + 6 unused = 11 findings
    assert!(output.contains(r#"<testsuite name="CytoScnPy" tests="11" failures="11" errors="0">"#));
    // The current implementation does not include the 'type' attribute in the failure tag
    assert!(output.contains(r#"<failure message="Dangerous function call">Line 10: Dangerous function call (test.py:10)</failure>"#));
}

#[test]
fn test_markdown_report_coverage() {
    let result = create_mock_result();
    let mut buffer = Vec::new();
    markdown::print_markdown(&mut buffer, &result).unwrap();
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("# CytoScnPy Analysis Report"));
    assert!(output.contains("## Security Issues"));
    // Matches: | Rule | File | Line | Message | Severity |
    // | CSP-D001 | test.py | 10 | Dangerous function call | HIGH |
    assert!(output.contains("| CSP-D001 | test.py | 10 | Dangerous function call | HIGH |"));
}

#[test]
fn test_gitlab_report_coverage() {
    let result = create_mock_result();
    let mut buffer = Vec::new();
    gitlab::print_gitlab(&mut buffer, &result).unwrap();
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains(r#""description": "Dangerous function call (test.py:10)""#));
    assert!(output.contains(r#""fingerprint": "danger-CSP-D001-test.py-10-5""#));
    assert!(output.contains(r#""check_name": "CSP-D001""#));
    assert!(output.contains(r#""description": "High complexity (test.py:5)""#));
    assert!(output.contains(r#""fingerprint": "quality-CSP-Q001-test.py-5-0""#));
    assert!(output.contains(r#""check_name": "CSP-Q001""#));
    assert!(output.contains(r#""fingerprint": "taint-CSP-T001-test.py-12-15-10""#));
    assert!(output.contains(r#""check_name": "CSP-T001""#));
    assert!(
        output.contains(r#""fingerprint": "secret-CSP-S001-test.py-20-100-Hardcoded password-"#)
    );
    assert!(output.contains(r#""check_name": "CSP-S001""#));
    assert!(output.contains(r#""fingerprint": "parse-error-test.py-Syntax error at line 40""#));
    assert!(output.contains(r#""check_name": "ParseError""#));

    let mut reordered_result = create_mock_result();
    let mut preceding_finding = reordered_result.danger[0].clone();
    preceding_finding.rule_id = "CSP-D002".to_owned();
    preceding_finding.line = 1;
    reordered_result.danger.insert(0, preceding_finding);
    let mut reordered_buffer = Vec::new();
    gitlab::print_gitlab(&mut reordered_buffer, &reordered_result).unwrap();
    let reordered_output = String::from_utf8(reordered_buffer).unwrap();
    assert!(reordered_output.contains(r#""fingerprint": "danger-CSP-D001-test.py-10-5""#));

    let mut root_buffer = Vec::new();
    gitlab::print_gitlab_with_root(&mut root_buffer, &result, Some(Path::new("."))).unwrap();
    let root_output = String::from_utf8(root_buffer).unwrap();
    assert!(root_output.contains(r#""path": "test.py""#));
}

#[test]
fn test_sarif_report_coverage() {
    let result = create_mock_result();
    let mut buffer = Vec::new();
    sarif::print_sarif(&mut buffer, &result).unwrap();
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains(r#""version": "2.1.0""#));
    // SARIF uses "ruleId" for results
    assert!(output.contains(r#""ruleId": "CSP-D001""#));
    // Check path in message
    assert!(output.contains(r#""text": "Dangerous function call (test.py:10)""#));
}

#[test]
fn test_init_command_full_coverage() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let mut buffer = Vec::new();

    // 1. Init in empty dir -> creates cytoscnpy.toml
    run_init_in(root, &mut buffer).unwrap();
    assert!(root.join(".cytoscnpy.toml").exists());

    // 2. Init again -> skips
    buffer.clear();
    run_init_in(root, &mut buffer).unwrap();
    let output = String::from_utf8(buffer.clone()).unwrap();
    assert!(output.contains("already exists"));

    // 3. Init with existing pyproject.toml -> appends
    let dir2 = tempdir().unwrap();
    let root2 = dir2.path();
    std::fs::write(root2.join("pyproject.toml"), "[project]\nname = 'test'").unwrap();
    buffer.clear();
    run_init_in(root2, &mut buffer).unwrap();
    let content = std::fs::read_to_string(root2.join("pyproject.toml")).unwrap();
    assert!(content.contains("[tool.cytoscnpy]"));
}

#[test]
fn test_output_formatting_coverage() {
    use cytoscnpy::output;
    let result = create_mock_result();
    let mut buffer = Vec::new();

    // Disable colors for consistent string matching across all output tests
    colored::control::set_override(false);

    // Test header
    output::print_header(&mut buffer).unwrap();
    assert!(String::from_utf8(buffer.clone())
        .unwrap()
        .contains("Python Static Analysis Results"));

    // Test summary pills
    buffer.clear();
    output::print_summary_pills(&mut buffer, &result).unwrap();
    let out = String::from_utf8(buffer.clone()).unwrap();
    assert!(out.contains("Unreachable: 0"));
    assert!(out.contains("Functions: 1"));

    // Test stats
    buffer.clear();
    output::print_analysis_stats(&mut buffer, &result.analysis_summary).unwrap();
    assert!(String::from_utf8(buffer.clone())
        .unwrap()
        .contains("Analyzed 1 files"));

    // Test findings
    buffer.clear();
    output::print_findings(&mut buffer, "Test Findings", &result.danger).unwrap();
    assert!(String::from_utf8(buffer.clone())
        .unwrap()
        .contains("CSP-D001"));

    // Test report grouped
    buffer.clear();
    output::print_report_grouped(&mut buffer, &result).unwrap();
    assert!(String::from_utf8(buffer.clone())
        .unwrap()
        .contains("File: test.py"));

    // Test report quiet
    buffer.clear();
    output::print_report_quiet(&mut buffer, &result).unwrap();
    assert!(String::from_utf8(buffer.clone())
        .unwrap()
        .contains("[SUMMARY]"));

    // Test exclusion list
    buffer.clear();
    output::print_exclusion_list(&mut buffer, &vec!["node_modules".to_owned()]).unwrap();
    assert!(String::from_utf8(buffer.clone())
        .unwrap()
        .contains("Excluding:"));

    // Test spinner and progress bar (smoke tests as they are hidden in test mode)
    let _pb = output::create_progress_bar(10);
    let _spinner = output::create_spinner();

    // Re-enable colors
    colored::control::unset_override();
}

#[test]
fn test_github_reporter_escaping() {
    // Add % to path to test suffix escaping
    let file = PathBuf::from("path/with,comma/and:colon/file%percentage.py");
    let mut result = create_mock_result();
    // Clear existing findings to focus on our specific test case
    result.danger.clear();

    result.danger.push(Finding {
        rule_id: "TEST-001".to_owned(),
        category: "Security".to_owned(),
        severity: "HIGH".to_owned(),
        // Message with newline and percentage
        message: "Message with \n newline and % percentage".to_owned(),
        file: file.clone(),
        line: 10,
        col: 5,
        cell_number: None,
    });

    let mut buffer = Vec::new();
    github::print_github(&mut buffer, &result).unwrap();
    let output = String::from_utf8(buffer).unwrap();

    // Verify escaping:
    // : -> %3A
    // , -> %2C
    // \n -> %0A
    // % -> %25 (in message and property)

    // Expected file path in property: path/with%2Ccomma/and%3Acolon/file%25percentage.py
    assert!(output.contains("file=path/with%2Ccomma/and%3Acolon/file%25percentage.py"));

    // Expected message in data: Message with %0A newline and %25 percentage
    assert!(output.contains("::Message with %0A newline and %25 percentage"));

    // Expected suffix in data: (path/with,comma/and:colon/file%25percentage.py:10)
    // Note: , and : are valid in message body, so they are NOT escaped in the suffix (via escape_message).
    assert!(output.contains("(path/with,comma/and:colon/file%25percentage.py:10)"));
}
