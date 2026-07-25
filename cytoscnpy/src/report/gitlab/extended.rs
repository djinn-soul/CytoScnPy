use super::make_gitlab_issue;
use crate::analyzer::AnalysisResult;
use crate::report::category_findings::collect_extended_findings;
use std::path::Path;

pub(super) fn add_issues(
    issues: &mut Vec<serde_json::Value>,
    result: &AnalysisResult,
    root: Option<&Path>,
) {
    for finding in collect_extended_findings(result) {
        let (file, line) = finding.location_or_manifest(root);
        issues.push(make_gitlab_issue(
            &finding.message,
            &finding.stable_id_with_path(&file),
            &file,
            line,
            gitlab_severity(&finding.severity),
            &finding.rule_id,
        ));
    }
}

fn gitlab_severity(severity: &str) -> &str {
    match severity.to_uppercase().as_str() {
        "CRITICAL" | "HIGH" => "critical",
        "MEDIUM" | "WARNING" => "major",
        "LOW" => "minor",
        _ => "info",
    }
}
