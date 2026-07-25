//! Cross-format coverage for clone and dependency findings.

#![allow(clippy::unwrap_used)]

mod all_format_support;

use all_format_support::extended_result;
use cytoscnpy::output;
use cytoscnpy::report::{gitlab, json, junit, markdown, sarif};
use serde_json::Value;

const RULE_IDS: [&str; 6] = [
    "CSP-C100", "CSP-R001", "CSP-R002", "CSP-R003", "CSP-R004", "CSP-R005",
];

fn rendered(formatter: impl FnOnce(&mut Vec<u8>) -> std::io::Result<()>) -> String {
    let mut buffer = Vec::new();
    formatter(&mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

fn assert_all_rule_ids(output: &str) {
    for rule_id in RULE_IDS {
        assert!(output.contains(rule_id), "missing {rule_id} in:\n{output}");
    }
}

fn issue_for_rule<'a>(report: &'a Value, rule_id: &str) -> &'a Value {
    report
        .as_array()
        .unwrap()
        .iter()
        .find(|issue| issue["check_name"].as_str() == Some(rule_id))
        .unwrap()
}

#[test]
fn structured_formats_include_all_extended_findings() {
    let result = extended_result();

    let junit = rendered(|writer| junit::print_junit(writer, &result));
    assert!(junit.contains(r#"tests="6" failures="6""#));
    assert_all_rule_ids(&junit);

    let markdown = rendered(|writer| markdown::print_markdown(writer, &result));
    assert!(markdown.contains("## Clone Findings"));
    assert!(markdown.contains("## Dependency Findings"));
    assert_all_rule_ids(&markdown);

    let sarif = rendered(|writer| sarif::print_sarif(writer, &result));
    assert_all_rule_ids(&sarif);
    let sarif: Value = serde_json::from_str(&sarif).unwrap();
    let clone = sarif["runs"][0]["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["ruleId"].as_str() == Some("CSP-C100"))
        .unwrap();
    assert_eq!(clone["level"], "warning");
}

#[test]
fn gitlab_extended_fingerprints_are_stable() {
    let first_root = tempfile::tempdir().unwrap();
    let second_root = tempfile::tempdir().unwrap();
    let render_at_root = |root: &std::path::Path| {
        let mut result = extended_result();
        result.clones[0].file = root.join("src/clone.py");
        serde_json::from_str::<Value>(&rendered(|writer| {
            gitlab::print_gitlab_with_root(writer, &result, Some(root))
        }))
        .unwrap()
    };

    let first = render_at_root(first_root.path());
    let second = render_at_root(second_root.path());
    let first_clone = issue_for_rule(&first, "CSP-C100");
    let second_clone = issue_for_rule(&second, "CSP-C100");

    assert_eq!(first_clone["location"]["path"], "src/clone.py");
    assert_eq!(first_clone["fingerprint"], second_clone["fingerprint"]);
    assert_all_rule_ids(&first.to_string());
}

#[test]
fn text_grouped_and_quiet_cover_dependency_and_clone_counts() {
    let result = extended_result();
    colored::control::set_override(false);

    let text = rendered(|writer| output::print_report(writer, &result));
    for heading in [
        "Missing Dependencies",
        "Unused Dependencies",
        "Transitive Dependencies",
        "Development Dependencies Used in Production",
        "Standard Library Dependencies",
    ] {
        assert!(text.contains(heading), "missing heading {heading}");
    }

    let grouped = rendered(|writer| output::print_report_grouped(writer, &result));
    for rule_id in &RULE_IDS[1..] {
        assert!(grouped.contains(rule_id), "missing {rule_id}");
    }

    let quiet = rendered(|writer| output::print_report_quiet(writer, &result));
    assert!(quiet.contains("0 unused code issues, 5 dependency issues"));
    assert!(quiet.contains("0 security/quality/parse issues, 1 clone findings"));
    colored::control::unset_override();
}

#[test]
fn json_contains_raw_vectors_and_stable_extended_findings() {
    let mut result = extended_result();
    let mut canonical = result.clones[0].clone();
    canonical.file = "src/original.py".into();
    canonical.message = "Canonical clone implementation".to_owned();
    canonical.severity = "INFO".to_owned();
    canonical.is_duplicate = false;
    result.clones.push(canonical);
    let payload: Value =
        serde_json::from_str(&json::machine_json_payload(&result).unwrap()).unwrap();

    assert_eq!(payload["schema_version"], "2");
    assert_eq!(payload["clones"].as_array().unwrap().len(), 2);
    assert_eq!(payload["unused_dependencies"].as_array().unwrap().len(), 1);
    let stable = payload["stable_findings"].as_array().unwrap();
    assert_eq!(
        stable
            .iter()
            .filter(|finding| finding["kind"].as_str() == Some("clone"))
            .count(),
        1
    );
    assert_all_rule_ids(&Value::Array(stable.clone()).to_string());
}

#[cfg(feature = "html_report")]
#[test]
fn html_lists_dependencies_and_keeps_clones_on_clone_page() {
    use cytoscnpy::report::generator::generate_report;

    let mut target = std::env::current_dir().unwrap();
    target.push("target");
    target.push("all-format-html-tests");
    std::fs::create_dir_all(&target).unwrap();
    let dir = tempfile::Builder::new()
        .prefix("extended_")
        .tempdir_in(target)
        .unwrap();

    generate_report(&extended_result(), std::path::Path::new("."), dir.path()).unwrap();
    let issues = std::fs::read_to_string(dir.path().join("issues.html")).unwrap();
    let clones = std::fs::read_to_string(dir.path().join("clones.html")).unwrap();
    let dashboard = std::fs::read_to_string(dir.path().join("index.html")).unwrap();

    for rule_id in &RULE_IDS[1..] {
        assert!(issues.contains(rule_id), "missing {rule_id}");
    }
    assert!(clones.contains(r#"data-name="duplicate""#));
    assert!(clones.contains(r#"data-type="Exact Copy""#));
    assert!(dashboard.replace("\r\n", "\n").contains(">\n        5\n"));
}
