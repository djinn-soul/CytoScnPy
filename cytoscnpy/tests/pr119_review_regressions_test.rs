//! Regression coverage for actionable bot feedback on merged PR #119.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::AnalysisResult;
use cytoscnpy::complexity::{analyze_complexity, calculate_module_complexity};
use cytoscnpy::framework::FrameworkAwareVisitor;
use cytoscnpy::halstead::analyze_halstead;
use cytoscnpy::metrics::mi_compute;
use cytoscnpy::raw_metrics::analyze_raw;
use cytoscnpy::report::gitlab;
use cytoscnpy::rules::secrets::SecretFinding;
use cytoscnpy::utils::LineIndex;
use std::path::{Path, PathBuf};

fn visit_framework<'a>(source: &str, line_index: &'a LineIndex) -> FrameworkAwareVisitor<'a> {
    let parsed = ruff_python_parser::parse_module(source).unwrap();
    let module = parsed.into_syntax();
    let mut visitor = FrameworkAwareVisitor::new(line_index);
    for statement in &module.body {
        visitor.visit_stmt(statement);
    }
    visitor
}

#[test]
fn complexity_reports_do_not_fold_nested_definition_bodies() {
    let source = "class Handler:\n    def run(self, value):\n        if value:\n            return 1\n        return 0\n";
    let findings = analyze_complexity(source, Path::new("sample.py"), false);

    assert!(!findings.iter().any(|finding| finding.name == "<module>"));
    assert_eq!(
        findings
            .iter()
            .find(|finding| finding.name == "Handler")
            .unwrap()
            .complexity,
        1
    );
    assert_eq!(
        findings
            .iter()
            .find(|finding| finding.name == "run")
            .unwrap()
            .complexity,
        2
    );
    assert_eq!(calculate_module_complexity(source), Some(2));
}

#[test]
fn raw_metrics_count_assigned_multiline_literals_and_fallback_lloc() {
    let multiline = "value = \"\"\"first\nsecond\nthird\"\"\"\n";
    let metrics = analyze_raw(multiline);
    assert_eq!(metrics.multi, 3);

    let invalid = "def broken(:\n    return 1\n";
    let invalid_metrics = analyze_raw(invalid);
    assert!(invalid_metrics.sloc > 0);
    assert_eq!(invalid_metrics.lloc, invalid_metrics.sloc);
}

#[test]
fn flask_add_url_rule_resolves_keyword_and_third_positional_views() {
    let source = r#"
from flask import Flask
app = Flask(__name__)

def keyword_view():
    return "keyword"

def positional_view():
    return "positional"

app.add_url_rule("/keyword", view_func=keyword_view)
app.add_url_rule("/positional", "positional", positional_view)
"#;
    let line_index = LineIndex::new(source);
    let visitor = visit_framework(source, &line_index);

    assert!(visitor
        .framework_references
        .contains(&"keyword_view".to_owned()));
    assert!(visitor
        .framework_references
        .contains(&"positional_view".to_owned()));
}

#[test]
fn common_drf_base_classes_mark_dispatched_methods() {
    let source = r"
from rest_framework import generics, viewsets

class Items(viewsets.ModelViewSet):
    def list(self, request):
        return []

class ItemDetail(generics.GenericAPIView):
    def get(self, request):
        return None
";
    let line_index = LineIndex::new(source);
    let visitor = visit_framework(source, &line_index);

    assert!(visitor.is_framework_file);
    assert!(visitor.framework_decorated_lines.contains(&5));
    assert!(visitor.framework_decorated_lines.contains(&9));
}

#[test]
fn relative_flask_blueprint_route_is_framework_provenance() {
    let source = r#"
from . import bp

@bp.route("/items")
def list_items():
    return []
"#;
    let line_index = LineIndex::new(source);
    let visitor = visit_framework(source, &line_index);

    assert!(visitor.is_framework_file);
    assert!(visitor.detected_frameworks.contains("flask"));
    assert!(visitor.framework_decorated_lines.contains(&5));

    let unrelated = "@bp.route('/items')\ndef list_items():\n    return []\n";
    let unrelated_index = LineIndex::new(unrelated);
    let unrelated_visitor = visit_framework(unrelated, &unrelated_index);
    assert!(!unrelated_visitor.is_framework_file);
}

#[test]
fn mi_comment_weight_uses_radians_of_comment_percentage() {
    let score = mi_compute(10_000.0, 20, 500, 50);
    assert!((score - 47.977_673_494_241_1).abs() < 1e-12);
}

#[test]
fn halstead_fields_preserve_standard_distinct_and_total_semantics() {
    let parsed = ruff_python_parser::parse_module("result = value + value + value").unwrap();
    let module = ruff_python_ast::Mod::Module(parsed.into_syntax());
    let metrics = analyze_halstead(&module);

    assert!(metrics.n1 > metrics.h1, "N1 must be total operators");
    assert!(metrics.n2 > metrics.h2, "N2 must be total operands");
    assert!((metrics.vocabulary - (metrics.h1 + metrics.h2) as f64).abs() < f64::EPSILON);
    assert!((metrics.length - (metrics.n1 + metrics.n2) as f64).abs() < f64::EPSILON);
}

#[test]
fn gitlab_secret_fingerprints_disambiguate_same_line_findings() {
    let mut result = AnalysisResult::default();
    for (message, matched, confidence) in [
        ("Hardcoded password", "abcd...wxyz", 100),
        ("Hardcoded password variant", "1234...7890", 95),
    ] {
        result.secrets.push(SecretFinding {
            message: message.to_owned(),
            rule_id: "CSP-S001".to_owned(),
            category: "Secrets".to_owned(),
            file: PathBuf::from("config.py"),
            line: 20,
            severity: "CRITICAL".to_owned(),
            matched_value: Some(matched.to_owned()),
            entropy: None,
            confidence,
        });
    }

    let mut buffer = Vec::new();
    gitlab::print_gitlab(&mut buffer, &result).unwrap();
    let issues: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
    let fingerprints: Vec<&str> = issues
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|issue| issue["fingerprint"].as_str())
        .collect();

    assert_eq!(fingerprints.len(), 2);
    assert_ne!(fingerprints[0], fingerprints[1]);
}
