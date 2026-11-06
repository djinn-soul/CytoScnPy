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

#[test]
fn notebook_cell_findings_keep_distinct_stable_ids() {
    // Cell lines restart in every cell: line 1 of cells 1 and 2 differ.
    let mut result = crate::analyzer::AnalysisResult::default();
    for cell in [Some(1), Some(2), None] {
        result.danger.push(crate::rules::Finding {
            rule_id: "CSP-D001".to_owned(),
            category: "Security".to_owned(),
            severity: "CRITICAL".to_owned(),
            message: "Avoid using eval".to_owned(),
            file: "nb.ipynb".into(),
            line: 1,
            col: 0,
            cell_number: cell,
        });
    }

    let items: Vec<Value> = stable_findings(&result)
        .into_iter()
        .map(|item| serde_json::to_value(item).unwrap())
        .collect();
    let ids: Vec<&str> = items
        .iter()
        .filter_map(|item| item["stable_id"].as_str())
        .collect();
    assert_eq!(
        ids,
        [
            "danger:CSP-D001:nb.ipynb:1:0",
            "danger:CSP-D001:nb.ipynb[cell 1]:1:0",
            "danger:CSP-D001:nb.ipynb[cell 2]:1:0",
        ]
    );
    assert_eq!(items[1]["cell_number"], 1);
    assert!(items[0].get("cell_number").is_none());
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
        cell_number: None,
    }
}
