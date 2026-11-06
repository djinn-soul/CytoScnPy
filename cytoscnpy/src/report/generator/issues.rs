use crate::analyzer::AnalysisResult;
use crate::report::stable_findings::{
    dead_code_stable_id, parse_error_stable_id, rule_finding_stable_id, secret_stable_id,
    taint_stable_id,
};
use crate::report::templates::{IssueDetail, IssueItem};
use crate::visitor::Definition;

/// Fields shared by every issue; the rest default to empty.
struct Base<'a> {
    category: &'a str,
    group: String,
    severity: &'a str,
    rule_id: &'a str,
    stable_id: String,
    message: String,
    file: String,
    line: usize,
    cell: Option<usize>,
    suppressible: bool,
}

impl Base<'_> {
    fn into_item(self) -> IssueItem {
        let noqa = if self.suppressible && !self.rule_id.is_empty() {
            format!("# noqa: {}", self.rule_id)
        } else {
            String::new()
        };
        IssueItem {
            category: self.category.to_owned(),
            group: self.group,
            severity: normalize_severity(self.severity),
            rule_id: self.rule_id.to_owned(),
            stable_id: self.stable_id,
            message: self.message,
            file: self.file,
            line: self.line,
            cell: self.cell,
            view_line: self.line,
            link: String::new(), // Assigned by the shared report path registry.
            confidence: None,
            details: Vec::new(),
            flow: Vec::new(),
            fixable: false,
            noqa,
            code_snippet: None,
        }
    }
}

fn detail(label: &str, value: String) -> IssueDetail {
    IssueDetail {
        label: label.to_owned(),
        value,
    }
}

/// Maps the severity spellings used across rules onto the report's four levels.
pub(super) fn normalize_severity(severity: &str) -> String {
    match severity.trim().to_ascii_uppercase().as_str() {
        "CRITICAL" => "CRITICAL",
        "HIGH" | "ERROR" => "HIGH",
        "LOW" | "INFO" | "NOTE" | "HINT" => "LOW",
        _ => "MEDIUM",
    }
    .to_owned()
}

fn dead_code_issues(items: &mut Vec<IssueItem>, kind: &str, label: &str, defs: &[Definition]) {
    for def in defs {
        let rule_id = def.def_type.unused_rule_id();
        let mut item = Base {
            category: "Unused",
            group: format!("Unused {label}"),
            severity: "LOW",
            rule_id,
            stable_id: dead_code_stable_id(kind, def),
            message: format!("Unused {label}: {}", def.full_name),
            file: def.file.to_string_lossy().to_string(),
            line: def.line,
            cell: def.cell_number,
            suppressible: true,
        }
        .into_item();
        item.confidence = Some(def.confidence);
        item.fixable = def.fix.is_some();
        items.push(item);
    }
}

