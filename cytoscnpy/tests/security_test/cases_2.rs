use super::*;

#[test]
fn test_subprocess_with_args_keyword_and_shell_true() {
    let source = r#"
import subprocess
user_input = "rm -rf /"
subprocess.call(shell=True, args=user_input)
"#;
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D003"));
}

#[test]
fn test_subprocess_shell_true_with_fstring() {
    let source = r#"
import subprocess
user_input = "test"
subprocess.run(f"echo {user_input}", shell=True)
"#;
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D003"));
}

#[test]
fn test_subprocess_shell_true_with_concatenation() {
    let source = r#"
import subprocess
user_input = "test"
subprocess.run("echo " + user_input, shell=True)
"#;
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D003"));
}

#[test]
fn test_subprocess_shell_true_literal_args_is_ok() {
    let source = r#"
import subprocess
subprocess.run("echo hello", shell=True)
"#;
    scan_danger!(source, linter);
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D003"));
}

#[test]
fn test_subprocess_shell_true_with_literal_list_is_ok() {
    let source = r#"
import subprocess
subprocess.run(shell=True, args=["echo", "hello"])
"#;
    scan_danger!(source, linter);
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D003"));
}

#[test]
fn test_subprocess_shell_false_dynamic_args_is_ok() {
    // shell=False with dynamic args is generally safe
    let source = r#"
import subprocess
user_input = "test"
subprocess.call(shell=False, args=user_input)
"#;
    scan_danger!(source, linter);
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D003"));
}

#[test]
fn test_requests_default_verify_true_is_ok() {
    let source = "import requests\nrequests.get('https://example.com')\n";
    scan_danger!(source, linter);
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D401"));
}

#[test]
fn test_sql_execute_interpolated_flags() {
    let source = r#"
def f(cur, name):
    # f-string interpolation -> should flag CSP-D101
    cur.execute(f"SELECT * FROM users WHERE name = '{name}'")
"#;
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D101"));
}

#[test]
fn test_sql_execute_parameterized_ok() {
    let source = r#"
def f(cur, name):
    cur.execute("SELECT * FROM users WHERE name = %s", (name,))
"#;
    scan_danger!(source, linter);
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D101"));
}

// --- SECRETS TESTS ---

#[test]
fn test_sql_execute_format_unsafe() {
    let source = r#"
def f(cur, name):
    # .format() -> should flag CSP-D101
    cur.execute("SELECT * FROM users WHERE name = '{}'".format(name))
"#;
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D101"));
}

#[test]
fn test_sql_execute_percent_unsafe() {
    let source = r#"
def f(cur, name):
    # % formatting -> should flag CSP-D101
    cur.execute("SELECT * FROM users WHERE name = '%s'" % name)
"#;
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D101"));
}

#[test]
fn test_aws_key_detection() {
    let source = r#"
AWS_ACCESS_KEY_ID = "AKIAIOSFODNN7EXAMPLE"
AWS_SECRET_ACCESS_KEY = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
"#;
    let findings = scan_secrets_compat(source, &PathBuf::from("test.py"));
    assert!(findings
        .iter()
        .any(|f| f.message.contains("AWS Access Key")));
}

#[test]
fn test_github_token_detection() {
    let source = "GITHUB_TOKEN = \"ghp_1234567890abcdef1234567890abcdef1234\"\n";
    let findings = scan_secrets_compat(source, &PathBuf::from("test.py"));
    assert!(findings
        .iter()
        .any(|f| f.message.to_lowercase().contains("github")));
}

#[test]
fn test_gitlab_pat_detection() {
    let source = "GITLAB_PAT = \"glpat-A1b2C3d4E5f6G7h8I9j0\"\n";
    let findings = scan_secrets_compat(source, &PathBuf::from("test.py"));
    assert!(findings
        .iter()
        .any(|f| f.message.to_lowercase().contains("gitlab")));
}

