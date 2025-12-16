//! Unit tests for security rules
//! Tests secrets and dangerous code detection

use cytoscnpy::config::Config;
use cytoscnpy::linter::LinterVisitor;
use cytoscnpy::rules::danger::get_danger_rules;
use cytoscnpy::rules::secrets::scan_secrets_compat;
use cytoscnpy::utils::LineIndex;
use ruff_python_parser::{parse, Mode};
use std::path::PathBuf;

// --- DANGER TESTS ---

macro_rules! scan_danger {
    ($source:expr, $linter:ident) => {
        let tree = parse($source, Mode::Module.into()).expect("Failed to parse");
        let line_index = LineIndex::new($source);
        let rules = get_danger_rules();
        let config = Config::default();
        let mut $linter = LinterVisitor::new(rules, PathBuf::from("test.py"), line_index, config);

        if let ruff_python_ast::Mod::Module(module) = tree.into_syntax() {
            for stmt in &module.body {
                $linter.visit_stmt(stmt);
            }
        }
    };
}

#[test]
fn test_eval_detection() {
    let source = r#"
user_input = input("Enter code: ")
result = eval(user_input)
"#;
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D001"));
}

#[test]
fn test_exec_detection() {
    let source = r#"
code = "print('hello')"
exec(code)
"#;
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D002"));
}

#[test]
fn test_pickle_loads() {
    let source = "import pickle\npickle.loads(b'\\x80\\x04K\\x01.')\n";
    scan_danger!(source, linter);
    assert!(linter
        .findings
        .iter()
        .any(|f| f.rule_id == "CSP-D201-unsafe"));
}

#[test]
fn test_yaml_load_without_safeloader() {
    let source = "import yaml\nyaml.load('a: 1')\n";
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D202"));
}

#[test]
fn test_md5_sha1() {
    let source = "import hashlib\nhashlib.md5(b'd')\nhashlib.sha1(b'd')\n";
    scan_danger!(source, linter);
    let ids: Vec<_> = linter.findings.iter().map(|f| &f.rule_id).collect();
    assert!(ids.contains(&&"CSP-D301".to_owned()));
    assert!(ids.contains(&&"CSP-D302".to_owned()));
}

#[test]
fn test_subprocess_shell_true() {
    let source = "import subprocess\ncmd = 'echo hi'\nsubprocess.run(cmd, shell=True)\n";
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D003"));
}

#[test]
fn test_requests_verify_false() {
    let source = "import requests\nrequests.get('https://x', verify=False)\n";
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D401"));
}

#[test]
fn test_yaml_safe_loader_does_not_trigger() {
    let source = "import yaml\nfrom yaml import SafeLoader\nyaml.load('a: 1', Loader=SafeLoader)\n";
    scan_danger!(source, linter);
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D202"));
}

#[test]
fn test_tarfile_extractall_unsafe() {
    let source = r#"
import tarfile
t = tarfile.open("archive.tar")
t.extractall()
"#;
    scan_danger!(source, linter);
    let finding = linter.findings.iter().find(|f| f.rule_id == "CSP-D502");
    assert!(finding.is_some());
    assert_eq!(finding.unwrap().severity, "HIGH");
}

#[test]
fn test_tarfile_extractall_chained_call() {
    // Chained call: tarfile.open(...).extractall()
    let source = r#"
import tarfile
tarfile.open("archive.tar").extractall()
"#;
    scan_danger!(source, linter);
    let finding = linter.findings.iter().find(|f| f.rule_id == "CSP-D502");
    assert!(finding.is_some());
    assert_eq!(finding.unwrap().severity, "HIGH");
}

#[test]
fn test_tarfile_extractall_with_filter_data_is_safe() {
    let source = r#"
import tarfile
t = tarfile.open("archive.tar")
t.extractall(filter='data')
"#;
    scan_danger!(source, linter);
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D502"));
}

#[test]
fn test_tarfile_extractall_with_filter_tar_is_safe() {
    let source = r#"
import tarfile
tarfile.open("archive.tar").extractall(filter='tar')
"#;
    scan_danger!(source, linter);
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D502"));
}

#[test]
fn test_tarfile_extractall_with_nonliteral_filter() {
    // Non-literal filter should be flagged with MEDIUM severity
    let source = r#"
import tarfile
t = tarfile.open("archive.tar")
t.extractall(filter=my_custom_filter)
"#;
    scan_danger!(source, linter);
    let finding = linter.findings.iter().find(|f| f.rule_id == "CSP-D502");
    assert!(finding.is_some());
    assert_eq!(finding.unwrap().severity, "MEDIUM");
}

#[test]
fn test_tarfile_extractall_with_path_but_no_filter() {
    let source = r#"
import tarfile
t = tarfile.open("archive.tar")
t.extractall(path="/tmp")
"#;
    scan_danger!(source, linter);
    let finding = linter.findings.iter().find(|f| f.rule_id == "CSP-D502");
    assert!(finding.is_some());
    assert_eq!(finding.unwrap().severity, "HIGH");
}

#[test]
fn test_unrelated_extractall_lower_severity() {
    // Random object with extractall() should be MEDIUM (not HIGH)
    let source = r#"
some_object.extractall()
"#;
    scan_danger!(source, linter);
    let finding = linter.findings.iter().find(|f| f.rule_id == "CSP-D502");
    assert!(finding.is_some());
    assert_eq!(finding.unwrap().severity, "MEDIUM");
}

// --- ZIPFILE TESTS (CSP-D503) ---

#[test]
fn test_zipfile_extractall_unsafe() {
    let source = r#"
import zipfile
z = zipfile.ZipFile("archive.zip")
z.extractall()
"#;
    scan_danger!(source, linter);
    let finding = linter.findings.iter().find(|f| f.rule_id == "CSP-D503");
    assert!(finding.is_some());
    assert_eq!(finding.unwrap().severity, "HIGH");
}

#[test]
fn test_zipfile_extractall_chained() {
    let source = r#"
import zipfile
zipfile.ZipFile("archive.zip").extractall()
"#;
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D503"));
}

#[test]
fn test_zipfile_extractall_variable_named_zip() {
    let source = r#"
zip_archive.extractall()
"#;
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D503"));
}

#[test]
fn test_unrelated_extractall_no_zipfile_finding() {
    // Random object that doesn't look like zipfile
    let source = r#"
my_data.extractall()
"#;
    scan_danger!(source, linter);
    // Should NOT trigger CSP-D503 (zipfile rule)
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D503"));
}

#[test]
fn test_subprocess_without_shell_true_is_ok() {
    let source = "import subprocess\nsubprocess.run(['echo','hi'])\n";
    scan_danger!(source, linter);
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D003"));
}

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

// --- SECRETS IN COMMENTS TESTS ---
// These tests verify that secrets in comments are found by default

use cytoscnpy::config::SecretsConfig;
use cytoscnpy::rules::secrets::scan_secrets;

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

    let findings = scan_secrets(source, &PathBuf::from("test.py"), &config, None);
    assert!(
        findings.is_empty(),
        "Should skip comments when scan_comments is false"
    );
}
