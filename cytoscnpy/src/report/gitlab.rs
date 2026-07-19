use crate::analyzer::AnalysisResult;
use serde_json::json;
use std::io::Write;

/// Generates `GitLab` Code Quality JSON report.
///
/// See: <https://docs.gitlab.com/ee/ci/testing/code_quality.html#implementing-a-custom-tool>
///
/// # Errors
///
/// Returns an error if writing to the `writer` fails.
pub fn print_gitlab(writer: &mut impl Write, result: &AnalysisResult) -> std::io::Result<()> {
    print_gitlab_with_root(writer, result, None)
}

/// Generates `GitLab` Code Quality JSON report with an optional root for path normalization.
///
/// # Errors
///
/// Returns an error if writing to the `writer` or JSON serialization fails.
pub fn print_gitlab_with_root(
    writer: &mut impl Write,
    result: &AnalysisResult,
    root: Option<&std::path::Path>,
) -> std::io::Result<()> {
    let mut issues = Vec::new();

    add_danger_issues(&mut issues, result, root);
    add_quality_issues(&mut issues, result, root);
    add_taint_issues(&mut issues, result, root);
    add_secrets_issues(&mut issues, result, root);
    add_unused_code_issues(&mut issues, result, root);
    add_parse_error_issues(&mut issues, result, root);

    serde_json::to_writer_pretty(writer, &issues)?;
    Ok(())
}

fn add_danger_issues(
    issues: &mut Vec<serde_json::Value>,
    result: &AnalysisResult,
    root: Option<&std::path::Path>,
) {
    for finding in &result.danger {
        let normalized_path = normalize_path(&finding.file, root);
        let fingerprint = format!(
            "danger-{}-{}-{}-{}",
            finding.rule_id, normalized_path, finding.line, finding.col
        );
        let severity = match finding.severity.as_str() {
            "CRITICAL" | "HIGH" => "critical",
            "MEDIUM" => "major",
            _ => "minor",
        };
        issues.push(make_gitlab_issue(
            &finding.message,
            &fingerprint,
            &normalized_path,
            finding.line,
            severity,
            &finding.rule_id,
        ));
    }
}

fn add_quality_issues(
    issues: &mut Vec<serde_json::Value>,
    result: &AnalysisResult,
    root: Option<&std::path::Path>,
) {
    for finding in &result.quality {
        let normalized_path = normalize_path(&finding.file, root);
        let fingerprint = format!(
            "quality-{}-{}-{}-{}",
            finding.rule_id, normalized_path, finding.line, finding.col
        );
        let severity = match finding.severity.as_str() {
            "CRITICAL" | "HIGH" => "critical",
            "MEDIUM" => "major",
            _ => "minor",
        };
        issues.push(make_gitlab_issue(
            &finding.message,
            &fingerprint,
            &normalized_path,
            finding.line,
            severity,
            &finding.rule_id,
        ));
    }
}

fn add_taint_issues(
    issues: &mut Vec<serde_json::Value>,
    result: &AnalysisResult,
    root: Option<&std::path::Path>,
) {
    for finding in &result.taint_findings {
        let normalized_path = normalize_path(&finding.file, root);
        let fingerprint = format!(
            "taint-{}-{}-{}-{}-{}",
            finding.rule_id,
            normalized_path,
            finding.source_line,
            finding.sink_line,
            finding.sink_col
        );
        let severity = match finding.severity.to_string().as_str() {
            "CRITICAL" | "HIGH" => "critical",
            "MEDIUM" => "major",
            _ => "minor",
        };
        issues.push(make_gitlab_issue(
            &format!("{} (Source: {})", finding.vuln_type, finding.source),
            &fingerprint,
            &normalized_path,
            finding.sink_line,
            severity,
            &finding.rule_id,
        ));
    }
}

fn add_secrets_issues(
    issues: &mut Vec<serde_json::Value>,
    result: &AnalysisResult,
    root: Option<&std::path::Path>,
) {
    for secret in &result.secrets {
        let normalized_path = normalize_path(&secret.file, root);
        let fingerprint = format!(
            "secret-{}-{}-{}",
            secret.rule_id, normalized_path, secret.line
        );
        issues.push(make_gitlab_issue(
            &secret.message,
            &fingerprint,
            &normalized_path,
            secret.line,
            "critical",
            &secret.rule_id,
        ));
    }
}

