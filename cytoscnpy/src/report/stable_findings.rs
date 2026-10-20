//! Sorted, stable identifiers for every finding in the machine JSON payload.

use rayon::prelude::*;
use serde::Serialize;
use serde_json::Value;

/// A stable finding. Dead-code findings dominate large scans, so they skip
/// building a `Value` map.
#[derive(Serialize)]
#[serde(untagged)]
pub(super) enum StableItem<'a> {
    DeadCode(StableDeadCodeItem<'a>),
    Other(Value),
}

impl StableItem<'_> {
    fn stable_id(&self) -> Option<&str> {
        match self {
            Self::DeadCode(item) => Some(&item.stable_id),
            Self::Other(value) => value.get("stable_id").and_then(Value::as_str),
        }
    }
}

/// Fields are declared in key order so the derived output matches a sorted
/// `Value` object.
#[derive(Serialize)]
pub(super) struct StableDeadCodeItem<'a> {
    confidence: u8,
    end_byte: usize,
    file: String,
    kind: &'static str,
    line: usize,
    name: &'a str,
    stable_id: String,
    start_byte: usize,
}

pub(super) fn stable_findings(result: &crate::analyzer::AnalysisResult) -> Vec<StableItem<'_>> {
    let mut items = Vec::new();

    let dead_code = [
        ("unused_function", &result.unused_functions),
        ("unused_method", &result.unused_methods),
        ("unused_class", &result.unused_classes),
        ("unused_import", &result.unused_imports),
        ("unused_variable", &result.unused_variables),
        ("unused_parameter", &result.unused_parameters),
    ];
    for (kind, defs) in dead_code {
        items.par_extend(
            defs.par_iter()
                .map(|def| StableItem::DeadCode(stable_dead_code_item(kind, def))),
        );
    }
    let mut other = Vec::new();
    other.par_extend(result.danger.par_iter().map(|finding| {
        let file = crate::utils::normalize_display_path(&finding.file);
        let stable_id = format!(
            "danger:{}:{}:{}:{}",
            finding.rule_id, file, finding.line, finding.col
        );
        serde_json::json!({
            "stable_id": stable_id, "kind": "danger", "rule_id": finding.rule_id,
            "file": file, "line": finding.line, "col": finding.col,
            "severity": finding.severity,
        })
    }));
    other.par_extend(result.secrets.par_iter().map(stable_secret_item));
    other.par_extend(result.quality.par_iter().map(|finding| {
        let file = crate::utils::normalize_display_path(&finding.file);
        let stable_id = format!(
            "quality:{}:{}:{}:{}",
            finding.rule_id, file, finding.line, finding.col
        );
        serde_json::json!({
            "stable_id": stable_id, "kind": "quality", "rule_id": finding.rule_id,
            "file": file, "line": finding.line, "col": finding.col,
            "severity": finding.severity,
        })
    }));
    other.par_extend(result.taint_findings.par_iter().map(|finding| {
        let file = crate::utils::normalize_display_path(&finding.file);
        let stable_id = format!(
            "taint:{}:{}:{}:{}",
            finding.rule_id, file, finding.source_line, finding.sink_line
        );
        serde_json::json!({
            "stable_id": stable_id, "kind": "taint", "rule_id": finding.rule_id,
            "file": file, "source_line": finding.source_line,
            "sink_line": finding.sink_line, "severity": finding.severity.to_string(),
            "exploitability_score": finding.exploitability_score,
        })
    }));
    other.par_extend(result.parse_errors.par_iter().map(|error| {
        let file = crate::utils::normalize_display_path(&error.file);
        let stable_id = format!("parse_error:{file}:{}", error.error);
        serde_json::json!({
            "stable_id": stable_id, "kind": "parse_error", "file": file,
            "error": error.error,
        })
    }));
    other.par_extend(
        crate::report::category_findings::collect_extended_findings(result)
            .par_iter()
            .map(stable_extended_item),
    );
    items.par_extend(other.into_par_iter().map(StableItem::Other));

    // Stable sort: findings sharing an ID keep their collection order.
    items.par_sort_by(|a, b| a.stable_id().cmp(&b.stable_id()));
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

fn stable_dead_code_item<'a>(
    kind: &'static str,
    def: &'a crate::visitor::Definition,
) -> StableDeadCodeItem<'a> {
    let file = crate::utils::normalize_display_path(&def.file);
    let stable_id = format!("{kind}:{}:{}:{}", def.name, file, def.start_byte);
    StableDeadCodeItem {
        confidence: def.confidence,
        end_byte: def.end_byte,
        file,
        kind,
        line: def.line,
        name: &def.name,
        stable_id,
        start_byte: def.start_byte,
    }
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
    use super::{stable_dead_code_item, stable_findings, StableItem};
    use serde_json::Value;

    #[test]
    fn dead_code_items_serialize_like_sorted_json_objects() {
        let def = crate::visitor::Definition {
            name: "pkg.mod.unused".to_owned(),
            file: std::sync::Arc::new("pkg/mod.py".into()),
            line: 3,
            start_byte: 10,
            end_byte: 42,
            confidence: 90,
            ..Default::default()
        };
        let item = StableItem::DeadCode(stable_dead_code_item("unused_function", &def));

        let expected = serde_json::json!({
            "stable_id": "unused_function:pkg.mod.unused:pkg/mod.py:10",
            "kind": "unused_function", "name": "pkg.mod.unused", "file": "pkg/mod.py",
            "line": 3, "start_byte": 10, "end_byte": 42, "confidence": 90,
        });
        assert_eq!(serde_json::to_value(&item).unwrap(), expected);
        // Field declaration order must match the sorted keys of a `Value`.
        assert_eq!(
            serde_json::to_string_pretty(&item).unwrap(),
            serde_json::to_string_pretty(&expected).unwrap()
        );
    }

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
            .map(|item| serde_json::to_value(item).unwrap())
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
