//! The streamed machine JSON payload must match `serde_json` pretty-printing
//! of the same value: identical bytes, keys sorted at every level.
#![allow(clippy::unwrap_used)]

mod all_format_support;

use all_format_support::extended_result;
use cytoscnpy::analyzer::{AnalysisResult, CytoScnPy};
use cytoscnpy::report::json;
use serde_json::Value;

const SOURCE: &str = r#"
import os
import subprocess

AWS_SECRET_ACCESS_KEY = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"

def unused_function(unused_parameter):
    unused_variable = 1
    return eval(os.environ["CODE"])

class UnusedClass:
    def unused_method(self):
        subprocess.call("ls " + os.environ["DIR"], shell=True)
"#;

fn analyzed_result() -> AnalysisResult {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("module.py"), SOURCE).unwrap();
    std::fs::write(dir.path().join("other.py"), "import json\n").unwrap();
    let mut analyzer = CytoScnPy::default()
        .with_confidence(0)
        .with_secrets(true)
        .with_danger(true)
        .with_quality(true);
    let mut result = analyzer.analyze(dir.path());

    // Add clone and dependency findings, including `missing_dependency_details`.
    let extended = extended_result();
    result.clones = extended.clones;
    result.unused_dependencies = extended.unused_dependencies;
    result.missing_dependencies = extended.missing_dependencies;
    result.missing_dependency_details = extended.missing_dependency_details;
    result.transitive_dependencies = extended.transitive_dependencies;
    result.dev_dependencies_in_production = extended.dev_dependencies_in_production;
    result.stdlib_dependencies = extended.stdlib_dependencies;
    result
}

fn assert_matches_serde_json(result: &AnalysisResult) {
    let payload = json::machine_json_payload(result).unwrap();
    let parsed: Value = serde_json::from_str(&payload).unwrap();
    // Without `preserve_order`, reprinting sorts keys at every level.
    assert_eq!(payload, serde_json::to_string_pretty(&parsed).unwrap());

    let mut expected = serde_json::to_value(result).unwrap();
    let fields = expected.as_object_mut().unwrap();
    fields.insert("schema_version".to_owned(), Value::from("2"));
    fields.insert(
        "stable_findings".to_owned(),
        parsed["stable_findings"].clone(),
    );
    assert_eq!(parsed, expected);
}

#[test]
fn populated_payload_matches_serde_json_rendering() {
    let result = analyzed_result();
    assert!(
        !result.unused_functions.is_empty()
            && !result.unused_methods.is_empty()
            && !result.unused_classes.is_empty()
            && !result.unused_imports.is_empty()
            && !result.danger.is_empty()
            && !result.secrets.is_empty(),
        "fixture should populate every major section"
    );
    assert!(!result.missing_dependency_details.is_empty());
    assert_matches_serde_json(&result);

    let stable = &serde_json::from_str::<Value>(&json::machine_json_payload(&result).unwrap())
        .unwrap()["stable_findings"];
    let ids: Vec<&str> = stable
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["stable_id"].as_str().unwrap())
        .collect();
    assert!(ids.is_sorted(), "stable findings must be sorted: {ids:?}");
    assert!(ids.iter().any(|id| id.starts_with("unused_function:")));
    assert!(ids.iter().any(|id| id.starts_with("secret:")));
}

#[test]
fn empty_payload_matches_serde_json_rendering() {
    assert_matches_serde_json(&AnalysisResult::default());
}

#[test]
fn streamed_payload_matches_returned_payload() {
    let result = analyzed_result();
    let mut streamed = Vec::new();
    json::write_machine_json(&result, &mut streamed).unwrap();
    assert_eq!(
        String::from_utf8(streamed).unwrap(),
        json::machine_json_payload(&result).unwrap()
    );
}