#[test]
fn test_slack_bot_detection() {
    let source = "SLACK_BOT = \"xoxb-1234567890ABCDEF12\"\n";
    let findings = scan_secrets_compat(source, &PathBuf::from("test.py"));
    assert!(findings
        .iter()
        .any(|f| f.message.to_lowercase().contains("slack")));
}

#[test]
fn test_stripe_key_detection() {
    let source = "STRIPE = \"sk_live_123456789012345678901234\"\n";
    let findings = scan_secrets_compat(source, &PathBuf::from("test.py"));
    assert!(findings
        .iter()
        .any(|f| f.message.to_lowercase().contains("stripe")));
}

#[test]
fn test_private_key_detection() {
    let source = "PK = \"-----BEGIN RSA PRIVATE KEY-----\"\n";
    let findings = scan_secrets_compat(source, &PathBuf::from("test.py"));
    assert!(findings
        .iter()
        .any(|f| f.message.to_lowercase().contains("private key")));
}

#[test]
fn test_ignore_directive_suppresses_matches() {
    let source =
        "GITHUB_TOKEN = \"ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"  # pragma: no cytoscnpy\n";
    let findings = scan_secrets_compat(source, &PathBuf::from("test.py"));
    assert!(findings.is_empty());
}

#[test]
fn test_no_secrets_in_clean_code() {
    let source = r#"
def calculate(x, y):
    return x + y

API_URL = "https://api.example.com"
"#;
    let findings = scan_secrets_compat(source, &PathBuf::from("test.py"));
    assert_eq!(findings.len(), 0);
}

#[test]
fn test_aws_key_in_comment_detected_by_default() {
    let source = r#"
# TODO: Remove this hardcoded key
# AWS_ACCESS_KEY_ID = "AKIAIOSFODNN7EXAMPLE"
"#;
    // Default config should scan comments
    let findings = scan_secrets_compat(source, &PathBuf::from("test.py"));
    assert!(
        findings.iter().any(|f| f.message.contains("AWS")),
        "Should detect AWS key in comment by default"
    );
}

#[test]
fn test_github_token_in_comment_detected() {
    let source = r#"
# OLD TOKEN: ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
"#;
    let findings = scan_secrets_compat(source, &PathBuf::from("test.py"));
    assert!(
        findings.iter().any(|f| f.message.contains("GitHub")),
        "Should detect GitHub token in comment"
    );
}

#[test]
fn test_secret_in_comment_ignored_with_pragma() {
    let source = r#"
# AWS_ACCESS_KEY_ID = "AKIAIOSFODNN7EXAMPLE" # pragma: no cytoscnpy
"#;
    let findings = scan_secrets_compat(source, &PathBuf::from("test.py"));
    assert!(
        findings.is_empty(),
        "Should respect pragma directive in comments"
    );
}

#[test]
fn test_skip_comments_when_disabled() {
    let source = r#"
# AWS_ACCESS_KEY_ID = "AKIAIOSFODNN7EXAMPLE"
"#;
    // Create config with scan_comments disabled
    let mut config = SecretsConfig::default();
    config.scan_comments = false;

    let findings = scan_secrets(
        source,
        &PathBuf::from("test.py"),
        &config,
        None::<&rustc_hash::FxHashSet<usize>>,
        false,
    );
    assert!(
        findings.is_empty(),
        "Should skip comments when scan_comments is false"
    );
}

// --- NEW BANDIT TESTS (CSP-D105, D403, D404, D405, D504) ---

#[test]
fn test_assert_used() {
    let source = "assert 1 == 1\n";
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D701"));
}

#[test]
fn test_debug_mode_enabled() {
    let source = r#"
app.run(debug=True)
run_simple(debug=True)
"#;
    scan_danger!(source, linter);
    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.rule_id == "CSP-D403")
        .collect();
    assert_eq!(findings.len(), 2);
}

#[test]
fn test_debug_mode_disabled_ok() {
    let source = r#"
app.run(debug=False)
run_simple()
"#;
    scan_danger!(source, linter);
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D403"));
}
