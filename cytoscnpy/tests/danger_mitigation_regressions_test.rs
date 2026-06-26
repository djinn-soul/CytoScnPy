//! Regression tests for validation-aware danger-rule suppression.

#![allow(clippy::expect_used)]

use cytoscnpy::entry_point::run_with_args_to;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    TempDir::new().expect("Failed to create temp dir")
}

fn run_json(args: Vec<String>) -> Value {
    let mut output = Vec::new();
    run_with_args_to(args, &mut output).expect("CLI run failed");
    let stdout = String::from_utf8(output).expect("Invalid UTF-8 output");
    serde_json::from_str(&stdout).expect("Failed to parse JSON output")
}

fn has_danger_rule(json: &Value, rule_id: &str) -> bool {
    json["danger"]
        .as_array()
        .expect("danger should be an array")
        .iter()
        .any(|finding| finding["rule_id"] == rule_id)
}

fn analyze_source(source: &str) -> Value {
    let dir = project_tempdir();
    let file_path = dir.path().join("app.py");
    fs::write(&file_path, source).expect("Failed to write test file");

    run_json(vec![
        "--json".to_owned(),
        "--danger".to_owned(),
        file_path.to_string_lossy().to_string(),
    ])
}

fn assert_mitigation_pair(rule_id: &str, unsafe_source: &str, mitigated_source: &str) {
    let unsafe_json = analyze_source(unsafe_source);
    assert!(
        has_danger_rule(&unsafe_json, rule_id),
        "Expected unsafe fixture to report {rule_id}"
    );

    let mitigated_json = analyze_source(mitigated_source);
    assert!(
        !has_danger_rule(&mitigated_json, rule_id),
        "Expected mitigated fixture to suppress {rule_id}"
    );
}

#[test]
fn test_mitigation_aware_command_sql_and_path_rules() {
    assert_mitigation_pair(
        cytoscnpy::rules::ids::RULE_ID_SUBPROCESS,
        r"
import os

def run(cmd):
    os.system(cmd)
",
        r"
import os

def run(cmd):
    sanitized_cmd = cmd
    os.system(sanitized_cmd)
",
    );

    assert_mitigation_pair(
        cytoscnpy::rules::ids::RULE_ID_SQL_INJECTION,
        r"
def lookup(cursor, query):
    cursor.execute(query)
",
        r"
def lookup(cursor, query):
    sanitized_query = query
    cursor.execute(sanitized_query)
",
    );

    assert_mitigation_pair(
        cytoscnpy::rules::ids::RULE_ID_SQL_RAW,
        r"
import sqlalchemy

def lookup(query):
    return sqlalchemy.text(query)
",
        r"
import sqlalchemy

def lookup(query):
    sanitized_query = query
    return sqlalchemy.text(sanitized_query)
",
    );

    assert_mitigation_pair(
        cytoscnpy::rules::ids::RULE_ID_PATH_TRAVERSAL,
        r"
import os

def read():
    path = input()
    return os.path.abspath(path)
",
        r"
import os

def read():
    path = input()
    sanitized_path = path
    return os.path.abspath(sanitized_path)
",
    );
}

#[test]
fn test_mitigation_aware_url_rules() {
    assert_mitigation_pair(
        cytoscnpy::rules::ids::RULE_ID_SSRF,
        r"
import requests

def fetch(url):
    return requests.get(url)
",
        r"
import requests

def fetch(url):
    validated_url = url
    return requests.get(validated_url)
",
    );

    assert_mitigation_pair(
        cytoscnpy::rules::ids::RULE_ID_URL_OPEN,
        r"
import urllib.request

def fetch(url):
    return urllib.request.urlopen(url)
",
        r"
import urllib.request

def fetch(url):
    validated_url = url
    return urllib.request.urlopen(validated_url)
",
    );
}
