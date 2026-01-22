use crate::analyzer::AnalysisResult;
use serde_json::json;
use std::io::Write;

/// Generates GitLab Code Quality JSON report.
///
/// See: https://docs.gitlab.com/ee/ci/testing/code_quality.html#implementing-a-custom-tool
pub fn print_gitlab(writer: &mut impl Write, result: &AnalysisResult) -> std::io::Result<()> {
    print_gitlab_with_root(writer, result, None)
}

/// Generates GitLab Code Quality JSON report with an optional root for path normalization.
pub fn print_gitlab_with_root(
    writer: &mut impl Write,
    result: &AnalysisResult,
    root: Option<&std::path::Path>,
) -> std::io::Result<()> {
    let mut issues = Vec::new();

    // Helper to add issue
    let mut add_issue =
        |description: String, fingerprint: String, file: &str, line: usize, severity: &str| {
            issues.push(json!({
                "description": description,
                "fingerprint": fingerprint,
                "location": {
                    "path": file,
                    "lines": {
                        "begin": line
                    }
                },
                "severity": severity,
                "check_name": fingerprint.split('-').nth(1).unwrap_or("unknown")
            }));
        };

    // Helper to normalize paths for fingerprints and locations (ensures stability across CI workspaces)
    let normalize = |path: &std::path::Path| -> String {
        let normalized = if let Some(r) = root {
            path.strip_prefix(r).unwrap_or(path)
        } else {
            path
        };
        normalized.to_string_lossy().replace('\\', "/")
    };

    // Security Findings
    for (i, finding) in result.danger.iter().enumerate() {
        let normalized_path = normalize(&finding.file);
        let fingerprint = format!("danger-{}-{}-{}", finding.rule_id, normalized_path, i);
        let severity = match finding.severity.as_str() {
            "CRITICAL" | "HIGH" => "critical",
            "MEDIUM" => "major",
            _ => "minor",
        };
        add_issue(
            finding.message.clone(),
            fingerprint,
            &normalized_path,
            finding.line,
            severity,
        );
    }

    // Taint Findings
    for (i, finding) in result.taint_findings.iter().enumerate() {
        let normalized_path = normalize(&finding.file);
        let fingerprint = format!("taint-{}-{}-{}", finding.rule_id, normalized_path, i);
        let severity = match finding.severity.to_string().as_str() {
            "CRITICAL" | "HIGH" => "critical",
            "MEDIUM" => "major",
            _ => "minor",
        };
        add_issue(
            format!("{} (Source: {})", finding.vuln_type, finding.source),
            fingerprint,
            &normalized_path,
            finding.sink_line,
            severity,
        );
    }

    // Secrets
    for (i, secret) in result.secrets.iter().enumerate() {
        let normalized_path = normalize(&secret.file);
        let fingerprint = format!("secret-{}-{}-{}", secret.rule_id, normalized_path, i);
        add_issue(
            secret.message.clone(),
            fingerprint,
            &normalized_path,
            secret.line,
            "critical",
        );
    }

    // Unused Code
    for func in &result.unused_functions {
        let normalized_path = normalize(&func.file);
        add_issue(
            format!("Unused function: {}", func.name),
            format!(
                "unused-func-{}-{}-{}",
                func.name, normalized_path, func.line
            ),
            &normalized_path,
            func.line,
            "minor",
        );
    }

    for cls in &result.unused_classes {
        let normalized_path = normalize(&cls.file);
        add_issue(
            format!("Unused class: {}", cls.name),
            format!("unused-class-{}-{}-{}", cls.name, normalized_path, cls.line),
            &normalized_path,
            cls.line,
            "minor",
        );
    }
    for imp in &result.unused_imports {
        let normalized_path = normalize(&imp.file);
        add_issue(
            format!("Unused import: {}", imp.name),
            format!(
                "unused-import-{}-{}-{}",
                imp.name, normalized_path, imp.line
            ),
            &normalized_path,
            imp.line,
            "info",
        );
    }
    for var in &result.unused_variables {
        let normalized_path = normalize(&var.file);
        add_issue(
            format!("Unused variable: {}", var.name),
            format!("unused-var-{}-{}-{}", var.name, normalized_path, var.line),
            &normalized_path,
            var.line,
            "info",
        );
    }
    for method in &result.unused_methods {
        let normalized_path = normalize(&method.file);
        add_issue(
            format!("Unused method: {}", method.name),
            format!(
                "unused-method-{}-{}-{}",
                method.name, normalized_path, method.line
            ),
            &normalized_path,
            method.line,
            "minor",
        );
    }
    for param in &result.unused_parameters {
        let normalized_path = normalize(&param.file);
        add_issue(
            format!("Unused parameter: {}", param.name),
            format!(
                "unused-param-{}-{}-{}",
                param.name, normalized_path, param.line
            ),
            &normalized_path,
            param.line,
            "info",
        );
    }

    // Parse Errors
    for (i, error) in result.parse_errors.iter().enumerate() {
        let normalized_path = normalize(&error.file);
        add_issue(
            format!("Parse Error: {}", error.error),
            format!("parse-error-{i}"),
            &normalized_path,
            1, // Parse errors usually apply to the whole file if line 0, but GitLab schema requires >= 1
            "critical",
        );
    }

    serde_json::to_writer_pretty(writer, &issues)?;
    Ok(())
}
