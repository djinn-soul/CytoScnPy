use anyhow::Result;
use serde_json::{Map, Value};

/// Serialize the complete analysis result with schema metadata and stable findings.
///
/// # Errors
///
/// Returns an error if the analysis result cannot be serialized as JSON.
pub fn machine_json_payload(result: &crate::analyzer::AnalysisResult) -> Result<String> {
    let mut root = match serde_json::to_value(result)? {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    root.insert("schema_version".to_owned(), Value::String("2".to_owned()));
    root.insert(
        "stable_findings".to_owned(),
        Value::Array(stable_findings(result)),
    );
    Ok(serde_json::to_string_pretty(&Value::Object(root))?)
}

fn stable_findings(result: &crate::analyzer::AnalysisResult) -> Vec<Value> {
    let mut items = Vec::new();

    for def in &result.unused_functions {
        items.push(stable_dead_code_item("unused_function", def));
    }
    for def in &result.unused_methods {
        items.push(stable_dead_code_item("unused_method", def));
    }
    for def in &result.unused_classes {
        items.push(stable_dead_code_item("unused_class", def));
    }
    for def in &result.unused_imports {
        items.push(stable_dead_code_item("unused_import", def));
    }
    for def in &result.unused_variables {
        items.push(stable_dead_code_item("unused_variable", def));
    }
    for def in &result.unused_parameters {
        items.push(stable_dead_code_item("unused_parameter", def));
    }
    for finding in &result.danger {
        let file = crate::utils::normalize_display_path(&finding.file);
        let stable_id = format!(
            "danger:{}:{}:{}:{}",
            finding.rule_id, file, finding.line, finding.col
        );
        items.push(serde_json::json!({
            "stable_id": stable_id, "kind": "danger", "rule_id": finding.rule_id,
            "file": file, "line": finding.line, "col": finding.col,
            "severity": finding.severity,
        }));
    }
    for finding in &result.secrets {
        items.push(stable_secret_item(finding));
    }
    for finding in &result.quality {
        let file = crate::utils::normalize_display_path(&finding.file);
        let stable_id = format!(
            "quality:{}:{}:{}:{}",
            finding.rule_id, file, finding.line, finding.col
        );
        items.push(serde_json::json!({
            "stable_id": stable_id, "kind": "quality", "rule_id": finding.rule_id,
            "file": file, "line": finding.line, "col": finding.col,
            "severity": finding.severity,
        }));
    }
    for finding in &result.taint_findings {
        let file = crate::utils::normalize_display_path(&finding.file);
        let stable_id = format!(
            "taint:{}:{}:{}:{}",
            finding.rule_id, file, finding.source_line, finding.sink_line
        );
        items.push(serde_json::json!({
            "stable_id": stable_id, "kind": "taint", "rule_id": finding.rule_id,
            "file": file, "source_line": finding.source_line,
            "sink_line": finding.sink_line, "severity": finding.severity.to_string(),
            "exploitability_score": finding.exploitability_score,
        }));
    }
    for error in &result.parse_errors {
        let file = crate::utils::normalize_display_path(&error.file);
        let stable_id = format!("parse_error:{file}:{}", error.error);
        items.push(serde_json::json!({
            "stable_id": stable_id, "kind": "parse_error", "file": file,
            "error": error.error,
        }));
    }
    items.extend(
        crate::report::category_findings::collect_extended_findings(result)
            .iter()
            .map(stable_extended_item),
    );

    items.sort_by(|a, b| {
        a.get("stable_id")
            .and_then(Value::as_str)
            .cmp(&b.get("stable_id").and_then(Value::as_str))
    });
    items
}

fn stable_extended_item(finding: &crate::report::category_findings::ExtendedFinding) -> Value {
    serde_json::json!({
        "stable_id": finding.stable_id(),
        "kind": finding.kind,
        "rule_id": finding.rule_id,
        "file": finding.normalized_path(None),
        "line": finding.line,
        "col": finding.column,
        "end_line": finding.end_line,
        "severity": finding.severity,
        "message": finding.message,
    })
}

fn stable_dead_code_item(kind: &str, def: &crate::visitor::Definition) -> Value {
    let file = crate::utils::normalize_display_path(&def.file);
    let stable_id = format!("{kind}:{}:{}:{}", def.name, file, def.start_byte);
    serde_json::json!({
        "stable_id": stable_id, "kind": kind, "name": def.name, "file": file,
        "line": def.line, "start_byte": def.start_byte, "end_byte": def.end_byte,
        "confidence": def.confidence,
    })
}

fn stable_secret_item(finding: &crate::rules::secrets::SecretFinding) -> Value {
    let file = crate::utils::normalize_display_path(&finding.file);
    let matched = finding.matched_value.as_deref().unwrap_or("-");
    let entropy = finding
        .entropy
        .map(|value| format!("{value:.4}"))
        .unwrap_or_else(|| "-".to_owned());
    let stable_id = format!(
        "secret:{}:{}:{}:{}:{}:{}:{}",
        finding.rule_id, file, finding.line, finding.confidence, finding.message, matched, entropy
    );
    serde_json::json!({
        "stable_id": stable_id, "kind": "secret", "rule_id": finding.rule_id,
        "file": file, "line": finding.line, "severity": finding.severity,
        "confidence": finding.confidence,
    })
}

#[cfg(test)]
mod tests {
    use super::stable_findings;
    use serde_json::Value;

    #[test]
    fn secret_stable_ids_include_disambiguators() {
        let mut result = crate::analyzer::AnalysisResult::default();
        let file = std::path::PathBuf::from("test.py");
        result
            .secrets
            .push(secret_finding(&file, "Hardcoded password", 100));
        result
            .secrets
            .push(secret_finding(&file, "Hardcoded password variant", 95));

        let stable_ids: Vec<String> = stable_findings(&result)
            .into_iter()
            .filter(|item| item.get("kind").and_then(Value::as_str) == Some("secret"))
            .filter_map(|item| item["stable_id"].as_str().map(str::to_owned))
            .collect();

        assert_eq!(stable_ids.len(), 2);
        assert_ne!(stable_ids[0], stable_ids[1]);
    }

    fn secret_finding(
        file: &std::path::Path,
        message: &str,
        confidence: u8,
    ) -> crate::rules::secrets::SecretFinding {
        crate::rules::secrets::SecretFinding {
            message: message.to_owned(),
            rule_id: "CSP-S001".to_owned(),
            category: "Secrets".to_owned(),
            file: file.to_path_buf(),
            line: 20,
            severity: "CRITICAL".to_owned(),
            matched_value: Some(message.to_owned()),
            entropy: None,
            confidence,
        }
    }
}
