use rustc_hash::{FxHashMap, FxHashSet};

use super::AnalysisResult;

pub(crate) fn apply_global_ignores(
    result: &mut AnalysisResult,
    configured_ignores: Option<&[String]>,
) {
    let Some(configured_ignores) = configured_ignores else {
        return;
    };
    let ignored = configured_ignores
        .iter()
        .map(|rule| rule.trim().to_uppercase())
        .filter(|rule| !rule.is_empty())
        .collect::<FxHashSet<_>>();
    if ignored.is_empty() {
        return;
    }

    let is_ignored = |rule_id: &str| ignored.contains(&rule_id.to_uppercase());
    let mut removed_by_file = FxHashMap::default();
    result.secrets.retain(|finding| {
        let remove = is_ignored(&finding.rule_id);
        if remove {
            *removed_by_file.entry(finding.file.clone()).or_insert(0) += 1;
        }
        !remove
    });
    result.danger.retain(|finding| {
        let remove = is_ignored(&finding.rule_id);
        if remove {
            *removed_by_file.entry(finding.file.clone()).or_insert(0) += 1;
        }
        !remove
    });
    result.quality.retain(|finding| {
        let remove = is_ignored(&finding.rule_id);
        if remove {
            *removed_by_file.entry(finding.file.clone()).or_insert(0) += 1;
        }
        !remove
    });
    result
        .taint_findings
        .retain(|finding| !is_ignored(&finding.rule_id));
    result
        .clones
        .retain(|finding| !is_ignored(&finding.rule_id));

    if is_ignored(crate::rules::ids::RULE_ID_MISSING_DEPENDENCY) {
        result.missing_dependencies.clear();
        result.missing_dependency_details.clear();
    }
    if is_ignored(crate::rules::ids::RULE_ID_UNUSED_DEPENDENCY) {
        result.unused_dependencies.clear();
    }
    if is_ignored(crate::rules::ids::RULE_ID_TRANSITIVE_DEPENDENCY) {
        result.transitive_dependencies.clear();
    }
    if is_ignored(crate::rules::ids::RULE_ID_DEV_DEPENDENCY_IN_PROD) {
        result.dev_dependencies_in_production.clear();
    }
    if is_ignored(crate::rules::ids::RULE_ID_STDLIB_DEPENDENCY) {
        result.stdlib_dependencies.clear();
    }
    for metric in &mut result.file_metrics {
        if let Some(removed) = removed_by_file.get(&metric.file) {
            metric.total_issues = metric.total_issues.saturating_sub(*removed);
        }
    }

    result.analysis_summary.secrets_count = result.secrets.len();
    result.analysis_summary.danger_count = result.danger.len();
    result.analysis_summary.quality_count = result.quality.len();
    result.analysis_summary.taint_count = result.taint_findings.len();
}

#[cfg(test)]
mod tests {
    use super::apply_global_ignores;
    use crate::rules::Finding;

    fn finding(rule_id: &str) -> Finding {
        Finding {
            rule_id: rule_id.to_owned(),
            category: "Quality".to_owned(),
            severity: "warning".to_owned(),
            message: "finding".to_owned(),
            file: "sample.py".into(),
            line: 1,
            col: 1,
        }
    }

    #[test]
    fn filters_configured_rules_case_insensitively_and_updates_counts() {
        let mut result = crate::analyzer::AnalysisResult {
            quality: vec![finding("CSP-L001"), finding("CSP-L002")],
            analysis_summary: crate::analyzer::AnalysisSummary {
                quality_count: 2,
                ..Default::default()
            },
            file_metrics: vec![crate::analyzer::types::FileMetrics {
                file: "sample.py".into(),
                loc: 1,
                sloc: 1,
                complexity: 1.0,
                mi: 100.0,
                total_issues: 2,
            }],
            ..Default::default()
        };

        apply_global_ignores(&mut result, Some(&["csp-l001".to_owned()]));

        assert_eq!(result.quality.len(), 1);
        assert_eq!(result.quality[0].rule_id, "CSP-L002");
        assert_eq!(result.analysis_summary.quality_count, 1);
        assert_eq!(result.file_metrics[0].total_issues, 1);
    }
}
