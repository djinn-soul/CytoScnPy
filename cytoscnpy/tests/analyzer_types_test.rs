//! Tests for analyzer types serialization and constructors.

use cytoscnpy::analyzer::types::{FixSuggestion, ParseError};
use std::path::PathBuf;

#[test]
fn test_fix_suggestion_constructor_deletion() {
    let fix = FixSuggestion::deletion(10, 20);
    assert_eq!(fix.start_byte, 10);
    assert_eq!(fix.end_byte, 20);
    assert!(fix.replacement.is_empty());
}

#[test]
fn test_fix_suggestion_constructor_replacement() {
    let fix = FixSuggestion::replacement(10, 20, "foo".to_owned());
    assert_eq!(fix.start_byte, 10);
    assert_eq!(fix.end_byte, 20);
    assert_eq!(fix.replacement, "foo");
}

#[test]
fn test_fix_suggestion_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let fix = FixSuggestion::replacement(0, 5, "bar".to_owned());
    let json = serde_json::to_string(&fix)?;

    assert!(json.contains("\"start_byte\":0"));
    assert!(json.contains("\"end_byte\":5"));
    assert!(json.contains("\"replacement\":\"bar\""));
    Ok(())
}

#[test]
fn test_parse_error_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let error = ParseError {
        file: PathBuf::from("foo.py"),
        error: "syntax error".to_owned(),
    };
    let json = serde_json::to_string(&error)?;
    assert!(json.contains("\"file\":"));
    assert!(json.contains("\"error\":\"syntax error\""));
    Ok(())
}
