//! Regression tests that prevent name-only danger-rule suppression.

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

#[test]
fn test_security_sounding_names_do_not_suppress_findings() {
    let cases = [
        (
            cytoscnpy::rules::ids::RULE_ID_SUBPROCESS,
            r"
import os

def run(cmd):
    sanitized_cmd = cmd
    os.system(sanitized_cmd)
",
        ),
        (
            cytoscnpy::rules::ids::RULE_ID_SQL_INJECTION,
            r"
def lookup(cursor, query):
    sanitized_query = query
    cursor.execute(sanitized_query)
",
        ),
        (
            cytoscnpy::rules::ids::RULE_ID_SQL_RAW,
            r"
import sqlalchemy

def lookup(query):
    sanitized_query = query
    return sqlalchemy.text(sanitized_query)
",
        ),
        (
            cytoscnpy::rules::ids::RULE_ID_PATH_TRAVERSAL,
            r"
import os

def read():
    path = input()
    sanitized_path = path
    return os.path.abspath(sanitized_path)
",
        ),
        (
            cytoscnpy::rules::ids::RULE_ID_SSRF,
            r"
import requests

def fetch(url):
    validated_url = url
    return requests.get(validated_url)
",
        ),
        (
            cytoscnpy::rules::ids::RULE_ID_URL_OPEN,
            r"
import urllib.request

def fetch(url):
    validated_url = url
    return urllib.request.urlopen(validated_url)
",
        ),
    ];

    for (rule_id, source) in cases {
        let json = analyze_source(source);
        assert!(
            has_danger_rule(&json, rule_id),
            "{rule_id} must not trust a variable name as validation evidence"
        );
    }
}
