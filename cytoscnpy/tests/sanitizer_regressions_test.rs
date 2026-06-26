//! Regression coverage for rule-scoped sanitizer propagation.

use cytoscnpy::rules::ids;
use cytoscnpy::taint::analyzer::{TaintAnalyzer, TaintConfig};
use cytoscnpy::taint::sanitizers::SanitizerConfig;
use cytoscnpy::taint::types::{TaintFinding, VulnType};
use std::path::PathBuf;

fn analyze(source: &str, sanitizers: SanitizerConfig) -> Vec<TaintFinding> {
    let config = TaintConfig::with_custom(Vec::new(), Vec::new(), sanitizers);
    TaintAnalyzer::new(config).analyze_file(source, &PathBuf::from("app.py"))
}

fn ssrf_sanitizers(return_value: &[&str], guard: &[&str], side_effect: &[&str]) -> SanitizerConfig {
    let mut config = SanitizerConfig::default();
    config.add_group(
        &VulnType::Ssrf,
        &return_value
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        &guard.iter().map(ToString::to_string).collect::<Vec<_>>(),
        &side_effect
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
    );
    config
}

fn count_rule(findings: &[TaintFinding], rule_id: &str) -> usize {
    findings
        .iter()
        .filter(|finding| finding.rule_id == rule_id)
        .count()
}

#[test]
fn return_value_sanitizer_is_rule_scoped() {
    let findings = analyze(
        r"
import os
import requests

def process(url):
    checked = validate_allowed_url(url)
    requests.get(checked)
    os.system(checked)
",
        ssrf_sanitizers(&["validate_allowed_url"], &[], &[]),
    );

    assert_eq!(count_rule(&findings, ids::RULE_ID_SSRF), 0);
    assert_eq!(count_rule(&findings, ids::RULE_ID_SUBPROCESS), 1);
}

#[test]
fn guard_sanitizer_applies_only_inside_truthy_branch() {
    let findings = analyze(
        r"
import requests

def fetch(url):
    if is_allowed_url(url):
        requests.get(url)
    requests.get(url)
",
        ssrf_sanitizers(&[], &["is_allowed_url"], &[]),
    );

    assert_eq!(count_rule(&findings, ids::RULE_ID_SSRF), 1);
}

#[test]
fn side_effect_sanitizer_applies_after_successful_call() {
    let findings = analyze(
        r"
import requests

def fetch(url):
    validate_url_or_raise(value=url)
    requests.get(url)
",
        ssrf_sanitizers(&[], &[], &["validate_url_or_raise"]),
    );

    assert_eq!(count_rule(&findings, ids::RULE_ID_SSRF), 0);
}

#[test]
fn unknown_validator_and_safe_names_do_not_suppress() {
    let findings = analyze(
        r"
import requests

def fetch(url):
    validated_url = validate_allowed_url(url)
    requests.get(validated_url)
",
        SanitizerConfig::default(),
    );

    assert_eq!(count_rule(&findings, ids::RULE_ID_SSRF), 1);
}