pub(super) fn flatten_issues(result: &AnalysisResult) -> Vec<IssueItem> {
    let mut items = Vec::new();

    dead_code_issues(
        &mut items,
        "unused_function",
        "function",
        &result.unused_functions,
    );
    dead_code_issues(
        &mut items,
        "unused_method",
        "method",
        &result.unused_methods,
    );
    dead_code_issues(&mut items, "unused_class", "class", &result.unused_classes);
    dead_code_issues(
        &mut items,
        "unused_import",
        "import",
        &result.unused_imports,
    );
    dead_code_issues(
        &mut items,
        "unused_variable",
        "variable",
        &result.unused_variables,
    );
    dead_code_issues(
        &mut items,
        "unused_parameter",
        "parameter",
        &result.unused_parameters,
    );

    for secret in &result.secrets {
        let mut item = Base {
            category: "Security",
            group: "Secrets".to_owned(),
            severity: &secret.severity,
            rule_id: &secret.rule_id,
            stable_id: secret_stable_id(secret),
            message: format!("Secret found: {}", secret.message),
            file: secret.file.to_string_lossy().to_string(),
            line: secret.line,
            cell: secret.cell_number,
            suppressible: true,
        }
        .into_item();
        item.confidence = Some(secret.confidence);
        if let Some(entropy) = secret.entropy {
            item.details
                .push(detail("Entropy", format!("{entropy:.2}")));
        }
        // Recognizers store the value already redacted.
        if let Some(matched) = &secret.matched_value {
            item.details
                .push(detail("Value (redacted)", matched.clone()));
        }
        items.push(item);
    }
    for finding in &result.danger {
        items.push(
            Base {
                category: "Security",
                group: finding.category.clone(),
                severity: &finding.severity,
                rule_id: &finding.rule_id,
                stable_id: rule_finding_stable_id("danger", finding),
                message: finding.message.clone(),
                file: finding.file.to_string_lossy().to_string(),
                line: finding.line,
                cell: finding.cell_number,
                suppressible: true,
            }
            .into_item(),
        );
    }
    for finding in &result.taint_findings {
        let severity = finding.severity.to_string();
        let mut item = Base {
            category: "Security",
            group: format!("Taint: {}", finding.vuln_type),
            severity: &severity,
            rule_id: &finding.rule_id,
            stable_id: taint_stable_id(finding),
            message: format!(
                "Taint flow detected: {} -> {}",
                finding.source, finding.sink
            ),
            file: finding.file.to_string_lossy().to_string(),
            // Suppressions and the fix belong at the sink.
            line: finding.sink_line,
            cell: finding.cell_number,
            suppressible: true,
        }
        .into_item();
        let source_at = match finding.source_cell_number {
            Some(cell) => format!("cell {cell}, line {}", finding.source_line),
            None => format!("line {}", finding.source_line),
        };
        item.details.push(detail(
            "Source",
            format!("{} ({source_at})", finding.source),
        ));
        item.details.push(detail(
            "Exploitability",
            format!("{}/100", finding.exploitability_score),
        ));
        if !finding.remediation.is_empty() {
            item.details
                .push(detail("Remediation", finding.remediation.clone()));
        }
        item.flow.clone_from(&finding.flow_path);
        items.push(item);
    }

    for finding in &result.quality {
        items.push(
            Base {
                category: "Quality",
                group: finding.category.clone(),
                severity: &finding.severity,
                rule_id: &finding.rule_id,
                stable_id: rule_finding_stable_id("quality", finding),
                message: finding.message.clone(),
                file: finding.file.to_string_lossy().to_string(),
                line: finding.line,
                cell: finding.cell_number,
                suppressible: true,
            }
            .into_item(),
        );
    }
    for error in &result.parse_errors {
        items.push(
            Base {
                category: "Quality",
                group: "Parse errors".to_owned(),
                severity: "HIGH",
                rule_id: "",
                stable_id: parse_error_stable_id(error),
                message: format!("Parse Error: {}", error.error),
                file: error.file.to_string_lossy().to_string(),
                line: 0,
                cell: None,
                suppressible: false,
            }
            .into_item(),
        );
    }
    for finding in crate::report::category_findings::collect_dependency_findings(result) {
        let (file, line) = finding.location_or_manifest(None);
        // Dependency findings are configured in the project, not suppressed inline.
        items.push(
            Base {
                category: finding.category,
                group: "Dependencies".to_owned(),
                severity: &finding.severity,
                rule_id: &finding.rule_id,
                stable_id: finding.stable_id(),
                message: finding.message.clone(),
                file,
                line,
                cell: None,
                suppressible: false,
            }
            .into_item(),
        );
    }

    items
}

pub(super) fn segregate_issues(
    items: &[IssueItem],
) -> (Vec<IssueItem>, Vec<IssueItem>, Vec<IssueItem>) {
    let mut unused = Vec::new();
    let mut security = Vec::new();
    let mut quality = Vec::new();

    for item in items {
        match item.category.as_str() {
            "Unused" => unused.push(item.clone()),
            "Security" => security.push(item.clone()),
            _ => quality.push(item.clone()),
        }
    }

    (unused, security, quality)
}

#[cfg(test)]
mod tests {
    use super::normalize_severity;

    #[test]
    fn severities_map_onto_report_levels() {
        assert_eq!(normalize_severity("critical"), "CRITICAL");
        assert_eq!(normalize_severity("error"), "HIGH");
        assert_eq!(normalize_severity("Warning"), "MEDIUM");
        assert_eq!(normalize_severity("info"), "LOW");
        assert_eq!(normalize_severity("unexpected"), "MEDIUM");
    }
}