fn add_unused_code_issues(
    issues: &mut Vec<serde_json::Value>,
    result: &AnalysisResult,
    root: Option<&std::path::Path>,
) {
    for func in &result.unused_functions {
        let normalized_path = normalize_path(&func.file, root);
        issues.push(make_gitlab_issue(
            &format!("Unused function: {}", func.name),
            &format!(
                "unused-func-{}-{}-{}",
                func.name, normalized_path, func.line
            ),
            &normalized_path,
            func.line,
            "minor",
            "UnusedFunction",
        ));
    }
    for cls in &result.unused_classes {
        let normalized_path = normalize_path(&cls.file, root);
        issues.push(make_gitlab_issue(
            &format!("Unused class: {}", cls.name),
            &format!("unused-class-{}-{}-{}", cls.name, normalized_path, cls.line),
            &normalized_path,
            cls.line,
            "minor",
            "UnusedClass",
        ));
    }
    for imp in &result.unused_imports {
        let normalized_path = normalize_path(&imp.file, root);
        issues.push(make_gitlab_issue(
            &format!("Unused import: {}", imp.name),
            &format!(
                "unused-import-{}-{}-{}",
                imp.name, normalized_path, imp.line
            ),
            &normalized_path,
            imp.line,
            "info",
            "UnusedImport",
        ));
    }
    for var in &result.unused_variables {
        let normalized_path = normalize_path(&var.file, root);
        issues.push(make_gitlab_issue(
            &format!("Unused variable: {}", var.name),
            &format!("unused-var-{}-{}-{}", var.name, normalized_path, var.line),
            &normalized_path,
            var.line,
            "info",
            "UnusedVariable",
        ));
    }
    for method in &result.unused_methods {
        let normalized_path = normalize_path(&method.file, root);
        issues.push(make_gitlab_issue(
            &format!("Unused method: {}", method.name),
            &format!(
                "unused-method-{}-{}-{}",
                method.name, normalized_path, method.line
            ),
            &normalized_path,
            method.line,
            "minor",
            "UnusedMethod",
        ));
    }
    for param in &result.unused_parameters {
        let normalized_path = normalize_path(&param.file, root);
        issues.push(make_gitlab_issue(
            &format!("Unused parameter: {}", param.name),
            &format!(
                "unused-param-{}-{}-{}",
                param.name, normalized_path, param.line
            ),
            &normalized_path,
            param.line,
            "info",
            "UnusedParameter",
        ));
    }
}

fn add_parse_error_issues(
    issues: &mut Vec<serde_json::Value>,
    result: &AnalysisResult,
    root: Option<&std::path::Path>,
) {
    for error in &result.parse_errors {
        let normalized_path = normalize_path(&error.file, root);
        issues.push(make_gitlab_issue(
            &format!("Parse Error: {}", error.error),
            &format!("parse-error-{}-{}", normalized_path, error.error),
            &normalized_path,
            1,
            "critical",
            "ParseError",
        ));
    }
}

fn normalize_path(path: &std::path::Path, root: Option<&std::path::Path>) -> String {
    let normalized = if let Some(r) = root {
        if r.as_os_str() == "." || r.as_os_str().is_empty() {
            path
        } else {
            path.strip_prefix(r).unwrap_or(path)
        }
    } else {
        path
    };
    let path = normalized.to_string_lossy().replace('\\', "/");
    path.strip_prefix("./").unwrap_or(&path).to_owned()
}

fn make_gitlab_issue(
    description: &str,
    fingerprint: &str,
    file: &str,
    line: usize,
    severity: &str,
    check_name: &str,
) -> serde_json::Value {
    json!({
        "description": format!("{} ({}:{})", description, file, line),
        "fingerprint": fingerprint,
        "location": {
            "path": file,
            "lines": {
                "begin": line
            }
        },
        "severity": severity,
        "check_name": check_name
    })
}
