//! Tests for `FixSuggestion` struct and related functionality.
//! TDD: These tests are written BEFORE the implementation.

// Test-specific lint suppressions
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#[test]
fn test_fix_suggestion_serialization() {
    // This test will fail until we implement FixSuggestion
    use cytoscnpy::analyzer::types::FixSuggestion;

    let fix = FixSuggestion {
        start_byte: 100,
        end_byte: 150,
        replacement: String::new(), // Empty string means "delete"
    };

    let json = serde_json::to_string(&fix).expect("should serialize");
    assert!(json.contains("\"start_byte\":100"));
    assert!(json.contains("\"end_byte\":150"));
    assert!(json.contains("\"replacement\":\"\""));
}

/// Test that `FixSuggestion` for deletion has empty replacement.
#[test]
fn test_fix_suggestion_deletion() {
    use cytoscnpy::analyzer::types::FixSuggestion;

    let fix = FixSuggestion::deletion(100, 200);

    assert_eq!(fix.start_byte, 100);
    assert_eq!(fix.end_byte, 200);
    assert!(fix.replacement.is_empty());
}

/// Test that `Definition` can optionally carry a `FixSuggestion`.
#[test]
fn test_definition_with_fix() {
    use cytoscnpy::analyzer::types::FixSuggestion;
    use cytoscnpy::visitor::Definition;
    use std::path::PathBuf;
    use std::sync::Arc;

    // Definitions should be able to carry fix suggestions
    let def = Definition {
        name: "unused_func".to_owned(),
        full_name: "module.unused_func".to_owned(),
        simple_name: "unused_func".to_owned(),
        def_type: "function".to_owned(),
        file: Arc::new(PathBuf::from("test.py")),
        line: 10,
        end_line: 10,
        col: 0,
        start_byte: 0,
        end_byte: 0,
        confidence: 100,
        references: 0,
        is_exported: false,
        in_init: false,
        is_framework_managed: false,
        base_classes: smallvec::smallvec![],
        is_type_checking: false,
        is_captured: false,
        cell_number: None,
        is_self_referential: false,
        message: Some("unused function".to_owned()),
        fix: Some(Box::new(FixSuggestion::deletion(50, 100))),
        is_enum_member: false,
        is_constant: false,
        is_potential_secret: false,
    };

    assert!(def.fix.is_some());
    let fix = def.fix.unwrap();
    assert_eq!(fix.start_byte, 50);
    assert_eq!(fix.end_byte, 100);
}

/// Test that Definition without fix serializes correctly (no fix field in JSON).
#[test]
fn test_definition_without_fix_serializes() {
    use cytoscnpy::visitor::Definition;
    use std::path::PathBuf;
    use std::sync::Arc;

    let def = Definition {
        name: "used_func".to_owned(),
        full_name: "module.used_func".to_owned(),
        simple_name: "used_func".to_owned(),
        def_type: "function".to_owned(),
        file: Arc::new(PathBuf::from("test.py")),
        line: 5,
        end_line: 5,
        col: 0,
        start_byte: 0,
        end_byte: 0,
        confidence: 100,
        references: 3,
        is_exported: true,
        in_init: false,
        is_framework_managed: false,
        base_classes: smallvec::smallvec![],
        is_type_checking: false,
        is_captured: false,
        cell_number: None,
        is_self_referential: false,
        message: None,
        fix: None,
        is_enum_member: false,
        is_constant: false,
        is_potential_secret: false,
    };

    let json = serde_json::to_string(&def).expect("should serialize");
    // fix should be skipped if None
    assert!(!json.contains("\"fix\""));
}

/// Test that Definition with fix serializes correctly (fix field present in JSON).
#[test]
fn test_definition_with_fix_serializes() {
    use cytoscnpy::analyzer::types::FixSuggestion;
    use cytoscnpy::visitor::Definition;
    use std::path::PathBuf;
    use std::sync::Arc;

    let def = Definition {
        name: "dead_code".to_owned(),
        full_name: "module.dead_code".to_owned(),
        simple_name: "dead_code".to_owned(),
        def_type: "function".to_owned(),
        file: Arc::new(PathBuf::from("test.py")),
        line: 20,
        end_line: 20,
        col: 0,
        start_byte: 0,
        end_byte: 0,
        confidence: 100,
        references: 0,
        is_exported: false,
        in_init: false,
        is_framework_managed: false,
        base_classes: smallvec::smallvec![],
        is_type_checking: false,
        is_captured: false,
        cell_number: None,
        is_self_referential: false,
        message: Some("unused function".to_owned()),
        fix: Some(Box::new(FixSuggestion {
            start_byte: 200,
            end_byte: 350,
            replacement: String::new(),
        })),
        is_enum_member: false,
        is_constant: false,
        is_potential_secret: false,
    };

    let json = serde_json::to_string(&def).expect("should serialize");
    assert!(json.contains("\"fix\""));
    assert!(json.contains("\"start_byte\":200"));
    assert!(json.contains("\"end_byte\":350"));
}
