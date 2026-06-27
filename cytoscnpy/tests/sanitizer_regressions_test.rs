//! Regression coverage for rule-scoped sanitizer propagation.

use cytoscnpy::rules::ids;
use cytoscnpy::taint::analyzer::{SanitizerPlugin, TaintAnalyzer, TaintConfig};
use cytoscnpy::taint::sanitizers::SanitizerConfig;
use cytoscnpy::taint::types::{TaintFinding, VulnType};
use ruff_python_ast::ExprCall;
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
fn side_effect_sanitizer_applies_in_assignment_rhs() {
    let findings = analyze(
        r"
import requests

def fetch(url):
    ok = validate_url_or_raise(url)
    requests.get(url)
",
        ssrf_sanitizers(&[], &[], &["validate_url_or_raise"]),
    );

    assert_eq!(count_rule(&findings, ids::RULE_ID_SSRF), 0);
}

#[test]
fn guard_sanitizer_applies_inside_boolean_test() {
    let findings = analyze(
        r"
import requests

def fetch(url):
    if is_allowed_url(url) and url.startswith('https'):
        requests.get(url)
    requests.get(url)
",
        ssrf_sanitizers(&[], &["is_allowed_url"], &[]),
    );

    assert_eq!(count_rule(&findings, ids::RULE_ID_SSRF), 1);
}

#[test]
fn legacy_builtin_command_sanitizers_are_restored() {
    let findings = analyze(
        r"
import os
import shlex

def run(cmd):
    parts = shlex.split(cmd)
    quoted = shlex.quote(cmd)
    os.system(quoted)
",
        SanitizerConfig::default(),
    );

    assert_eq!(count_rule(&findings, ids::RULE_ID_SUBPROCESS), 0);
}

struct LegacySanitizerPlugin;

impl SanitizerPlugin for LegacySanitizerPlugin {
    fn name(&self) -> &'static str {
        "legacy"
    }

    fn is_sanitizer(&self, _call: &ExprCall) -> bool {
        true
    }
}

#[test]
fn legacy_sanitizer_plugin_default_still_clears_taint() {
    let config = TaintConfig::with_custom(Vec::new(), Vec::new(), SanitizerConfig::default());
    let mut analyzer = TaintAnalyzer::new(config);
    analyzer.plugins.register_sanitizer(LegacySanitizerPlugin);

    let findings = analyzer.analyze_file(
        r"
import os

def run(cmd):
    checked = legacy_clean(cmd)
    os.system(checked)
",
        &PathBuf::from("app.py"),
    );

    assert_eq!(count_rule(&findings, ids::RULE_ID_SUBPROCESS), 0);
}

#[test]
fn code_injection_sanitizer_group_covers_custom_sinks() {
    let mut config = SanitizerConfig::default();
    config.add_group(
        &VulnType::CodeInjection,
        &["clean_custom".to_owned()],
        &[],
        &[],
    );
    let taint_config = TaintConfig::with_custom(
        vec!["custom_source".to_owned()],
        vec!["custom_sink".to_owned()],
        config,
    );

    let findings = TaintAnalyzer::new(taint_config).analyze_file(
        r"
def process():
    data = custom_source()
    safe = clean_custom(data)
    custom_sink(safe)
",
        &PathBuf::from("app.py"),
    );

    assert!(findings.is_empty());
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
