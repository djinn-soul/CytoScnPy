//! Unit tests for security rules
//! Tests secrets and dangerous code detection
#![allow(
    clippy::unwrap_used,
    clippy::needless_raw_string_hashes,
    clippy::field_reassign_with_default
)]

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
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D201"));
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
tar = tarfile.open("archive.tar")
tar.extractall()
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
tar = tarfile.open("archive.tar")
tar.extractall(filter='data')
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
tar = tarfile.open("archive.tar")
tar.extractall(filter=my_custom_filter)
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
tar = tarfile.open("archive.tar")
tar.extractall(path="/tmp")
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
zip = zipfile.ZipFile("archive.zip")
zip.extractall()
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

// --- XML PARSING TESTS (CSP-D104) ---

#[test]
fn test_xml_etree_parse_unsafe() {
    let source = r#"
import xml.etree.ElementTree as ET
ET.parse("file.xml")
"#;
    scan_danger!(source, linter);
    let finding = linter.findings.iter().find(|f| f.rule_id == "CSP-D104");
    assert!(finding.is_some());
    assert_eq!(finding.unwrap().severity, "MEDIUM");
}

#[test]
fn test_xml_etree_fromstring_unsafe() {
    let source = r#"
import xml.etree.ElementTree as ET
ET.fromstring("<root>...</root>")
"#;
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D104"));
}

#[test]
fn test_xml_dom_minidom_parse_unsafe() {
    let source = r#"
import xml.dom.minidom
xml.dom.minidom.parse("file.xml")
"#;
    scan_danger!(source, linter);
    let finding = linter.findings.iter().find(|f| f.rule_id == "CSP-D104");
    assert!(finding.is_some());
    assert!(finding.unwrap().message.contains("minidom"));
}

#[test]
fn test_xml_sax_parse_unsafe() {
    let source = r#"
import xml.sax
xml.sax.parse("file.xml", None)
"#;
    scan_danger!(source, linter);
    let finding = linter.findings.iter().find(|f| f.rule_id == "CSP-D104");
    assert!(finding.is_some());
    assert!(finding.unwrap().message.contains("XXE"));
}

#[test]
fn test_xml_sax_make_parser_unsafe() {
    let source = r#"
import xml.sax
xml.sax.make_parser()
"#;
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D104"));
}

#[test]
fn test_lxml_etree_parse_high_severity() {
    let source = r#"
import lxml.etree
lxml.etree.parse("file.xml")
"#;
    scan_danger!(source, linter);
    let finding = linter.findings.iter().find(|f| f.rule_id == "CSP-D104");
    assert!(finding.is_some());
    assert_eq!(finding.unwrap().severity, "HIGH");
}

#[test]
fn test_lxml_etree_fromstring_high_severity() {
    let source = r#"
import lxml.etree
lxml.etree.fromstring("<root>...</root>")
"#;
    scan_danger!(source, linter);
    let finding = linter.findings.iter().find(|f| f.rule_id == "CSP-D104");
    assert!(finding.is_some());
    assert_eq!(finding.unwrap().severity, "HIGH");
    assert!(finding.unwrap().message.contains("XXE"));
}

#[test]
fn test_defusedxml_is_safe() {
    // defusedxml with a different alias should NOT trigger findings
    // Note: ET.parse is flagged because we can't track import aliases statically
    // Using a different alias name demonstrates safe xml parsing
    let source = r#"
import defusedxml.ElementTree as SafeET
SafeET.parse("file.xml")
SafeET.fromstring("<root>...</root>")
"#;
    scan_danger!(source, linter);
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D104"));
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

#[test]
fn test_hardcoded_bind_all_interfaces() {
    let source = r#"
HOST = "0.0.0.0"
IPV6_HOST = "::"
"#;
    scan_danger!(source, linter);
    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.rule_id == "CSP-D404")
        .collect();
    assert_eq!(findings.len(), 2);
}

#[test]
fn test_hardcoded_bind_safe_address() {
    let source = r#"
HOST = "127.0.0.1"
"#;
    scan_danger!(source, linter);
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D404"));
}

#[test]
fn test_request_without_timeout() {
    let source = r#"
import requests
requests.get("https://example.com")
requests.post("https://example.com", data={})
"#;
    scan_danger!(source, linter);
    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.rule_id == "CSP-D405")
        .collect();
    assert_eq!(findings.len(), 2);
}

#[test]
fn test_request_with_timeout_ok() {
    let source = r#"
import requests
requests.get("https://example.com", timeout=10)
requests.post("https://example.com", timeout=5)
"#;
    scan_danger!(source, linter);
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D405"));
}

#[test]
fn test_tempfile_mktemp_unsafe() {
    let source = r#"
import tempfile
tempfile.mktemp()
"#;
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D504"));
}

