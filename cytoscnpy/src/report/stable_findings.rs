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
        let stable_id = rule_finding_stable_id("danger", finding);
        with_cell(
            serde_json::json!({
                "stable_id": stable_id, "kind": "danger", "rule_id": finding.rule_id,
                "file": file, "line": finding.line, "col": finding.col,
                "severity": finding.severity,
            }),
            "cell_number",
            finding.cell_number,
        )
    }));
    other.par_extend(result.secrets.par_iter().map(stable_secret_item));
    other.par_extend(result.quality.par_iter().map(|finding| {
        let file = crate::utils::normalize_display_path(&finding.file);
        let stable_id = rule_finding_stable_id("quality", finding);
        with_cell(
            serde_json::json!({
                "stable_id": stable_id, "kind": "quality", "rule_id": finding.rule_id,
                "file": file, "line": finding.line, "col": finding.col,
                "severity": finding.severity,
            }),
            "cell_number",
            finding.cell_number,
        )
    }));
    other.par_extend(result.taint_findings.par_iter().map(|finding| {
        let file = crate::utils::normalize_display_path(&finding.file);
        let stable_id = taint_stable_id(finding);
        let item = serde_json::json!({
            "stable_id": stable_id, "kind": "taint", "rule_id": finding.rule_id,
            "file": file, "source_line": finding.source_line,
            "sink_line": finding.sink_line, "severity": finding.severity.to_string(),
            "exploitability_score": finding.exploitability_score,
        });
        with_cell(
            with_cell(item, "cell_number", finding.cell_number),
            "source_cell_number",
            finding.source_cell_number,
        )
    }));
    other.par_extend(result.parse_errors.par_iter().map(|error| {
        let file = crate::utils::normalize_display_path(&error.file);
        let stable_id = parse_error_stable_id(error);
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
    let stable_id = dead_code_stable_id(kind, def);
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
    let stable_id = secret_stable_id(finding);
    with_cell(
        serde_json::json!({
            "stable_id": stable_id, "kind": "secret", "rule_id": finding.rule_id,
            "file": file, "line": finding.line, "severity": finding.severity,
            "confidence": finding.confidence,
        }),
        "cell_number",
        finding.cell_number,
    )
}

/// Stable ID of a dead-code finding; `kind` is e.g. `unused_function`.
pub(crate) fn dead_code_stable_id(kind: &str, def: &crate::visitor::Definition) -> String {
    let file = crate::utils::normalize_display_path(&def.file);
    format!("{kind}:{}:{}:{}", def.name, file, def.start_byte)
}

/// Stable ID of a rule finding; `kind` is `danger` or `quality`.
pub(crate) fn rule_finding_stable_id(kind: &str, finding: &crate::rules::Finding) -> String {
    let file = crate::utils::normalize_display_path(&finding.file);
    format!(
        "{kind}:{}:{}:{}:{}",
        finding.rule_id,
        in_cell(&file, finding.cell_number),
        finding.line,
        finding.col
    )
}

/// Stable ID of a taint finding.
pub(crate) fn taint_stable_id(finding: &crate::taint::types::TaintFinding) -> String {
    let file = crate::utils::normalize_display_path(&finding.file);
    format!(
        "taint:{}:{}:{}:{}",
        finding.rule_id,
        in_cell(&file, finding.cell_number),
        in_cell(&finding.source_line.to_string(), finding.source_cell_number),
        finding.sink_line
    )
}

/// Stable ID of a parse error.
pub(crate) fn parse_error_stable_id(error: &crate::analyzer::types::ParseError) -> String {
    let file = crate::utils::normalize_display_path(&error.file);
    format!("parse_error:{file}:{}", error.error)
}

/// Stable ID of a secret finding. The matched value is already redacted.
pub(crate) fn secret_stable_id(finding: &crate::rules::secrets::SecretFinding) -> String {
    let file = crate::utils::normalize_display_path(&finding.file);
    let matched = finding.matched_value.as_deref().unwrap_or("-");
    let entropy = finding
        .entropy
        .map(|value| format!("{value:.4}"))
        .unwrap_or_else(|| "-".to_owned());
    format!(
        "secret:{}:{}:{}:{}:{}:{}:{}",
        finding.rule_id,
        in_cell(&file, finding.cell_number),
        finding.line,
        finding.confidence,
        finding.message,
        matched,
        entropy
    )
}

/// `location` qualified by its notebook cell. Cell lines restart in every
/// cell, so IDs built from them need the cell to stay unique; without a cell
/// the ID is unchanged, keeping existing baselines valid.
fn in_cell(location: &str, cell: Option<usize>) -> String {
    match cell {
        Some(cell) => format!("{location}[cell {cell}]"),
        None => location.to_owned(),
    }
}

/// Adds `key: cell` to `item` when the finding has a cell.
fn with_cell(mut item: Value, key: &str, cell: Option<usize>) -> Value {
    if let (Some(cell), Some(object)) = (cell, item.as_object_mut()) {
        object.insert(key.to_owned(), cell.into());
    }
    item
}

#[cfg(test)]
#[path = "stable_findings_tests.rs"]
mod tests;
