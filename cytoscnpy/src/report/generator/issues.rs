use crate::analyzer::AnalysisResult;
use crate::report::templates::IssueItem;

pub(super) fn flatten_issues(result: &AnalysisResult) -> Vec<IssueItem> {
    let mut items = Vec::new();

    let mut add = |category: &str, severity: &str, msg: String, file: String, line: usize| {
        let safe_name = file.replace(['/', '\\', ':'], "_") + ".html";
        let link = format!("files/{safe_name}#L{line}");

        items.push(IssueItem {
            category: category.to_owned(),
            severity: severity.to_owned(),
            message: msg,
            file,
            line,
            link,
            code_snippet: None,
        });
    };

    for item in &result.unused_functions {
        add(
            "Unused",
            "LOW",
            format!("Unused function: {}", item.full_name),
            item.file.to_string_lossy().to_string(),
            item.line,
        );
    }
    for item in &result.unused_classes {
        add(
            "Unused",
            "LOW",
            format!("Unused class: {}", item.full_name),
            item.file.to_string_lossy().to_string(),
            item.line,
        );
    }
    for item in &result.unused_imports {
        add(
            "Unused",
            "LOW",
            format!("Unused import: {}", item.full_name),
            item.file.to_string_lossy().to_string(),
            item.line,
        );
    }

    for secret in &result.secrets {
        add(
            "Security",
            "HIGH",
            format!("Secret found: {}", secret.message),
            secret.file.to_string_lossy().to_string(),
            secret.line,
        );
    }
    for finding in &result.danger {
        add(
            "Security",
            "HIGH",
            finding.message.clone(),
            finding.file.to_string_lossy().to_string(),
            finding.line,
        );
    }
    for finding in &result.taint_findings {
        add(
            "Security",
            "CRITICAL",
            format!(
                "Taint flow detected: {} -> {}",
                finding.source, finding.sink
            ),
            finding.file.to_string_lossy().to_string(),
            finding.source_line,
        );
    }

    for finding in &result.quality {
        add(
            "Quality",
            &finding.severity,
            finding.message.clone(),
            finding.file.to_string_lossy().to_string(),
            finding.line,
        );
    }
    for error in &result.parse_errors {
        add(
            "Quality",
            "HIGH",
            format!("Parse Error: {}", error.error),
            error.file.to_string_lossy().to_string(),
            0,
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