#[test]
fn test_tempfile_mkstemp_safe() {
    let source = r#"
import tempfile
tempfile.mkstemp()
tempfile.TemporaryFile()
"#;
    scan_danger!(source, linter);

    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D504"));
}

// --- NEW RULES TESTS (CSP-D004, D505, D106) ---

#[test]
fn test_insecure_imports() {
    let source = r#"
import telnetlib
import ftplib
import pyghmi
import Crypto.Cipher
from wsgiref.handlers import CGIHandler
"#;
    scan_danger!(source, linter);
    let ids: Vec<_> = linter.findings.iter().map(|f| &f.rule_id).collect();
    // Expect 5 findings
    assert_eq!(linter.findings.len(), 5);
    // Insecure Import is CSP-D702
    assert!(ids.iter().all(|id| *id == "CSP-D702"));
}

#[test]
fn test_bad_file_permissions() {
    let source = r#"
import os
import stat
os.chmod("file", stat.S_IWOTH)
os.chmod("file", mode=stat.S_IWOTH)
"#;
    scan_danger!(source, linter);
    assert_eq!(linter.findings.len(), 2);
    assert!(linter.findings.iter().all(|f| f.rule_id == "CSP-D505"));
}

#[test]
fn test_jinja2_autoescape_false() {
    let source = r#"
import jinja2
env = jinja2.Environment(autoescape=False)
env2 = jinja2.Environment(loader=x, autoescape=False)
"#;
    scan_danger!(source, linter);
    assert_eq!(linter.findings.len(), 2);
    assert!(linter.findings.iter().all(|f| f.rule_id == "CSP-D703"));
}

#[test]
fn test_jinja2_autoescape_true_ok() {
    let source = r#"
import jinja2
env = jinja2.Environment(autoescape=True)
"#;
    scan_danger!(source, linter);
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D703"));
}

#[test]
fn test_blacklist_calls() {
    let source = r#"
import marshal
import hashlib
import urllib
import telnetlib
import ftplib
import ssl
from django.utils.safestring import mark_safe

marshal.load(f)
hashlib.md5("abc")
urllib.urlopen("http://evil.com")
telnetlib.Telnet("host")
ftplib.FTP("host")
input("prompt")
ssl._create_unverified_context()
mark_safe("<div>")
"#;
    scan_danger!(source, linter);
    let ids: Vec<String> = linter.findings.iter().map(|f| f.rule_id.clone()).collect();

    // Check coverage of B3xx rules
    assert!(ids.contains(&"CSP-D203".to_owned())); // marshal
    assert!(ids.contains(&"CSP-D301".to_owned())); // md5
    assert!(ids.contains(&"CSP-D406".to_owned()) || ids.contains(&"CSP-D410".to_owned())); // urllib
    assert!(ids.contains(&"CSP-D409".to_owned()) || ids.contains(&"CSP-D702".to_owned())); // telnet func
    assert!(ids.contains(&"CSP-D406".to_owned()) || ids.contains(&"CSP-D702".to_owned())); // ftp func
    assert!(ids.contains(&"CSP-D005".to_owned())); // input
    assert!(ids.contains(&"CSP-D408".to_owned())); // ssl (D408 is SSL Unverified, wait context is D408?)
    assert!(ids.contains(&"CSP-D105".to_owned())); // mark_safe
}

#[test]
fn test_pickle_expansion() {
    let source = r#"
import dill
import shelve
import jsonpickle
import pandas

dill.loads(x)
shelve.open("db")
jsonpickle.decode(data)
pandas.read_pickle("file")
"#;
    scan_danger!(source, linter);
    // 6 findings: 2 imports (CSP-D004) + 4 calls (CSP-D201)
    assert_eq!(linter.findings.len(), 6);
    let ids: Vec<String> = linter.findings.iter().map(|f| f.rule_id.clone()).collect();
    assert!(ids.contains(&"CSP-D702".to_owned()));
    assert!(ids.contains(&"CSP-D201".to_owned()));
}

#[test]
fn test_random_rule() {
    let source = r#"
import random
random.random()
random.randint(1, 10)
random.choice([1, 2])
"#;
    scan_danger!(source, linter);
    assert_eq!(linter.findings.len(), 3);
    assert!(linter.findings.iter().all(|f| f.rule_id == "CSP-D311"));
}

