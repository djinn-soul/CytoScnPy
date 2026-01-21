use crate::analyzer::AnalysisResult;
use serde_json::json;
use std::io::Write;

/// Generates GitLab Code Quality JSON report.
///
/// See: https://docs.gitlab.com/ee/ci/testing/code_quality.html#implementing-a-custom-tool
pub fn print_gitlab(writer: &mut impl Write, result: &AnalysisResult) -> std::io::Result<()> {
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

    // Security Findings
    for (i, finding) in result.danger.iter().enumerate() {
        let fingerprint = format!(
            "danger-{}-{}-{}",
            finding.rule_id,
            finding.file.display(),
            i
        ); // quick fingerprint
        let severity = match finding.severity.as_str() {
            "CRITICAL" | "HIGH" => "critical",
            "MEDIUM" => "major",
            _ => "minor",
        };
        add_issue(
            finding.message.clone(),
            fingerprint,
            &finding.file.to_string_lossy(),
            finding.line,
            severity,
        );
    }

    // Taint Findings
    for (i, finding) in result.taint_findings.iter().enumerate() {
        let fingerprint = format!("taint-{}-{}-{}", finding.rule_id, finding.file.display(), i);
        let severity = match finding.severity.to_string().as_str() {
            "CRITICAL" | "HIGH" => "critical",
            "MEDIUM" => "major",
            _ => "minor",
        };
        add_issue(
            format!("{} (Source: {})", finding.vuln_type, finding.source),
            fingerprint,
            &finding.file.to_string_lossy(),
            finding.sink_line,
            severity,
        );
    }

    // Secrets
    for (i, secret) in result.secrets.iter().enumerate() {
        let fingerprint = format!("secret-{}-{}-{}", secret.rule_id, secret.file.display(), i);
        add_issue(
            secret.message.clone(),
            fingerprint,
            &secret.file.to_string_lossy(),
            secret.line,
            "critical",
        );
    }

    // Unused Code
    for (i, func) in result.unused_functions.iter().enumerate() {
        add_issue(
            format!("Unused function: {}", func.name),
            format!("unused-func-{}", i),
            &func.file.to_string_lossy(),
            func.line,
            "minor",
        );
    }

    // ... (Add other unused types similarly if verbose needed, or keep minimal)
    // For brevity in initial implementation, covering minimal set.
    // Expanding to all unused types:
    for (i, cls) in result.unused_classes.iter().enumerate() {
        add_issue(
            format!("Unused class: {}", cls.name),
            format!("unused-class-{}", i),
            &cls.file.to_string_lossy(),
            cls.line,
            "minor",
        );
    }
    for (i, imp) in result.unused_imports.iter().enumerate() {
        add_issue(
            format!("Unused import: {}", imp.name),
            format!("unused-import-{}", i),
            &imp.file.to_string_lossy(),
            imp.line,
            "info",
        );
    }
    for (i, var) in result.unused_variables.iter().enumerate() {
        add_issue(
            format!("Unused variable: {}", var.name),
            format!("unused-var-{}", i),
            &var.file.to_string_lossy(),
            var.line,
            "info",
        );
    }
    for (i, method) in result.unused_methods.iter().enumerate() {
        add_issue(
            format!("Unused method: {}", method.name),
            format!("unused-method-{}", i),
            &method.file.to_string_lossy(),
            method.line,
            "minor",
        );
    }
    for (i, param) in result.unused_parameters.iter().enumerate() {
        add_issue(
            format!("Unused parameter: {}", param.name),
            format!("unused-param-{}", i),
            &param.file.to_string_lossy(),
            param.line,
            "info",
        );
    }

    // Parse Errors
    for (i, error) in result.parse_errors.iter().enumerate() {
        add_issue(
            format!("Parse Error: {}", error.error),
            format!("parse-error-{}", i),
            &error.file.to_string_lossy(),
            0, // Parse errors usually apply to the whole file if line 0
            "critical",
        );
    }

    serde_json::to_writer_pretty(writer, &issues)?;
    Ok(())
}