#[test]
fn test_xml_extras_and_urllib2() {
    let source = r#"
import xml.dom.pulldom
import xml.dom.expatbuilder
import urllib2
import six

xml.dom.pulldom.parse("file")
xml.dom.expatbuilder.parse("file")
urllib2.urlopen("http://evil.com")
six.moves.urllib.request.urlopen("http://evil.com")
"#;
    scan_danger!(source, linter);
    // 6 findings: 2 imports (CSP-D004) + 4 calls (CSP-D104/D406)
    assert_eq!(linter.findings.len(), 6);

    let ids: Vec<String> = linter.findings.iter().map(|f| f.rule_id.clone()).collect();
    assert!(ids.contains(&"CSP-D104".to_owned())); // pulldom
    assert!(ids.contains(&"CSP-D104".to_owned())); // expatbuilder
    assert!(ids.contains(&"CSP-D410".to_owned())); // urllib2
    assert!(ids.contains(&"CSP-D410".to_owned())); // six.moves
}

#[test]
fn test_b320_lxml_etree() {
    let source = r#"
import lxml.etree as etree
etree.parse("file.xml")
etree.fromstring("<root/>")
etree.RestrictedElement()
etree.GlobalParserTLS()
etree.getDefaultParser()
etree.check_docinfo(doc)
"#;
    scan_danger!(source, linter);
    // 6 calls (CSP-D104) + 1 import (CSP-D004) = 7 findings
    assert_eq!(linter.findings.len(), 7);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D702")); // import
    assert!(
        linter
            .findings
            .iter()
            .filter(|f| f.rule_id == "CSP-D104")
            .count()
            == 6
    ); // calls
}

#[test]
fn test_b325_tempnam() {
    let source = r#"
import os
os.tempnam("/tmp", "prefix")
os.tmpnam()
"#;
    scan_danger!(source, linter);
    assert_eq!(linter.findings.len(), 2);
    assert!(linter.findings.iter().all(|f| f.rule_id == "CSP-D506"));
}

#[test]
fn test_blacklist_imports_full_coverage() {
    let source = r#"
import telnetlib
import ftplib
import pickle
import cPickle
import dill
import shelve
import subprocess
import xml.etree.ElementTree
import xml.sax
import xmlrpc
import Crypto.Cipher
from wsgiref.handlers import CGIHandler
import pyghmi
import lxml.etree
"#;
    scan_danger!(source, linter);

    // We expect 14 findings (all imports should be flagged)
    assert_eq!(linter.findings.len(), 14);

    let findings = &linter.findings;

    // Verify high severity
    let high = findings.iter().filter(|f| f.severity == "HIGH").count();
    assert_eq!(high, 6); // telnetlib, ftplib, xmlrpc, Crypto, wsgiref, pyghmi

    // Verify low severity
    let low = findings.iter().filter(|f| f.severity == "LOW").count();
    assert_eq!(low, 8); // pickle, cPickle, dill, shelve, subprocess, xml.etree, xml.sax, lxml

    // Verify rule IDs are generally correct (mostly Imports or Insecure calls)
    assert!(findings.iter().all(|f| f.rule_id == "CSP-D702"
        || f.rule_id == "CSP-D003"
        || f.rule_id == "CSP-D409"
        || f.rule_id == "CSP-D406"));
}

#[test]
fn test_b324_hashlib_new_unsafe() {
    let source = r#"
import hashlib
hashlib.new("md5", b"data")
hashlib.new("sha1", b"data")
hashlib.new("MD5", b"data")
hashlib.new("sha256", b"data") # secure
"#;
    scan_danger!(source, linter);
    assert_eq!(linter.findings.len(), 3);
    // MD5 should be CSP-D301, SHA1 should be CSP-D302
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D301"));
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D302"));
}

#[test]
fn test_b309_https_connection_unsafe() {
    let source = r#"
import http.client
import ssl
http.client.HTTPSConnection("host")
http.client.HTTPSConnection("host", context=ssl.create_default_context()) # safe
"#;
    scan_danger!(source, linter);
    assert_eq!(linter.findings.len(), 1);
    assert!(linter.findings.iter().all(|f| f.rule_id == "CSP-D407"));
}

#[test]
fn test_request_timeout_refined() {
    let source = r#"
import requests
import httpx
requests.get("url", timeout=None) # unsafe
requests.post("url", timeout=0)    # unsafe
httpx.post("url", timeout=False)  # unsafe
httpx.get("url", timeout=5)       # safe
requests.get("url", timeout=5.0)  # safe
"#;
    scan_danger!(source, linter);
    assert_eq!(linter.findings.len(), 3);
    assert!(linter.findings.iter().all(|f| f.rule_id == "CSP-D405"));
}

#[test]
fn test_socket_bind_positional() {
    let source = r#"
import socket
s = socket.socket()
s.bind(("0.0.0.0", 80)) # unsafe
s.bind(("127.0.0.1", 80)) # safe
"#;
    scan_danger!(source, linter);
    assert_eq!(linter.findings.len(), 1);
    assert_eq!(linter.findings[0].rule_id, "CSP-D404");
}
